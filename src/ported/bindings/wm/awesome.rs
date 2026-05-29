// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/wm/awesome.py`.
//!
//! awesome WM bridge: spawns a `MultiRunnedThread` that renders the
//! statusline once per `interval` seconds and pipes the result into
//! `awesome-client`'s `powerline_widget:set_markup(...)` call.
//!
//! Most of the body depends on the unported `Powerline` class
//! (`update_renderer`, `render`, `update_interval`, `shutdown_event`).
//! This first chunk ports `read_to_log` (the awesome-client output
//! forwarder) and the `AwesomeThread` shell; the orchestrator `run()`
//! lands together with `Powerline.__init__`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// from threading import Thread, Event              // py:6
// from time import sleep                           // py:7
// from subprocess import Popen, PIPE               // py:8
// from powerline import Powerline                  // py:10
// from powerline.lib.monotonic import monotonic    // py:11

use crate::ported::lib::threaded::MultiRunnedThread;

/// Port of `read_to_log()` from
/// `powerline/bindings/wm/awesome.py:14`.
///
/// Reads the stdout + stderr of an `awesome-client` subprocess and
/// forwards each non-empty line to the powerline logger.
///
/// Rust port: takes the captured `Output` (since Rust subprocess APIs
/// give us full stdout/stderr after `wait_with_output`) and emits the
/// lines via `eprintln!` for now — the structured `pl.info` /
/// `pl.error` dispatch arrives with the Powerline logger trait.
pub fn read_to_log(_pl: &(), client: std::process::Output) {
    // py:14  def read_to_log(pl, client):
    // py:15  for line in client.stdout:
    for line in String::from_utf8_lossy(&client.stdout).lines() {
        // py:16  if line:
        if !line.is_empty() {
            // py:17  pl.info(line, prefix='awesome-client')
            eprintln!("awesome-client: {}", line);
        }
    }
    // py:18  for line in client.stderr:
    for line in String::from_utf8_lossy(&client.stderr).lines() {
        // py:19  if line:
        if !line.is_empty() {
            // py:20  pl.error(line, prefix='awesome-client')
            eprintln!("awesome-client: {}", line);
        }
    }
    // py:21  if client.wait():
    if !client.status.success() {
        // py:22  pl.error('Client exited with {0}', client.returncode, prefix='awesome')
        eprintln!("awesome-client: exited {}", client.status);
    }
}

/// Port of `run()` from `powerline/bindings/wm/awesome.py:24`.
///
/// Driver loop: constructs a Powerline, renders once per `interval`
/// seconds, pipes the result to `awesome-client`.
///
/// **Status:** stub — requires the Powerline class which isn't ported.
/// Returns immediately so the binding doesn't busy-loop when the
/// awesome thread is started against a not-yet-ported orchestrator.
pub fn run() {
    // py:25  def run(thread_shutdown_event=None, pl_shutdown_event=None, pl_config_loader=None,
    // py:26  interval=None):
    // py:27  powerline = Powerline(
    // py:28  'wm',
    // py:29  renderer_module='pango_markup',
    // py:30  shutdown_event=pl_shutdown_event,
    // py:31  config_loader=pl_config_loader,
    // py:32  )
    // py:33  powerline.update_renderer()
    // py:35  if not thread_shutdown_event:
    // py:36  thread_shutdown_event = powerline.shutdown_event
    // py:38  while not thread_shutdown_event.is_set():
    // py:39  # powerline.update_interval may change over time
    // py:40  used_interval = interval or powerline.update_interval
    // py:41  start_time = monotonic()
    // py:42  s = powerline.render(side='right')
    // py:43  request = 'powerline_widget:set_markup(\'' + s.translate(...) + '\')\n'
    // py:44  client = Popen(['awesome-client'], shell=False, stdout=PIPE, stderr=PIPE, stdin=PIPE)
    // py:45  client.stdin.write(request.encode('utf-8'))
    // py:46  client.stdin.close()
    // py:47  read_to_log(powerline.pl, client)
    // py:48  thread_shutdown_event.wait(max(used_interval - (monotonic() - start_time), 0.1))
    eprintln!(
        "powerliners: bindings::wm::awesome::run() — Powerline class not yet ported; \
         awesome WM integration disabled until Phase 2 lands"
    );
}

/// Port of `class AwesomeThread` from
/// `powerline/bindings/wm/awesome.py:47`.
///
/// Subclasses `Thread`; on `run()` calls the module-level `run` with
/// the kwargs captured at construction.
pub struct AwesomeThread {
    /// Underlying `MultiRunnedThread`.
    pub thread: MultiRunnedThread,
}

impl Default for AwesomeThread {
    fn default() -> Self {
        Self::new()
    }
}

impl AwesomeThread {
    /// Port of `AwesomeThread.__init__()` from
    /// `powerline/bindings/wm/awesome.py:50`.
    pub fn new() -> Self {
        Self {
            // py:51-53  super().__init__() + self.powerline_run_kwargs = kwargs
            thread: MultiRunnedThread::new(),
        }
    }

    /// Port of `AwesomeThread.run()` from
    /// `powerline/bindings/wm/awesome.py:55`.
    ///
    /// Python: `def run(self): run(**self.powerline_run_kwargs)`.
    /// Rust port spawns the run loop on the underlying
    /// `MultiRunnedThread`, calling the module-level `run` which is
    /// currently a stub.
    pub fn start(&self) {
        self.thread.start_with(|_event| {
            run();
        });
    }

    /// Mirror of MultiRunnedThread.join — wait for the worker.
    pub fn join(&self) {
        self.thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awesome_thread_starts_and_joins() {
        let t = AwesomeThread::new();
        t.start();
        t.join();
        // No panic = pass; the stub run() returns immediately.
    }

    #[test]
    fn read_to_log_does_not_panic_on_success() {
        let out = std::process::Output {
            status: std::process::Command::new("true")
                .status()
                .unwrap_or_else(|_| {
                    // Fallback for systems without /bin/true — fabricate a success.
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        std::process::ExitStatus::from_raw(0)
                    }
                    #[cfg(not(unix))]
                    {
                        std::process::Command::new("cmd")
                            .arg("/c")
                            .arg("exit 0")
                            .status()
                            .unwrap()
                    }
                }),
            stdout: b"hello\nworld\n".to_vec(),
            stderr: Vec::new(),
        };
        read_to_log(&(), out);
    }
}
