// vim:fileencoding=utf-8:noet
//! Stress tests: concurrent clients + sustained throughput.
//!
//! These complement `daemon_e2e.rs` by hammering the daemon's
//! lifecycle paths under load. Failures here indicate threadsafety
//! bugs, state-leak slowdowns, or fd-exhaustion in the daemon's
//! accept loop / poll vector / config-cache pipeline.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

const SHUTDOWN_SENTINEL: &[u8] = b"EOF\0\0";
const READ_TIMEOUT: Duration = Duration::from_secs(10);

fn daemon_binary() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
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

fn fixture_root(scenario: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/data/e2e");
    p.push(scenario);
    assert!(p.is_dir(), "fixture root missing: {}", p.display());
    p
}

fn unique_socket(tag: &str) -> PathBuf {
    // macOS caps UNIX socket paths around 104 chars (sockaddr_un.sun_path).
    // Always pick /tmp/<short> rather than $TMPDIR/... which on darwin
    // already eats ~50 chars before our filename.
    let p = PathBuf::from(format!(
        "/tmp/pls-{}-{}-{}",
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
    panic!("daemon never became ready on {}", socket.display());
}

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
    out.push(0);
    out.push(0);
    out
}

fn render_once(socket: &PathBuf, args: &[&str]) -> Vec<u8> {
    let mut conn = UnixStream::connect(socket).expect("connect");
    conn.set_read_timeout(Some(READ_TIMEOUT)).ok();
    conn.write_all(&build_request(args, "/tmp", &[("HOME", "/tmp"), ("PWD", "/tmp")]))
        .expect("send request");
    let mut buf = Vec::new();
    let _ = conn.read_to_end(&mut buf);
    buf
}

#[test]
fn concurrent_clients_all_get_valid_responses() {
    // 8 threads × 25 requests = 200 total. Each thread shares the
    // daemon socket but uses its own UnixStream per request — the
    // daemon's accept loop must demultiplex them correctly.
    let daemon = start_daemon("scenario_hostname", "concurrent");
    let socket = Arc::new(daemon.socket.clone());

    let n_threads = 8;
    let per_thread = 25;
    let mut handles = Vec::with_capacity(n_threads);
    let start = Instant::now();
    for _ in 0..n_threads {
        let sock = socket.clone();
        handles.push(std::thread::spawn(move || {
            let mut local_ok = 0;
            for _ in 0..per_thread {
                let resp = render_once(&sock, &["tmux", "right"]);
                if !resp.is_empty() && resp.windows(2).any(|w| w == b"#[") {
                    local_ok += 1;
                }
            }
            local_ok
        }));
    }
    let total_ok: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let elapsed = start.elapsed();
    let total = n_threads * per_thread;
    assert_eq!(
        total_ok, total,
        "expected {} valid responses, got {}",
        total, total_ok
    );
    println!(
        "concurrent: {} requests across {} threads in {:?} ({:.0} req/s)",
        total,
        n_threads,
        elapsed,
        total as f64 / elapsed.as_secs_f64()
    );
}

#[test]
fn sustained_throughput_does_not_degrade() {
    // Run 500 sequential requests; measure latency in two halves. If
    // the second half is dramatically slower than the first, the
    // daemon has a state-leak that's slowing per-request work.
    let daemon = start_daemon("scenario_hostname", "throughput");
    let total: usize = 500;
    let mut first_half_ns = 0u128;
    let mut second_half_ns = 0u128;
    let mut ok = 0;
    for i in 0..total {
        let t0 = Instant::now();
        let resp = render_once(&daemon.socket, &["tmux", "right"]);
        let dt = t0.elapsed().as_nanos();
        if !resp.is_empty() {
            ok += 1;
        }
        if i < total / 2 {
            first_half_ns += dt;
        } else {
            second_half_ns += dt;
        }
    }
    assert_eq!(ok, total, "some requests returned empty: ok={}", ok);
    let avg1 = first_half_ns as f64 / (total / 2) as f64;
    let avg2 = second_half_ns as f64 / (total / 2) as f64;
    println!(
        "sustained: first half avg {:.0} µs, second half avg {:.0} µs",
        avg1 / 1000.0,
        avg2 / 1000.0
    );
    // Allow some variation but flag pathological slowdowns (>4x).
    assert!(
        avg2 < avg1 * 4.0,
        "throughput degraded: first {:.0} µs vs second {:.0} µs",
        avg1 / 1000.0,
        avg2 / 1000.0
    );
}

#[test]
fn rapid_connect_disconnect_does_not_exhaust_fds() {
    // 500 connect+disconnect cycles in batches of 50. Each batch
    // sleeps briefly so the daemon's accept loop drains the listen
    // backlog (kernel default 128) before the next batch fills it.
    // The point of the test is fd-reaping in the daemon (verify Drop
    // closes accepted conns), not stressing the kernel's listen
    // queue. ECONNREFUSED here means backlog full, not fd leak —
    // separate concern.
    let daemon = start_daemon("scenario_hostname", "fd_exhaust");
    let mut refused = 0;
    for batch in 0..10 {
        for _ in 0..50 {
            match UnixStream::connect(&daemon.socket) {
                Ok(s) => drop(s),
                Err(_) => refused += 1,
            }
        }
        // Give the daemon time to accept-and-drain the queue.
        std::thread::sleep(Duration::from_millis(20));
        let _ = batch;
    }
    // Up to ~5% transient ECONNREFUSED is acceptable (kernel-side
    // backlog flutter), but the daemon must still respond afterward.
    assert!(
        refused < 30,
        "too many connection refusals ({}/500) — likely backlog issue, not fd leak",
        refused
    );
    let resp = render_once(&daemon.socket, &["tmux", "right"]);
    assert!(
        !resp.is_empty(),
        "daemon lost responsiveness after connect storm (refused: {})",
        refused
    );
}

#[test]
fn multiple_exts_keep_per_ext_config_cache_isolated() {
    // The daemon caches `Configs` per ext via `Mutex<HashMap<String,
    // Configs>>`. Interleaved tmux + shell requests should not
    // poison each other's cache state.
    let daemon = start_daemon("scenario_hostname", "multi_ext");
    // tmux comes back populated, shell ext is not in the fixture so
    // we expect a config-error body — but the daemon must keep
    // serving tmux after that.
    let r1 = render_once(&daemon.socket, &["tmux", "right"]);
    let r2 = render_once(&daemon.socket, &["shell", "left"]);
    let r3 = render_once(&daemon.socket, &["tmux", "right"]);
    assert!(!r1.is_empty() && r1.windows(2).any(|w| w == b"#["));
    // r2 should be the config-error message (no theme for `shell` in
    // the hostname fixture) — but the daemon must produce SOMETHING
    // rather than hang or crash.
    assert!(!r2.is_empty(), "shell ext rendered nothing");
    assert!(!r3.is_empty() && r3.windows(2).any(|w| w == b"#["));
}
