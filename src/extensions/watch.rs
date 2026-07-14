// vim:fileencoding=utf-8:noet
//! Reactive Prompt Push — real filesystem-event watching for the warm
//! daemon. Sanctioned non-port location per `docs/PORT.md` (upstream
//! powerline has no equivalent: its VCS `tree` watcher only feeds the
//! *pull* render, it never pushes a redraw back to the shell).
//!
//! ## What this does
//!
//! The warm `powerline-daemon` renders a client's prompt on demand
//! (pull). This extension adds an orthogonal *push* path: it watches the
//! filesystem inputs backing each client's currently-displayed prompt
//! (its `cwd`, `.git/HEAD`, `.git/index`, and the resolved branch ref)
//! and, when one of those changes for real, writes a single wake byte to
//! a per-client FIFO the shell is watching (`zle -F` in zsh). The shell
//! reacts by re-rendering the prompt in place — so the branch name flips
//! the instant you `git checkout` in another pane, between keystrokes,
//! with no `Enter` and no interval timer.
//!
//! It is **edge-triggered**: the OS backend (kqueue/FSEvents on macOS,
//! inotify on Linux) delivers an event only on a real change, and the
//! wake byte is written only when the recomputed [`Fingerprint`] differs
//! from the one captured at the last render — zero wasted redraws.
//!
//! ## Layering
//!
//! Nothing here touches the 1:1-ported daemon (`src/ported/`). The
//! daemon binary calls [`register`] from inside its injected `render_fn`
//! (a `src/bin/` seam), passing the client's `cwd` and the FIFO path the
//! client advertised via the `POWERLINE_RESET_FIFO` environment variable
//! (exported by the zsh binding in `shell_hooks/reactive.zsh`). The
//! pull-only ported code path is unchanged and unaware of any of this.
//!
//! ## Relationship to the ported `stat`-fallback watcher
//!
//! `src/ported/lib/watcher/` is a faithful port of powerline's own
//! tree-watcher (`inotify` / `uv` / `dummy` / `stat`). That code answers
//! "did this tree change since I last polled it" for VCS segments. This
//! module solves a different problem — pushing a wake across processes —
//! and uses the `notify` crate's real cross-platform backends. When no
//! backend can be created (or `notify` errors), [`register`] logs and
//! returns without watching; the prompt then simply behaves as it always
//! has (pull-only on each precmd). That is the graceful fallback seam.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::UNIX_EPOCH;

use notify::{recommended_watcher, RecommendedWatcher, RecursiveMode, Watcher};

/// A snapshot of the inputs that determine what a client's prompt shows.
///
/// Two fingerprints comparing equal means the prompt would render
/// identically with respect to the watched inputs, so no wake is needed.
/// Fields are intentionally cheap to read (a couple of small file reads
/// and one `stat`), because [`compute`](Fingerprint::compute) runs on
/// every filesystem event delivered for a watched client.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    /// The client's working directory.
    pub cwd: PathBuf,
    /// Raw contents of `.git/HEAD` (e.g. `ref: refs/heads/main` or a
    /// detached-HEAD object id), trimmed. `None` when not in a repo.
    pub head: Option<String>,
    /// The commit object id `HEAD` currently resolves to, so a commit on
    /// the same branch (which does not touch `.git/HEAD`) still flips the
    /// fingerprint. `None` when not in a repo or on an unborn branch.
    pub head_sha: Option<String>,
    /// `.git/index` modification time in nanoseconds since the UNIX
    /// epoch — staging/unstaging changes this. `None` when absent.
    pub index_mtime: Option<u128>,
}

impl Fingerprint {
    /// Compute the fingerprint for `cwd` right now.
    pub fn compute(cwd: &Path) -> Fingerprint {
        let git = discover_git_dir(cwd);
        let head = git
            .as_ref()
            .and_then(|g| fs::read_to_string(g.join("HEAD")).ok())
            .map(|s| s.trim().to_string());
        let head_sha = git
            .as_ref()
            .zip(head.as_ref())
            .and_then(|(g, h)| resolve_head_sha(g, h));
        let index_mtime = git.as_ref().and_then(|g| mtime_ns(&g.join("index")));
        Fingerprint {
            cwd: cwd.to_path_buf(),
            head,
            head_sha,
            index_mtime,
        }
    }

    /// The concrete filesystem paths whose changes could flip this
    /// fingerprint. The daemon hands these to the OS watcher.
    ///
    /// We watch directories (not the individual files) because git
    /// rewrites `HEAD`/`index`/refs via a temp-file-plus-rename dance;
    /// a bare per-file watch would miss the replacement on some
    /// backends, whereas the containing directory reliably reports the
    /// rename. `cwd` is watched too so a plain `cd`-target reorganization
    /// (or a repo appearing/disappearing under it) is seen.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        let mut out = vec![self.cwd.clone()];
        if let Some(git) = discover_git_dir(&self.cwd) {
            // Top level of the git dir: catches HEAD, index, ORIG_HEAD.
            out.push(git.clone());
            // Branch pointers: catches a commit on the current branch.
            let heads = git.join("refs").join("heads");
            if heads.is_dir() {
                out.push(heads);
            }
        }
        out.retain(|p| p.exists());
        out.dedup();
        out
    }
}

