// vim:fileencoding=utf-8:noet
//! Integration tests for `src/bin/powerline.rs` — the thin native
//! client that tmux/shells invoke as `$POWERLINE_COMMAND`.
//!
//! Coverage targets the wire-format contract documented at
//! `src/bin/powerline.rs:11-12` (from the C client `client/powerline.c`
//! lines 126-148):
//!
//!   hex(argc-1) "\0" argv[1] "\0" … cwd "\0" env[0] "\0" … "\0\0"
//!
//! Strategy: spin up a mock Unix-domain listener that accepts the
//! client's connection, reads bytes until the request terminator, then
//! responds with a fixed payload. Pass the mock socket via `--socket
//! PATH`. The client process becomes a black box; we assert on (a) the
//! exact bytes it wrote and (b) the bytes it copied to stdout.
//!
//! No daemon, no powerline-render fallback exercised in these tests —
//! those paths are out of scope for the wire-format contract.

use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn client_binary() -> PathBuf {
    // Same walk pattern as daemon_e2e.rs:daemon_binary() — current_exe
    // is `.../target/debug/deps/bin_powerline_client-<hash>`; pop deps
    // to reach `.../target/debug/`, then sibling `powerline`.
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    p.pop();
    p.push("powerline");
    assert!(
        p.exists(),
        "powerline client binary missing at {} — run `cargo build --bin powerline` first",
        p.display()
    );
    p
}

/// Create a unique temporary socket path. Use a path-based socket on
/// every platform here (even Linux, where the production client uses
/// abstract sockets by default) because abstract sockets aren't
/// addressable via `--socket /path` and we want one test surface.
fn temp_socket_path(tag: &str) -> PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let mut p = std::env::temp_dir();
    p.push(format!("powerline-test-{}-{}-{}", tag, pid, nanos));
    p
}

/// Spawn a single-shot mock daemon on `path`. Returns a receiver that
/// yields `(request_bytes, ())` once the client disconnects, after the
/// mock has written `response` back to the client.
fn mock_daemon(path: PathBuf, response: Vec<u8>) -> mpsc::Receiver<Vec<u8>> {
    let (tx, rx) = mpsc::channel();
    let listener =
        UnixListener::bind(&path).unwrap_or_else(|e| panic!("bind {}: {e}", path.display()));
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        // Read the full request — terminated by "\0\0".
        let mut buf = Vec::with_capacity(4096);
        let mut tmp = [0u8; 4096];
        loop {
            let n = stream.read(&mut tmp).expect("read request");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&tmp[..n]);
            if buf.ends_with(b"\0\0") {
                break;
            }
        }
        stream.write_all(&response).expect("write response");
        // Drop = shutdown signals EOF to client.
        drop(stream);
        let _ = std::fs::remove_file(&path);
        tx.send(buf).expect("send request");
    });
    rx
}

/// Run the client with explicit `--socket` and a given argv tail.
/// Returns (stdout, exit_status).
fn run_client(socket: &PathBuf, args: &[&str]) -> (Vec<u8>, std::process::ExitStatus) {
    let mut cmd = Command::new(client_binary());
    cmd.arg("--socket").arg(socket);
    for a in args {
        cmd.arg(a);
    }
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    // Pin a deterministic cwd + minimal env so the request bytes are
    // predictable across machines. The client reads its own cwd and
    // env via std::env::current_dir / std::env::vars_os; clearing the
    // env first then setting just a few vars keeps the request small.
    let test_cwd = std::env::temp_dir();
    cmd.current_dir(&test_cwd);
    cmd.env_clear();
    cmd.env("PATH", "/usr/bin:/bin");
    cmd.env("POWERLINE_TEST", "1");
    let out = cmd.output().expect("spawn powerline client");
    (out.stdout, out.status)
}

/// Decode the client's wire request bytes — `hex_count "\0" arg "\0"
/// … cwd "\0" env0 "\0" … "\0\0"` — into its parts so tests can
/// assert on individual fields.
struct DecodedRequest<'a> {
    hex_count: &'a str,
    argv: Vec<&'a str>,
    cwd: &'a str,
    env: Vec<&'a str>,
}

fn decode_request(raw: &[u8]) -> DecodedRequest<'_> {
    // Wire ending is three consecutive NULs: <last_env_entry> NUL +
    // request-terminator NUL NUL (src/bin/powerline.rs:118-132). Strip
    // both terminator NULs so split() doesn't produce a phantom empty
    // segment between them.
    assert!(
        raw.ends_with(b"\0\0"),
        "request not terminated with double-NUL: {:?}",
        raw
    );
    let body = &raw[..raw.len() - 2];
    let mut parts: Vec<&str> = body
        .split(|b| *b == 0)
        .map(|s| std::str::from_utf8(s).expect("utf8 wire bytes"))
        .collect();
    // The last entry trailing the split is empty (from the env
    // entry's NUL terminator); drop it.
    if parts.last().map(|s| s.is_empty()).unwrap_or(false) {
        parts.pop();
    }
    let hex_count = parts.remove(0);
    let argc = usize::from_str_radix(hex_count, 16).expect("hex argc-1");
    let argv: Vec<&str> = parts.drain(..argc).collect();
    let cwd = parts.remove(0);
    let env: Vec<&str> = parts;
    DecodedRequest {
        hex_count,
        argv,
        cwd,
        env,
    }
}

