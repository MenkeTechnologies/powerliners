// vim:fileencoding=utf-8:noet
//! End-to-end tests: spawn the `powerline-daemon` binary on a temp
//! UNIX socket, send a real tmux render request via the wire protocol,
//! parse the rendered tmux markup, assert on the segments produced.
//!
//! Each scenario uses a fixture config root under `tests/data/e2e/`
//! containing a complete `config.json` + `colors.json` +
//! `colorschemes/tmux/default.json` + `themes/tmux/default.json` tree.
//! The daemon resolves them via `POWERLINE_CONFIG_PATHS`, exactly the
//! same path the production binary follows.
//!
//! The harness is process-level: a real fork+exec of
//! `target/debug/powerline-daemon`, no in-process shortcuts. This
//! catches integration regressions that lib-tests don't (wire format
//! decode, socket binding, config cascade, render dispatch, response
//! framing, EOF shutdown).

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SHUTDOWN_SENTINEL: &[u8] = b"EOF\0\0";
const READ_TIMEOUT: Duration = Duration::from_secs(5);

/// Resolve the path to the daemon binary built by cargo. Walks the
/// test executable's grandparent (which is always `target/debug/` for
/// integration tests) to find the sibling `powerline-daemon` binary.
fn daemon_binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    // current_exe is `.../target/debug/deps/daemon_e2e-<hash>`. Pop
    // the deps dir to reach `.../target/debug/`.
    p.pop();
    p.pop();
    p.push("powerline-daemon");
    assert!(
        p.exists(),
        "powerline-daemon binary missing at {} — run `cargo build --bin powerline-daemon` first",
        p.display()
    );
    p
}

/// Resolve the absolute path of a fixture config root.
fn fixture_root(scenario: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/data/e2e");
    p.push(scenario);
    assert!(p.is_dir(), "fixture root missing: {}", p.display());
    p
}

/// Picks a unique-per-process socket path under `$TMPDIR`.
fn unique_socket() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "powerliners-e2e-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    // Defensive: clear any prior leftover.
    let _ = std::fs::remove_file(&p);
    p
}

/// Spawned daemon plus cleanup metadata. `Drop` kills the process and
/// scrubs the socket+pidfile so a failing assertion doesn't leak state
/// across tests.
struct DaemonHandle {
    child: Child,
    socket: PathBuf,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        // Try a clean EOF first so the daemon path runs its
        // shutdown sequence; fall back to SIGKILL if it ignores us.
        if let Ok(mut s) = UnixStream::connect(&self.socket) {
            let _ = s.write_all(SHUTDOWN_SENTINEL);
        }
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
        let mut pid = self.socket.clone();
        pid.set_extension("pid");
        let _ = std::fs::remove_file(&pid);
    }
}

