// vim:fileencoding=utf-8:noet
//! Diagnostic log writer. Appends timestamped lines to
//! `$HOME/.powerliners/powerliners.log` for post-mortem analysis when a
//! segment renders unexpectedly (silently dropped, slow, returning
//! `None`, etc.). Lives in `extensions` because it's not part of the
//! upstream powerline contract.
//!
//! Rotation: when the active file exceeds [`MAX_BYTES`], it is renamed
//! to `powerliners.log.1` and the existing `.1` → `.2`, … up to
//! [`KEEP_ROTATIONS`]. The oldest file is dropped.

use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MAX_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB
const KEEP_ROTATIONS: usize = 3;

static LOG_STATE: OnceLock<Mutex<Option<LogState>>> = OnceLock::new();

struct LogState {
    path: PathBuf,
    file: std::fs::File,
}

fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut dir = PathBuf::from(home);
    dir.push(".powerliners");
    create_dir_all(&dir).ok()?;
    dir.push("powerliners.log");
    Some(dir)
}

fn open(path: &Path) -> Option<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

fn rotate(path: &Path) {
    // Shift .1 → .2, .2 → .3, …, .N drops off the end.
    for i in (1..KEEP_ROTATIONS).rev() {
        let from = path.with_extension(format!("log.{}", i));
        let to = path.with_extension(format!("log.{}", i + 1));
        let _ = rename(&from, &to);
    }
    let dot1 = path.with_extension("log.1");
    let _ = rename(path, &dot1);
}

fn init_state() -> Option<LogState> {
    let path = log_path()?;
    let file = open(&path)?;
    Some(LogState { path, file })
}

/// Append `msg` to the diagnostic log with a microsecond-precision
/// epoch timestamp + the calling process id. No-op when the file
/// can't be opened (permissions, missing $HOME, etc.) so the renderer
/// never fails on logging issues.
pub fn log(msg: &str) {
    let cell = LOG_STATE.get_or_init(|| Mutex::new(init_state()));
    if let Ok(mut guard) = cell.lock() {
        if let Some(state) = guard.as_mut() {
            // Rotate if the active file has grown past the cap. We
            // check via metadata rather than tracking writes so that
            // log files left behind by a previous run still rotate
            // correctly on the next append.
            if let Ok(meta) = state.file.metadata() {
                if meta.len() >= MAX_BYTES {
                    rotate(&state.path);
                    if let Some(f) = open(&state.path) {
                        state.file = f;
                    }
                }
            }
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            let _ = writeln!(
                state.file,
                "[{:.3} pid={}] {}",
                ts,
                std::process::id(),
                msg
            );
            let _ = state.file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build N files at `<base>` and `<base>.log.1..N` so we can
    /// watch the rotation cascade move each one along.
    fn seed_log_chain(base: &Path, n: usize) {
        fs::write(base, b"head\n").unwrap();
        for i in 1..=n {
            let p = base.with_extension(format!("log.{}", i));
            fs::write(&p, format!("body-{}\n", i).as_bytes()).unwrap();
        }
    }

    fn tmp_log_base(tag: &str) -> PathBuf {
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("powerliners-diag-test-{tag}-{pid}-{nanos}"));
        p.push("powerliners.log");
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        p
    }

    #[test]
    fn rotate_moves_active_log_to_dot1() {
        // Empty board: no rotations yet — rotate must rename
        // active → .log.1 with no other side effects.
        let base = tmp_log_base("dot1");
        fs::write(&base, b"active\n").unwrap();
        rotate(&base);
        assert!(!base.exists(), "active log should be renamed away");
        let dot1 = base.with_extension("log.1");
        assert!(dot1.exists(), ".log.1 should now exist");
        assert_eq!(fs::read(&dot1).unwrap(), b"active\n");
        fs::remove_dir_all(base.parent().unwrap()).ok();
    }

    #[test]
    fn rotate_cascades_through_keep_rotations() {
        // Seed .log + .log.1 + .log.2; rotate must produce
        // .log.1 (was active), .log.2 (was .1), .log.3 (was .2).
        // KEEP_ROTATIONS=3, so .log.3 is the eldest survivor.
        let base = tmp_log_base("cascade");
        seed_log_chain(&base, KEEP_ROTATIONS - 1);
        rotate(&base);
        let read = |i: usize| -> Vec<u8> {
            fs::read(base.with_extension(format!("log.{}", i))).unwrap_or_default()
        };
        // After rotation:
        //   .log.1 holds what was the live "head\n"
        //   .log.2 holds what was .log.1's "body-1\n"
        //   .log.3 holds what was .log.2's "body-2\n"
        assert_eq!(read(1), b"head\n", ".log.1 must hold ex-active content");
        assert_eq!(read(2), b"body-1\n", ".log.2 must hold ex-.log.1 content");
        assert_eq!(read(3), b"body-2\n", ".log.3 must hold ex-.log.2 content");
        fs::remove_dir_all(base.parent().unwrap()).ok();
    }

    #[test]
    fn rotate_drops_oldest_when_chain_is_full() {
        // Pre-populate up to .log.KEEP_ROTATIONS. Rotate. The
        // KEEP_ROTATIONS+1 slot is never created (the loop runs
        // `(1..KEEP_ROTATIONS).rev()` — top index is KEEP_ROTATIONS-1
        // → renamed to KEEP_ROTATIONS, then base → .log.1). So the
        // pre-existing .log.KEEP_ROTATIONS is silently overwritten,
        // and the original .log.KEEP_ROTATIONS content is gone.
        let base = tmp_log_base("drop_oldest");
        seed_log_chain(&base, KEEP_ROTATIONS);
        let eldest_path = base.with_extension(format!("log.{}", KEEP_ROTATIONS));
        let pre = fs::read(&eldest_path).unwrap();
        rotate(&base);
        let post = fs::read(&eldest_path).unwrap();
        assert_ne!(
            pre, post,
            "eldest .log.{} should have been overwritten by .log.{}",
            KEEP_ROTATIONS,
            KEEP_ROTATIONS - 1
        );
        // The KEEP_ROTATIONS+1 slot must NOT exist — we cap chain length.
        let beyond = base.with_extension(format!("log.{}", KEEP_ROTATIONS + 1));
        assert!(
            !beyond.exists(),
            "rotation must not create slot beyond KEEP_ROTATIONS"
        );
        fs::remove_dir_all(base.parent().unwrap()).ok();
    }

    #[test]
    fn rotate_on_missing_active_is_a_noop() {
        // No file at the base path — rotate must not panic and must
        // not synthesize ghost files.
        let base = tmp_log_base("missing");
        // Don't create base.
        rotate(&base);
        assert!(!base.exists());
        assert!(!base.with_extension("log.1").exists());
        fs::remove_dir_all(base.parent().unwrap()).ok();
    }

    #[test]
    fn open_creates_file_and_appends_on_reopen() {
        // The OpenOptions config at line 36-42 uses create+append; two
        // successive opens of the same path should accumulate writes
        // rather than truncate.
        let base = tmp_log_base("append");
        {
            let mut f = open(&base).expect("first open");
            writeln!(f, "first").unwrap();
        }
        {
            let mut f = open(&base).expect("second open");
            writeln!(f, "second").unwrap();
        }
        let text = fs::read_to_string(&base).unwrap();
        assert!(text.contains("first"), "append lost first write: {text:?}");
        assert!(text.contains("second"), "append lost second write: {text:?}");
        fs::remove_dir_all(base.parent().unwrap()).ok();
    }
}