/// Read `.git/index`'s mtime as nanoseconds since the UNIX epoch.
fn mtime_ns(path: &Path) -> Option<u128> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    Some(mtime.duration_since(UNIX_EPOCH).ok()?.as_nanos())
}

/// Resolve the raw `.git/HEAD` contents to a commit object id.
///
/// - `ref: refs/heads/foo` → read `<git>/refs/heads/foo`, else scan
///   `<git>/packed-refs` for the same ref.
/// - a bare 40/64-hex id (detached HEAD) → itself.
fn resolve_head_sha(git: &Path, head: &str) -> Option<String> {
    if let Some(refname) = head.strip_prefix("ref:") {
        let refname = refname.trim();
        let loose = git.join(refname);
        if let Ok(sha) = fs::read_to_string(&loose) {
            let sha = sha.trim().to_string();
            if !sha.is_empty() {
                return Some(sha);
            }
        }
        // packed-refs fallback: lines are "<sha> <refname>".
        if let Ok(packed) = fs::read_to_string(git.join("packed-refs")) {
            for line in packed.lines() {
                let line = line.trim();
                if line.starts_with('#') || line.starts_with('^') {
                    continue;
                }
                if let Some((sha, name)) = line.split_once(' ') {
                    if name.trim() == refname {
                        return Some(sha.trim().to_string());
                    }
                }
            }
        }
        None
    } else if !head.is_empty() {
        // Detached HEAD: the id is the HEAD contents itself.
        Some(head.to_string())
    } else {
        None
    }
}

/// Walk up from `start` to find the effective git directory.
///
/// Handles the common `.git` directory, and the `.git` *file*
/// (`gitdir: <path>`) form used by worktrees and submodules.
fn discover_git_dir(start: &Path) -> Option<PathBuf> {
    let mut dir = start;
    loop {
        let dot = dir.join(".git");
        if dot.is_dir() {
            return Some(dot);
        }
        if dot.is_file() {
            if let Ok(contents) = fs::read_to_string(&dot) {
                if let Some(rest) = contents.trim().strip_prefix("gitdir:") {
                    let target = PathBuf::from(rest.trim());
                    let resolved = if target.is_absolute() {
                        target
                    } else {
                        dir.join(target)
                    };
                    if resolved.exists() {
                        return Some(resolved);
                    }
                }
            }
        }
        dir = dir.parent()?;
    }
}

/// Start a real filesystem watch over `paths`, invoking `on_change` on
/// every delivered event. This is the testable core of the extension:
/// it has no FIFO / fingerprint policy, it just proves the OS backend
/// wiring fires a Rust callback on a real file change.
///
/// The returned watcher must be kept alive; dropping it stops watching.
pub fn watch_paths<F>(paths: &[PathBuf], on_change: F) -> notify::Result<RecommendedWatcher>
where
    F: Fn() + Send + 'static,
{
    let mut watcher = recommended_watcher(move |res: notify::Result<notify::Event>| {
        // We do not care *what* changed, only that something did — the
        // caller re-derives truth from the filesystem. Errors from the
        // backend are non-fatal; a missed event just means the next
        // render falls back to pull, which is correct behavior.
        if res.is_ok() {
            on_change();
        }
    })?;
    for p in paths {
        // NonRecursive: we watch specific dirs (cwd, git dir, refs/heads)
        // deliberately, never the object store, so a busy repo does not
        // drown us in irrelevant events.
        watcher.watch(p, RecursiveMode::NonRecursive)?;
    }
    Ok(watcher)
}

/// Write a single wake byte to a FIFO the shell is watching.
///
/// Opens the FIFO write-only and non-blocking so a client that has gone
/// away (no reader) yields `ENXIO`/`EWOULDBLOCK` instead of hanging the
/// daemon. Any error is swallowed — the client simply will not redraw
/// this time.
pub fn wake(fifo: &str) {
    let opened = fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(fifo);
    if let Ok(mut f) = opened {
        let _ = f.write_all(b"\x01");
    }
}

/// Per-client watch state held for the daemon's lifetime.
struct ClientState {
    /// Kept alive to keep the OS watch active.
    _watcher: RecommendedWatcher,
    /// The fingerprint of the prompt the client is currently displaying.
    /// Shared with the event-handler closure so a change can be detected
    /// and the baseline advanced atomically.
    baseline: Arc<Mutex<Fingerprint>>,
}

/// The daemon-wide registry, keyed by the client's FIFO path (unique per
/// shell). `OnceLock` keeps it lazily initialized and lock-free to read.
fn registry() -> &'static Mutex<HashMap<String, ClientState>> {
    static REG: OnceLock<Mutex<HashMap<String, ClientState>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register (or refresh) reactive watching for the client identified by
