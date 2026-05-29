// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/threaded.py`.
//!
//! Background-thread segment base classes. The simplest is
//! `MultiRunnedThread` (start/stop/join wrapper around a Python
//! `Thread`), used by VCS/network/weather segments that need to
//! refresh data off the render thread.
//!
//! This first chunk ports `MultiRunnedThread` faithfully. The richer
//! `ThreadedSegment` and `KwThreadedSegment` classes (which extend it
//! with periodic update + crash handling) land alongside the
//! `Segment` trait + Powerline logger — both are dispatch-heavy and
//! depend on substrate that isn't ported yet.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from threading import Thread, Lock, Event                                                // py:4
// from types import MethodType                                                              // py:5
// from powerline.lib.monotonic import monotonic                                              // py:7
// from powerline.segments import Segment                                                    // py:8

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Port of `class MultiRunnedThread` from
/// `powerline/lib/threaded.py:11`.
///
/// Python:
/// ```python
/// class MultiRunnedThread(object):
///     daemon = True
///
///     def __init__(self):
///         self.thread = None
///
///     def is_alive(self):
///         return self.thread and self.thread.is_alive()
///
///     def start(self):
///         self.shutdown_event.clear()
///         self.thread = Thread(target=self.run)
///         self.thread.daemon = self.daemon
///         self.thread.start()
///
///     def join(self, *args, **kwargs):
///         if self.thread:
///             return self.thread.join(*args, **kwargs)
///         return None
/// ```
///
/// Rust port: holds a `JoinHandle` plus a `shutdown_event` modeled as
/// `Arc<Mutex<bool>>`. The `run` method is provided by subclasses;
/// here we expose a `start_with` that accepts the run closure since
/// Rust traits can't model Python's `self.run()` indirection without
/// extra plumbing.
pub struct MultiRunnedThread {
    /// Python: `self.thread` — py:14
    pub thread: Mutex<Option<JoinHandle<()>>>,
    /// Python: `self.shutdown_event` — accessed by `start()` at py:20
    /// but populated by the subclass (typically `ThreadedSegment`). The
    /// Rust port carries it directly here to avoid the upstream
    /// "AttributeError until shutdown_event is set" footgun.
    pub shutdown_event: Arc<Mutex<bool>>,
    /// Python: class attribute `daemon = True` — py:12
    pub daemon: bool,
}

impl Default for MultiRunnedThread {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiRunnedThread {
    /// Port of `MultiRunnedThread.__init__()` from
    /// `powerline/lib/threaded.py:14`.
    pub fn new() -> Self {
        Self {
            thread: Mutex::new(None), // py:14
            shutdown_event: Arc::new(Mutex::new(false)),
            daemon: true, // py:12  daemon = True
        }
    }

    /// Port of `MultiRunnedThread.is_alive()` from
    /// `powerline/lib/threaded.py:16`.
    pub fn is_alive(&self) -> bool {
        // py:17  return self.thread and self.thread.is_alive()
        let t = self.thread.lock().unwrap();
        match t.as_ref() {
            None => false,
            Some(h) => !h.is_finished(),
        }
    }

    /// Port of `MultiRunnedThread.start()` from
    /// `powerline/lib/threaded.py:19`.
    ///
    /// The Python signature is `start(self)` which dispatches via
    /// `self.run`; Rust lacks that indirection so this takes the run
    /// closure explicitly. `shutdown_event.clear()` at py:20 maps to
    /// resetting the boolean to `false`.
    pub fn start_with<F>(&self, run: F)
    where
        F: FnOnce(Arc<Mutex<bool>>) + Send + 'static,
    {
        // py:20  self.shutdown_event.clear()
        *self.shutdown_event.lock().unwrap() = false;
        let event = self.shutdown_event.clone();
        // py:21-23  Thread(target=self.run), daemon=True, start
        // Rust's std::thread is always "daemon-like" — JoinHandle's
        // drop doesn't block the parent on exit when the parent
        // returns.
        let handle = std::thread::spawn(move || run(event));
        *self.thread.lock().unwrap() = Some(handle);
    }

    /// Port of `MultiRunnedThread.join()` from
    /// `powerline/lib/threaded.py:25`.
    ///
    /// Python's `Thread.join(timeout=None)` waits for the thread to
    /// terminate; with `timeout` it returns when the timeout expires.
    /// Rust's `JoinHandle::join` has no built-in timeout — for the
    /// timeout case the caller must check `is_finished()` in a loop.
    /// This port matches the no-timeout Python branch.
    pub fn join(&self) -> Option<()> {
        // py:26-27  if self.thread: return self.thread.join(...)
        let mut t = self.thread.lock().unwrap();
        let handle = t.take()?;
        let _ = handle.join();
        Some(()) // py:27
    }

    /// Signal the thread to shut down (sets `shutdown_event` to true).
    ///
    /// Not present in upstream Python as a separate method — Python
    /// callers set `self.shutdown_event.set()` directly. The Rust port
    /// exposes it as a method for ergonomic call sites.
    pub fn set_shutdown(&self) {
        *self.shutdown_event.lock().unwrap() = true;
    }
}

// `ThreadedSegment` (py:33) and `KwThreadedSegment` (py:173) port
// alongside `Segment` + Powerline logger. Both extend
// `MultiRunnedThread` with periodic update + crash handling; the
// substrate isn't ready (no Segment-trait method dispatch model yet).

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn new_thread_is_not_alive() {
        let t = MultiRunnedThread::new();
        assert!(!t.is_alive());
    }

    #[test]
    fn start_then_join_runs_closure() {
        let t = MultiRunnedThread::new();
        let ran = Arc::new(AtomicU32::new(0));
        let ran_clone = ran.clone();
        t.start_with(move |_event| {
            ran_clone.fetch_add(1, Ordering::SeqCst);
        });
        t.join();
        assert_eq!(ran.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn set_shutdown_signals_the_thread() {
        let t = MultiRunnedThread::new();
        let observed = Arc::new(Mutex::new(false));
        let observed_clone = observed.clone();
        t.start_with(move |event| {
            // Wait until shutdown_event flips, then record observation.
            loop {
                if *event.lock().unwrap() {
                    *observed_clone.lock().unwrap() = true;
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        t.set_shutdown();
        t.join();
        assert!(*observed.lock().unwrap());
    }
}