// =====================================================================
// Wire-format contract tests
// =====================================================================

#[test]
fn no_args_exits_with_usage_message() {
    // C:93-96  "Must provide at least one argument."
    // Note: `--socket PATH` requires argv.len() > 3 at
    // src/bin/powerline.rs:43 to be consumed, so we must invoke with
    // NO args at all (just argv[0]) to hit argv.len() < 2.
    let mut cmd = Command::new(client_binary());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.env_clear();
    cmd.env("PATH", "/bin:/usr/bin");
    let out = cmd.output().expect("spawn");
    assert_eq!(out.status.code(), Some(1), "expected exit 1");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Must provide at least one argument"),
        "stdout missing usage message: {stdout:?}"
    );
}

#[test]
fn hex_count_is_lowercase_hex_of_argc_minus_one() {
    // C:126-127  argv[0] is the program, argv[1..] is the payload.
    // Wire prefix = lowercase hex of (argv.len() - 1).
    let socket = temp_socket_path("hex_count");
    let rx = mock_daemon(socket.clone(), b"resp\0".to_vec());
    let (_stdout, status) = run_client(&socket, &["render", "tmux", "left"]);
    assert!(status.success(), "client exit: {status:?}");
    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("got request");
    let req = decode_request(&raw);
    // argv passed = ["render", "tmux", "left"] → 3 entries → 0x3 → "3"
    assert_eq!(req.hex_count, "3");
    assert_eq!(req.argv, vec!["render", "tmux", "left"]);
}

#[test]
fn hex_count_uses_lowercase_for_values_above_9() {
    // 16 args → 0x10 → must be lowercase "10" (lowercase doesn't
    // matter for "10" itself, but ensures we exercise the multi-char
    // hex path — what we really care about is that the value matches
    // the C format spec `%x`).
    let socket = temp_socket_path("hex_count_big");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let many: Vec<String> = (0..16).map(|i| format!("a{i}")).collect();
    let many_refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let (_stdout, status) = run_client(&socket, &many_refs);
    assert!(status.success());
    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("got request");
    let req = decode_request(&raw);
    assert_eq!(req.hex_count, "10", "16 args must hex-encode as '10'");
    assert_eq!(req.argv.len(), 16);
}

#[test]
fn cwd_appears_after_argv_in_wire_format() {
    // C:135-141  cwd is appended after argv and before the env block.
    let socket = temp_socket_path("cwd_pos");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let (_stdout, _) = run_client(&socket, &["render"]);
    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("got request");
    let req = decode_request(&raw);
    // run_client sets cwd to temp_dir(); resolve symlinks for macOS's
    // /tmp → /private/tmp before comparing.
    let expected = std::fs::canonicalize(std::env::temp_dir()).expect("canon temp");
    let got = std::fs::canonicalize(req.cwd).expect("canon req cwd");
    assert_eq!(got, expected, "cwd field mismatch");
}

#[test]
fn env_block_contains_key_equals_value_entries() {
    // C:143-146  each entry is `K=V` + NUL.
    let socket = temp_socket_path("env_block");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let (_stdout, _) = run_client(&socket, &["render"]);
    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("got request");
    let req = decode_request(&raw);
    // run_client env_clear's then sets PATH + POWERLINE_TEST.
    assert!(
        req.env.contains(&"POWERLINE_TEST=1"),
        "env missing POWERLINE_TEST=1: {:?}",
        req.env
    );
    assert!(
        req.env.iter().any(|e| e.starts_with("PATH=")),
        "env missing PATH: {:?}",
        req.env
    );
    // No bare keys, every entry has '='.
    for e in &req.env {
        assert!(e.contains('='), "env entry without '=': {e:?}");
    }
}

#[test]
fn socket_flag_consumes_two_argv_slots() {
    // C:98-105  `--socket PATH` strips argv[1..=2] before counting.
    // Wire `argc-1` should not include the --socket pair.
    let socket = temp_socket_path("socket_flag");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let (_stdout, _) = run_client(&socket, &["render", "tmux"]);
    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("got request");
    let req = decode_request(&raw);
    assert_eq!(req.hex_count, "2", "--socket pair must not be in count");
    assert_eq!(req.argv, vec!["render", "tmux"]);
    assert!(
        !req.argv.contains(&"--socket"),
        "--socket leaked into argv payload"
    );
}