/// Spawn the daemon in foreground mode pointed at `fixture` for its
/// config root. Polls the socket until it's bind-able (up to 3 s) so
/// callers can connect immediately after `start_daemon` returns.
fn start_daemon(scenario: &str) -> DaemonHandle {
    let socket = unique_socket();
    let bin = daemon_binary();
    let fixture = fixture_root(scenario);
    let mut child = Command::new(&bin)
        .arg("--foreground")
        .arg("--socket")
        .arg(&socket)
        .env("POWERLINE_CONFIG_PATHS", &fixture)
        // Prevent the daemon's status output from leaking into the
        // test's captured output.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn powerline-daemon");

    // Wait for the daemon to listen. CI runners (Ubuntu cold-start, no
    // page cache for the freshly-linked binary + first-run config-cascade
    // JSON parse) routinely take 5–8 s before the socket binds; macOS dev
    // hits ~200 ms. Budget generously so CI is deterministic.
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(probe) = UnixStream::connect(&socket) {
            // Connect-only probe; close without sending anything so we
            // don't trigger a render. The daemon's `do_read` will time
            // out and close the conn.
            let _ = probe.shutdown(std::net::Shutdown::Both);
            return DaemonHandle { child, socket };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    // Reap the spawned daemon before panicking so the test process doesn't
    // leak a zombie when the bind fails.
    let _ = child.kill();
    let _ = child.wait();
    panic!("daemon never became ready on {}", socket.display());
}

/// Build the daemon's wire-format payload for a `powerline <ext> <side>`
/// invocation. Mirrors `client/powerline.c:126-148` byte-for-byte:
/// `<argc_hex>\0<arg>\0...<cwd>\0<KEY=VAL>\0...\0\0`.
fn build_request(args: &[&str], cwd: &str, env: &[(&str, &str)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend(format!("{:x}", args.len()).as_bytes());
    out.push(0);
    for arg in args {
        out.extend(arg.as_bytes());
        out.push(0);
    }
    out.extend(cwd.as_bytes());
    out.push(0);
    for (k, v) in env {
        out.extend(format!("{}={}", k, v).as_bytes());
        out.push(0);
    }
    // EOF terminator: 2 more null bytes per `do_write(sd, eof, 2)` at
    // client/powerline.c:148.
    out.push(0);
    out.push(0);
    out
}

/// One render round-trip. Send the request, read until peer-close (or
/// the read timeout fires), return the bytes.
fn render_once(socket: &PathBuf, args: &[&str], cwd: &str, env: &[(&str, &str)]) -> Vec<u8> {
    let mut conn = UnixStream::connect(socket).expect("connect");
    conn.set_read_timeout(Some(READ_TIMEOUT)).ok();
    conn.write_all(&build_request(args, cwd, env))
        .expect("send request");
    let mut buf = Vec::new();
    let _ = conn.read_to_end(&mut buf);
    buf
}

/// Strip every `#[...]` tmux style marker so assertions match the
/// visible characters only.
fn strip_tmux_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' && chars.peek() == Some(&'[') {
            chars.next();
            for d in chars.by_ref() {
                if d == ']' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn scenario_hostname_renders_local_hostname() {
    let daemon = start_daemon("scenario_hostname");
    let raw = render_once(
        &daemon.socket,
        &["tmux", "right"],
        "/tmp",
        &[("HOME", "/tmp"), ("PWD", "/tmp")],
    );
    let body = String::from_utf8_lossy(&raw).to_string();
    let visible = strip_tmux_markup(&body);
    assert!(
        !body.is_empty(),
        "daemon returned empty body: raw={:?}",
        raw
    );
    // Hostname segment must produce SOMETHING — at minimum the local
    // hostname or its first label. Match a leading char run that
    // looks like a hostname character (alphanumeric or dash).
    // Skip leading divider glyph(s) + non-breaking spaces — the
    // right-side render starts with a `` hard divider plus a NBSP.
    let trimmed = visible.trim_start_matches(|c: char| {
        c.is_whitespace() || c == '\u{a0}' || ('\u{e000}'..='\u{f8ff}').contains(&c)
        // Private Use Area — powerline glyphs
    });
    assert!(
        trimmed
            .chars()
            .next()
            .map(|c| c.is_alphanumeric() || c == '-' || c == '.')
            .unwrap_or(false),
        "expected hostname-like leading char after stripping divider glyphs, got {:?} (original: {:?})",
        trimmed,
        visible
    );
    // Tmux markup must be present (`#[fg=colour…]` for the styled
    // hostname). Drop the assertion if zero-length, but a real render
    // always emits the highlight prefix.
    assert!(
        body.contains("#["),
        "expected tmux markup tags in body: {:?}",
        body
    );
}

#[test]
fn scenario_date_renders_iso_date_and_hhmm_time() {
    let daemon = start_daemon("scenario_date");
    let raw = render_once(
        &daemon.socket,
        &["tmux", "right"],
        "/tmp",
        &[("HOME", "/tmp"), ("PWD", "/tmp")],
    );
    let body = String::from_utf8_lossy(&raw).to_string();
    let visible = strip_tmux_markup(&body);

    // ISO date pattern YYYY-MM-DD anywhere in the visible output.
    let mut iso_ok = false;
    let chars: Vec<char> = visible.chars().collect();
    for window in chars.windows(10) {
        if window[0].is_ascii_digit()
            && window[1].is_ascii_digit()
            && window[2].is_ascii_digit()
            && window[3].is_ascii_digit()
            && window[4] == '-'
            && window[5].is_ascii_digit()
            && window[6].is_ascii_digit()
            && window[7] == '-'
            && window[8].is_ascii_digit()
            && window[9].is_ascii_digit()
        {
            iso_ok = true;
            break;
        }
    }
    assert!(
        iso_ok,
        "expected ISO date YYYY-MM-DD in visible output: {:?}",
        visible
    );

    // HH:MM time pattern.
    let mut hhmm_ok = false;
    for window in chars.windows(5) {
        if window[0].is_ascii_digit()
            && window[1].is_ascii_digit()
            && window[2] == ':'
            && window[3].is_ascii_digit()
            && window[4].is_ascii_digit()
        {
            hhmm_ok = true;
            break;
        }
    }
    assert!(
        hhmm_ok,
        "expected HH:MM time in visible output: {:?}",
        visible
    );
}

#[test]
fn scenario_full_renders_all_six_segments() {
    let daemon = start_daemon("scenario_full");
    let raw = render_once(
        &daemon.socket,
        &["tmux", "right"],
        "/tmp",
        &[("HOME", "/tmp"), ("PWD", "/tmp")],
    );
    let body = String::from_utf8_lossy(&raw).to_string();
    let visible = strip_tmux_markup(&body);

    // CPU% segment ends with '%'.
    assert!(
        visible.contains('%'),
        "expected '%' from cpu_load_percent in: {:?}",
        visible
    );

    // ISO date present.
    let chars: Vec<char> = visible.chars().collect();
    let has_iso = chars.windows(10).any(|w| {
        w[0].is_ascii_digit()
            && w[1].is_ascii_digit()
            && w[2].is_ascii_digit()
            && w[3].is_ascii_digit()
            && w[4] == '-'
    });
    assert!(has_iso, "expected ISO date in: {:?}", visible);

    // HH:MM time present.
    let has_hhmm = chars
        .windows(5)
        .any(|w| w[0].is_ascii_digit() && w[2] == ':' && w[4].is_ascii_digit());
    assert!(has_hhmm, "expected HH:MM in: {:?}", visible);

    // Tmux markup tags emitted.
    let tag_count = body.matches("#[").count();
    assert!(
        tag_count >= 6,
        "expected at least 6 tmux style tags (one per segment), got {} in {:?}",
        tag_count,
        body
    );
}

#[test]
fn eof_sentinel_terminates_daemon_cleanly() {
    let daemon = start_daemon("scenario_hostname");
    let socket = daemon.socket.clone();

    // Send the EOF sentinel; the daemon's `do_read` matches it
    // verbatim and returns `Some(0)` from `do_one`, which propagates
    // to `main_loop` as exit code 0.
    let mut conn = UnixStream::connect(&socket).expect("connect for EOF");
    conn.write_all(SHUTDOWN_SENTINEL).expect("send EOF");
    drop(conn);

    // Wait for the daemon to exit. We give it 2 s — `shutdown()` in
    // the Rust daemon path includes a 2 s budget for thread join +
    // sleep, so we poll a little past that.
    let mut handle = daemon;
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut exited = false;
    while Instant::now() < deadline {
        match handle.child.try_wait() {
            Ok(Some(status)) => {
                exited = true;
                assert_eq!(
                    status.code(),
                    Some(0),
                    "daemon exited non-zero after EOF: {:?}",
                    status
                );
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,
        }
    }
    assert!(
        exited,
        "daemon did not exit within 3 s of EOF sentinel; pid {}",
        handle.child.id()
    );
}

#[test]
fn semantically_empty_request_does_not_crash_daemon() {
    let daemon = start_daemon("scenario_hostname");
    // Send a wire-format-valid but semantically-empty request: zero
    // args, empty cwd, no env. The daemon's `get_answer` will hit
    // either the parse-args fall-through or finish_args's "expected
    // one argument" error — both must surface as a normal response
    // rather than a daemon crash.
    let mut conn = UnixStream::connect(&daemon.socket).expect("connect");
    conn.write_all(b"0\0\0\0").expect("send empty request");
    let mut response = Vec::new();
    conn.set_read_timeout(Some(Duration::from_secs(3))).ok();
    let _ = conn.read_to_end(&mut response);
    drop(conn);

    // Daemon must still respond to a follow-up real render request.
    let raw = render_once(
        &daemon.socket,
        &["tmux", "right"],
        "/tmp",
        &[("HOME", "/tmp"), ("PWD", "/tmp")],
    );
    assert!(
        !raw.is_empty(),
        "daemon stopped responding after empty-args request"
    );
    assert!(
        String::from_utf8_lossy(&raw).contains("#["),
        "follow-up render had no tmux markup: {:?}",
        raw
    );
}
