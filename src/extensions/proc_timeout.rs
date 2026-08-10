// vim:fileencoding=utf-8:noet
//! Bounded subprocess execution. Sanctioned non-port location per
//! `docs/PORT.md` — upstream `powerline/lib/shell.py` has no timeout
//! concept, so `src/ported/lib/shell.rs` must not grow one.
//!
//! Why this exists: the warm daemon renders a statusline every couple
//! of seconds, and several segments shell out. A single child that
//! never returns freezes the segment, and (before the render pool) the
//! whole daemon with it. This was not hypothetical — a stale login
//! session made every `osascript` in the Spotify segment block for
//! 30 s inside LaunchServices, so renders took 30–120 s and tmux
//! printed `<'…' not ready>` in place of the statusline.
//!
//! Contract: [`run_with_timeout`] either returns the child's completed
//! [`Output`] or it kills the child and returns `None`. It never
//! returns while leaving a process behind that we are still waiting on,
//! and it never blocks past `timeout + REAP_GRACE`.

use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long we are willing to block *after* SIGKILL waiting for the
/// child to be reaped. A process wedged in an uninterruptible kernel
/// wait cannot be killed at all; rather than hang the render we give up
/// and let the waiter thread reap it whenever the kernel lets go.
const REAP_GRACE: Duration = Duration::from_secs(2);

/// Run `cmd` to completion, or SIGKILL it once `timeout` elapses.
///
/// Returns `None` when the spawn fails, the wait fails, or the deadline
/// is hit. Stdout and stderr are drained on a helper thread, so a child
/// that writes more than a pipe buffer's worth cannot deadlock against
/// our own wait.
pub fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Option<Output> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = cmd.spawn().ok()?;
    let pid = child.id() as libc::pid_t;

    // `wait_with_output` both drains the pipes and reaps the child, so
    // the helper thread owns the whole lifecycle. We only ever observe
    // the result through the channel.
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    match rx.recv_timeout(timeout) {
        Ok(Ok(out)) => Some(out),
        Ok(Err(_)) => None,
        Err(_) => {
            // Deadline hit. Re-check the channel first: if the child
            // finished in the gap between the timeout firing and this
            // line, it has already been reaped and `pid` may have been
            // recycled onto an unrelated process — killing it would be
            // a bug far worse than a slow segment.
            if let Ok(Ok(out)) = rx.try_recv() {
                return Some(out);
            }
            // SAFETY: kill(2) with a pid we spawned and have not reaped
            // (the waiter thread still holds the `Child`). SIGKILL is
            // unblockable, so this terminates anything that is not
            // stuck in an uninterruptible kernel wait.
            unsafe { libc::kill(pid, libc::SIGKILL) };
            // Block briefly so the common case leaves no zombie behind.
            let _ = rx.recv_timeout(REAP_GRACE);
            None
        }
    }
}

/// [`run_with_timeout`] shaped like `ported::lib::shell::run_cmd`:
/// trimmed stdout on a successful exit, `None` on failure or timeout.
pub fn run_cmd_with_timeout(argv: &[&str], timeout: Duration) -> Option<String> {
    let mut cmd = Command::new(argv.first()?);
    cmd.args(&argv[1..]);
    let out = run_with_timeout(cmd, timeout)?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn completed_child_returns_output() {
        let mut cmd = Command::new("printf");
        cmd.arg("hello");
        let out = run_with_timeout(cmd, Duration::from_secs(5)).expect("printf should complete");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"hello");
    }

    /// The regression this module exists for: a child that outlives its
    /// budget must be killed, and the caller must be released roughly on
    /// the deadline rather than when the child would have finished.
    #[test]
    fn hung_child_is_killed_at_the_deadline() {
        let mut cmd = Command::new("sleep");
        cmd.arg("30");
        let t0 = Instant::now();
        let r = run_with_timeout(cmd, Duration::from_millis(200));
        let dt = t0.elapsed();
        assert!(r.is_none(), "a killed child must not report an Output");
        assert!(
            dt < Duration::from_secs(5),
            "caller waited {:?}, expected release near the 200ms deadline",
            dt
        );
    }

    /// A child that writes far more than one pipe buffer must not
    /// deadlock against our wait. 1 MiB is ~16 × the typical 64 KiB
    /// pipe, so a non-draining implementation blocks forever here.
    #[test]
    fn output_larger_than_a_pipe_buffer_still_completes() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "yes powerliners | head -c 1048576"]);
        let out = run_with_timeout(cmd, Duration::from_secs(20)).expect("should not deadlock");
        assert_eq!(out.stdout.len(), 1048576);
    }

    #[test]
    fn missing_binary_returns_none() {
        let cmd = Command::new("powerliners-nonexistent-binary");
        assert!(run_with_timeout(cmd, Duration::from_secs(1)).is_none());
    }

    #[test]
    fn run_cmd_with_timeout_trims_and_checks_status() {
        assert_eq!(
            run_cmd_with_timeout(&["printf", "  hi  "], Duration::from_secs(5)).as_deref(),
            Some("hi")
        );
        assert!(run_cmd_with_timeout(&["false"], Duration::from_secs(5)).is_none());
    }
}
