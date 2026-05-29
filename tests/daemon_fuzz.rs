// vim:fileencoding=utf-8:noet
//! Fuzz tests: random wire-format payloads + valid-payload mutations.
//!
//! These exercise the daemon's parsing and dispatch under adversarial
//! input. The goal is liveness — the daemon must survive every
//! payload and continue serving valid requests afterward. Crashes,
//! hangs, or response corruption are bugs.
//!
//! Uses a deterministic seeded PRNG (no external crate) so failures
//! are reproducible.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const SHUTDOWN_SENTINEL: &[u8] = b"EOF\0\0";

fn daemon_binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop();
    p.pop();
    p.push("powerline-daemon");
    assert!(p.exists(), "powerline-daemon binary missing");
    p
}

fn fixture_root(scenario: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/data/e2e");
    p.push(scenario);
    p
}

fn unique_socket(tag: &str) -> PathBuf {
    // macOS caps UNIX socket paths around 104 chars; use /tmp.
    let p = PathBuf::from(format!(
        "/tmp/plf-{}-{}-{}",
        tag,
        std::process::id() % 100000,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 10_000_000_000
    ));
    let _ = std::fs::remove_file(&p);
    p
}

struct DaemonHandle {
    child: Child,
    socket: PathBuf,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
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

fn start_daemon(scenario: &str, tag: &str) -> DaemonHandle {
    let socket = unique_socket(tag);
    let child = Command::new(daemon_binary())
        .arg("--foreground")
        .arg("--socket")
        .arg(&socket)
        .env("POWERLINE_CONFIG_PATHS", fixture_root(scenario))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn powerline-daemon");
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Ok(probe) = UnixStream::connect(&socket) {
            let _ = probe.shutdown(std::net::Shutdown::Both);
            return DaemonHandle { child, socket };
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("daemon never became ready");
}

fn build_valid_request() -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend(b"2\0tmux\0right\0/tmp\0HOME=/tmp\0\0");
    out
}

fn render_real(socket: &PathBuf) -> Vec<u8> {
    let mut conn = UnixStream::connect(socket).expect("connect");
    conn.set_read_timeout(Some(Duration::from_secs(5))).ok();
    conn.write_all(&build_valid_request()).expect("send");
    let mut buf = Vec::new();
    let _ = conn.read_to_end(&mut buf);
    buf
}

/// xorshift64 — small deterministic PRNG. No external crate.
struct Xs64 {
    s: u64,
}

impl Xs64 {
    fn new(seed: u64) -> Self {
        Self {
            s: if seed == 0 { 0xdeadbeef } else { seed },
        }
    }
    fn next(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.s = x;
        x
    }
    fn range(&mut self, lo: usize, hi: usize) -> usize {
        lo + (self.next() as usize) % (hi - lo).max(1)
    }
    fn byte(&mut self) -> u8 {
        (self.next() & 0xff) as u8
    }
}

/// Send a payload, read whatever, drop. Returns true if the
/// connection succeeded; we don't care about response content.
fn send_and_drop(socket: &PathBuf, payload: &[u8]) -> bool {
    match UnixStream::connect(socket) {
        Ok(mut conn) => {
            conn.set_read_timeout(Some(Duration::from_secs(3))).ok();
            let _ = conn.write_all(payload);
            // Half-close write side so the daemon's read returns; the
            // daemon should still read its 0-or-more bytes and produce
            // either a response or no response within 2 s (its
            // do_read timeout).
            let _ = conn.shutdown(std::net::Shutdown::Write);
            let mut buf = Vec::new();
            let _ = conn.read_to_end(&mut buf);
            true
        }
        Err(_) => false,
    }
}

#[test]
fn random_wire_bytes_keep_daemon_alive() {
    let daemon = start_daemon("scenario_hostname", "rand_wire");
    let mut rng = Xs64::new(0xc0ffee);
    // 200 random payloads of varied length. Bias toward short
    // payloads since that's the most common adversarial input.
    for _ in 0..200 {
        let len = rng.range(0, 1024);
        let mut payload: Vec<u8> = Vec::with_capacity(len);
        for _ in 0..len {
            payload.push(rng.byte());
        }
        assert!(
            send_and_drop(&daemon.socket, &payload),
            "daemon refused connection mid-fuzz; payload len={}",
            len
        );
    }
    // Sentinel: a real request must still work.
    let r = render_real(&daemon.socket);
    assert!(
        !r.is_empty() && r.windows(2).any(|w| w == b"#["),
        "daemon lost responsiveness after 200 random payloads"
    );
}

#[test]
fn mutated_valid_payload_keeps_daemon_alive() {
    let daemon = start_daemon("scenario_hostname", "mutate");
    let base = build_valid_request();
    let mut rng = Xs64::new(0xfeed);
    // 200 mutations of the base wire payload. Mutations: bit flips,
    // byte truncation, byte duplication, embedded nulls, length
    // prefix corruption.
    for _ in 0..200 {
        let mut payload = base.clone();
        let mutation = rng.range(0, 5);
        match mutation {
            0 => {
                // bit flip at random position
                if !payload.is_empty() {
                    let idx = rng.range(0, payload.len());
                    let bit = rng.range(0, 8);
                    payload[idx] ^= 1 << bit;
                }
            }
            1 => {
                // truncate by random amount
                let cut = rng.range(0, payload.len());
                payload.truncate(cut);
            }
            2 => {
                // duplicate a slice
                if payload.len() > 4 {
                    let s = rng.range(0, payload.len() - 1);
                    let e = rng.range(s, payload.len());
                    let slice: Vec<u8> = payload[s..e].to_vec();
                    payload.extend(slice);
                }
            }
            3 => {
                // sprinkle null bytes
                for _ in 0..rng.range(1, 8) {
                    let idx = rng.range(0, payload.len());
                    payload[idx] = 0;
                }
            }
            _ => {
                // corrupt argc hex prefix
                payload[0] = rng.byte();
                if payload.len() > 1 && payload[1] != 0 {
                    payload[1] = rng.byte();
                }
            }
        }
        assert!(
            send_and_drop(&daemon.socket, &payload),
            "daemon refused connection during mutation fuzz"
        );
    }
    let r = render_real(&daemon.socket);
    assert!(
        !r.is_empty() && r.windows(2).any(|w| w == b"#["),
        "daemon lost responsiveness after 200 mutations"
    );
}

#[test]
fn overlong_argc_value_does_not_oom_or_panic() {
    let daemon = start_daemon("scenario_hostname", "overlong");
    // The wire format starts with hex argc. Send a HUGE value to
    // see if parse_args bounds-checks before allocating. Daemon
    // must remain responsive.
    let mut payload: Vec<u8> = Vec::new();
    // 8-char hex (max u32) — `parse_args` parses via from_str_radix.
    payload.extend(b"ffffffff\0");
    // Then no args, no cwd, no env, just terminator. parse_args sees
    // numargs > parts available, returns None, get_answer surfaces
    // "malformed request" — daemon stays alive.
    payload.push(0);
    payload.push(0);
    assert!(send_and_drop(&daemon.socket, &payload));
    let r = render_real(&daemon.socket);
    assert!(!r.is_empty(), "daemon died from overlong argc");
}

#[test]
fn embedded_null_storm_does_not_break_parser() {
    let daemon = start_daemon("scenario_hostname", "null_storm");
    // 4096 alternating zeros and 'a' bytes; parse_args splits on \0
    // so this produces many empty fields. Daemon must handle them.
    let mut payload: Vec<u8> = Vec::with_capacity(8192);
    for i in 0..4096 {
        if i % 2 == 0 {
            payload.push(0);
        } else {
            payload.push(b'a');
        }
    }
    // Final terminator
    payload.push(0);
    payload.push(0);
    assert!(send_and_drop(&daemon.socket, &payload));
    let r = render_real(&daemon.socket);
    assert!(!r.is_empty(), "daemon died from null storm");
}

#[test]
fn partial_write_then_eof_does_not_hang_daemon() {
    let daemon = start_daemon("scenario_hostname", "partial");
    // Write a partial valid request, then close immediately. The
    // daemon's do_read should time out at 2 s, drop the conn,
    // continue serving. We send 30 partial requests then verify a
    // real one works.
    for _ in 0..30 {
        let mut conn = UnixStream::connect(&daemon.socket).expect("connect");
        let partial = &b"2\0tmux"[..];
        let _ = conn.write_all(partial);
        drop(conn);
    }
    // Wait past the daemon's 2 s read timeout to ensure all the
    // half-closed conns are reaped.
    std::thread::sleep(Duration::from_secs(3));
    let r = render_real(&daemon.socket);
    assert!(
        !r.is_empty() && r.windows(2).any(|w| w == b"#["),
        "daemon hung after partial writes"
    );
}

#[test]
fn extreme_env_count_handled() {
    let daemon = start_daemon("scenario_hostname", "many_envs");
    let mut payload: Vec<u8> = Vec::new();
    payload.extend(b"2\0tmux\0right\0/tmp\0");
    // 1000 env entries — way more than a normal client process has.
    for i in 0..1000 {
        payload.extend(format!("K{}=V{}\0", i, i).as_bytes());
    }
    payload.push(0);
    payload.push(0);
    let mut conn = UnixStream::connect(&daemon.socket).expect("connect");
    conn.set_read_timeout(Some(Duration::from_secs(5))).ok();
    conn.write_all(&payload).expect("send");
    let mut buf = Vec::new();
    let _ = conn.read_to_end(&mut buf);
    drop(conn);
    assert!(
        !buf.is_empty() && buf.windows(2).any(|w| w == b"#["),
        "daemon failed to render with 1000 env entries"
    );
}