#[test]
fn response_bytes_pass_through_to_stdout_verbatim() {
    // C:150-159  client reads response from daemon and writes to
    // stdout. Including embedded NULs and high-byte data — the client
    // must not interpret the response.
    let socket = temp_socket_path("response");
    let payload: Vec<u8> = vec![
        0x1b, b'[', b'3', b'1', b'm', b'h', b'i', 0x1b, b'[', b'0', b'm', b'\n', 0xff, 0x00, 0xfe,
    ];
    let rx = mock_daemon(socket.clone(), payload.clone());
    let (stdout, status) = run_client(&socket, &["render"]);
    assert!(status.success(), "client exit: {status:?}");
    let _ = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("daemon recv");
    assert_eq!(
        stdout, payload,
        "stdout passthrough must be byte-exact, got {:?}",
        stdout
    );
}

#[test]
fn unreachable_socket_falls_back_via_execvp() {
    // C:117-123  When connect() fails, client execvp's
    // "powerline-render" with argv[1..]. We can't easily intercept
    // execvp's lookup without owning $PATH, so just confirm:
    //   - the client does NOT hang on a missing socket
    //   - it exits with a non-zero status when powerline-render fails
    //     to launch (the test rig has no powerline-render on its PATH)
    let socket = temp_socket_path("unreachable");
    // Deliberately do NOT spawn a listener — socket path doesn't exist.
    let mut cmd = Command::new(client_binary());
    cmd.arg("--socket").arg(&socket);
    cmd.arg("render");
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.env_clear();
    // Empty PATH guarantees execvp("powerline-render") can't resolve.
    cmd.env("PATH", "/nonexistent-path-for-test");
    let out = cmd.output().expect("spawn");
    assert!(
        !out.status.success(),
        "expected non-zero exit when fallback fails, got {:?}",
        out.status
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("execvp(powerline-render) failed"),
        "expected execvp failure stderr, got: {stderr:?}"
    );
}

#[test]
fn double_nul_terminator_ends_the_request() {
    // C:148  the request body must terminate with "\0\0". Our decoder
    // panics if the request isn't terminated correctly — this test
    // asserts the contract observably (and would catch a regression
    // where the client forgot the final write_all).
    let socket = temp_socket_path("dbl_nul");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let (_stdout, status) = run_client(&socket, &["render"]);
    assert!(status.success());
    let raw = rx.recv_timeout(Duration::from_secs(5)).expect("got req");
    // Raw bytes must end with two consecutive NULs.
    assert_eq!(
        &raw[raw.len() - 2..],
        b"\0\0",
        "missing double-NUL terminator at end of request"
    );
}

#[test]
fn separator_nul_after_hex_count() {
    // C:128  the separator NUL after the hex count must be present
    // even when argc-1 == 0 isn't a case we hit (the client requires
    // at least one argv entry). Verify the parsed structure: byte
    // immediately after the hex count is a NUL.
    let socket = temp_socket_path("hex_sep");
    let rx = mock_daemon(socket.clone(), b"r".to_vec());
    let (_stdout, _) = run_client(&socket, &["x"]);
    let raw = rx.recv_timeout(Duration::from_secs(5)).expect("got req");
    // Hex count is at most a few bytes — find the first NUL, assert
    // everything before it is ASCII-hex.
    let first_nul = raw.iter().position(|b| *b == 0).expect("has NUL");
    let prefix = &raw[..first_nul];
    for b in prefix {
        assert!(
            b.is_ascii_hexdigit(),
            "hex count contains non-hex byte: {:?}",
            prefix
        );
    }
    assert_eq!(
        raw[first_nul], 0,
        "expected NUL separator at position {first_nul}"
    );
}

// =====================================================================
// Helper-function unit tests (small, deterministic, no socket).
// These exercise pure logic via the decoder we use elsewhere.
// =====================================================================

#[test]
fn decoder_rejects_unterminated_request() {
    // Defensive: our test decoder must catch a missing terminator,
    // not silently pass it. Use a unique panic-catching approach.
    let result = std::panic::catch_unwind(|| {
        // Synthesize bytes missing the trailing NUL.
        decode_request(b"1\0arg\0cwd\0E=1\0");
    });
    // Single-NUL terminator (instead of double) should panic.
    assert!(result.is_err(), "decoder should reject missing terminator");
}

#[test]
fn decoder_round_trips_minimal_request() {
    // Wire format: hex(1) NUL arg NUL cwd NUL env-entry NUL + terminator NUL NUL.
    // Three trailing NULs total (entry-NUL + double-NUL terminator).
    let raw = b"1\0render\0/tmp\0K=V\0\0\0";
    let req = decode_request(raw);
    assert_eq!(req.hex_count, "1");
    assert_eq!(req.argv, vec!["render"]);
    assert_eq!(req.cwd, "/tmp");
    assert_eq!(req.env, vec!["K=V"]);
}