/// `wake_fifo`, whose prompt currently reflects `cwd`.
///
/// Called from the daemon's `render_fn` on every render. Behavior:
///
/// - **First render / new cwd:** (re)build an OS watch over the paths
///   backing `cwd` and record the current fingerprint as the baseline.
/// - **Same cwd as last render:** advance the baseline to *now* (the
///   just-rendered prompt is the new source of truth) without rebuilding
///   the watch.
///
/// When the watch later fires and the recomputed fingerprint differs
/// from the baseline, a wake byte is written to `wake_fifo` and the
/// baseline advances so the same change is not signalled twice.
///
/// All failures are logged via `diag_log` and swallowed; a client that
/// cannot be watched degrades to the pre-existing pull-only behavior.
pub fn register(wake_fifo: &str, cwd: &Path) {
    let now = Fingerprint::compute(cwd);

    let mut reg = match registry().lock() {
        Ok(g) => g,
        Err(_) => return,
    };

    if let Some(state) = reg.get(&wake_fifo.to_string()) {
        // Same client already watched. If it is still in the same cwd,
        // just advance the baseline to the freshly-rendered state.
        if let Ok(mut base) = state.baseline.lock() {
            if base.cwd == now.cwd {
                *base = now;
                return;
            }
        }
        // cwd changed — fall through to rebuild the watch below.
    }

    let paths = now.watched_paths();
    let baseline = Arc::new(Mutex::new(now));

    let cb_cwd = cwd.to_path_buf();
    let cb_fifo = wake_fifo.to_string();
    let cb_baseline = baseline.clone();
    let on_change = move || {
        let current = Fingerprint::compute(&cb_cwd);
        let changed = match cb_baseline.lock() {
            Ok(mut base) => {
                if *base != current {
                    *base = current;
                    true
                } else {
                    false
                }
            }
            Err(_) => false,
        };
        if changed {
            wake(&cb_fifo);
        }
    };

    match watch_paths(&paths, on_change) {
        Ok(watcher) => {
            reg.insert(
                wake_fifo.to_string(),
                ClientState {
                    _watcher: watcher,
                    baseline,
                },
            );
            crate::extensions::diag_log::log(&format!(
                "reactive WATCH fifo={} cwd={} paths={}",
                wake_fifo,
                cwd.display(),
                paths.len()
            ));
        }
        Err(e) => {
            crate::extensions::diag_log::log(&format!(
                "reactive WATCH-FAIL fifo={} cwd={} err={} (falling back to pull-only)",
                wake_fifo,
                cwd.display(),
                e
            ));
        }
    }
}

/// Drop a client's watch (e.g. its shell exited). Idempotent.
pub fn unregister(wake_fifo: &str) {
    if let Ok(mut reg) = registry().lock() {
        reg.remove(&wake_fifo.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    /// The OS watch actually fires a Rust callback on a real file change.
    #[test]
    fn watch_paths_fires_callback_on_real_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fired = Arc::new(AtomicBool::new(false));
        let fired_cb = fired.clone();

        let _watcher = watch_paths(&[dir.path().to_path_buf()], move || {
            fired_cb.store(true, Ordering::SeqCst);
        })
        .expect("watch_paths");

        // Give the backend a moment to arm before mutating.
        std::thread::sleep(Duration::from_millis(200));
        fs::write(dir.path().join("touched"), b"x").expect("write");

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if fired.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(
            fired.load(Ordering::SeqCst),
            "callback did not fire on a real filesystem change"
        );
    }

    /// The fingerprint flips when `.git/HEAD` (a checkout) changes.
    #[test]
    fn fingerprint_changes_on_head_checkout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        fs::create_dir_all(git.join("refs").join("heads")).expect("mkdir .git");
        fs::write(
            git.join("refs/heads/main"),
            "1111111111111111111111111111111111111111\n",
        )
        .expect("write main ref");
        fs::write(
            git.join("refs/heads/feature"),
            "2222222222222222222222222222222222222222\n",
        )
        .expect("write feature ref");
        fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

        let before = Fingerprint::compute(dir.path());
        assert_eq!(before.head.as_deref(), Some("ref: refs/heads/main"));
        assert_eq!(
            before.head_sha.as_deref(),
            Some("1111111111111111111111111111111111111111")
        );

        // Simulate `git checkout feature` in another pane.
        fs::write(git.join("HEAD"), "ref: refs/heads/feature\n").expect("rewrite HEAD");
        let after = Fingerprint::compute(dir.path());

        assert_ne!(before, after, "fingerprint must change after a checkout");
        assert_eq!(after.head.as_deref(), Some("ref: refs/heads/feature"));
        assert_eq!(
            after.head_sha.as_deref(),
            Some("2222222222222222222222222222222222222222")
        );
    }

    /// The fingerprint flips when the working directory changes.
    #[test]
    fn fingerprint_changes_on_cwd_change() {
        let a = tempfile::tempdir().expect("tempdir a");
        let b = tempfile::tempdir().expect("tempdir b");
        let fa = Fingerprint::compute(a.path());
        let fb = Fingerprint::compute(b.path());
        assert_ne!(fa, fb, "different cwds must fingerprint differently");
    }
}
