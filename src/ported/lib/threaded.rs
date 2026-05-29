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

/// Port of `class ThreadedSegment(Segment, MultiRunnedThread)` from
/// `powerline/lib/threaded.py:33`.
///
/// Periodic-update segment base. Tracks crash state + the cached
/// update value. The actual `update`/`render` methods are
/// caller-supplied closures since Rust traits can't model Python's
/// `self.update`/`self.render` indirection without trait dispatch.
pub struct ThreadedSegment {
    pub base: MultiRunnedThread,
    /// Python class attribute: `min_sleep_time = 0.1` (py:34).
    pub min_sleep_time: f64,
    /// Python class attribute: `update_first = True` (py:35).
    pub update_first: bool,
    /// Python class attribute: `interval = 1` (py:36).
    pub interval: f64,
    /// Python: `self.run_once = True` (py:42).
    pub run_once: bool,
    /// Python: `self.crashed = False` (py:43).
    pub crashed: bool,
    /// Python: `self.crashed_value` (py:44).
    pub crashed_value: Option<String>,
    /// Python: `self.updated = False` (py:46).
    pub updated: bool,
    /// Python: `self.do_update_first` — set by set_state (py:121).
    pub do_update_first: bool,
}

impl Default for ThreadedSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl ThreadedSegment {
    /// Port of `ThreadedSegment.__init__()` from
    /// `powerline/lib/threaded.py:40`.
    pub fn new() -> Self {
        Self {
            base: MultiRunnedThread::new(),
            // py:34  min_sleep_time = 0.1
            min_sleep_time: 0.1,
            // py:35  update_first = True
            update_first: true,
            // py:36  interval = 1
            interval: 1.0,
            // py:42-46  initial state
            run_once: true,
            crashed: false,
            crashed_value: None,
            updated: false,
            do_update_first: true,
        }
    }

    /// Port of `ThreadedSegment.set_interval()` from
    /// `powerline/lib/threaded.py:109`.
    pub fn set_interval(&mut self, interval: Option<f64>) {
        // py:114-115  interval = interval or self.interval
        if let Some(i) = interval {
            self.interval = i;
        }
    }

    /// Port of `ThreadedSegment.set_state()` from
    /// `powerline/lib/threaded.py:117`.
    pub fn set_state(&mut self, interval: Option<f64>, update_first: bool) {
        // py:118-122
        self.set_interval(interval);
        self.do_update_first = update_first && self.update_first;
        self.updated = self.updated || !self.do_update_first;
    }

    /// Port of `ThreadedSegment.startup()` from
    /// `powerline/lib/threaded.py:123`.
    ///
    /// Marks the segment as no-longer-run-once and starts the worker.
    /// The actual run closure is caller-supplied since Rust can't
    /// dispatch via self.run.
    pub fn startup<F>(&mut self, use_daemon_threads: bool, run: F)
    where
        F: FnOnce(Arc<Mutex<bool>>) + Send + 'static,
    {
        // py:124-131
        self.run_once = false;
        self.base.daemon = use_daemon_threads;
        if !self.base.is_alive() {
            self.base.start_with(run);
        }
    }

    /// Port of `ThreadedSegment.shutdown()` from
    /// `powerline/lib/threaded.py:102`.
    pub fn shutdown(&self) {
        // py:103-107
        self.base.set_shutdown();
        // Python: if daemon and is_alive: join(0.01)
        // Rust port: join() blocks indefinitely; caller can opt to
        // detach by not calling join.
    }

    /// Port of `ThreadedSegment.set_update_value()` from
    /// `powerline/lib/threaded.py:69`.
    ///
    /// Wraps the caller-supplied update closure with crash handling.
    /// Returns the fresh update value or None when the update
    /// crashed (and sets `self.crashed = true`).
    pub fn set_update_value<F>(&mut self, mut update: F) -> Option<String>
    where
        F: FnMut() -> Result<String, String>,
    {
        // py:71-81  try update; except: crashed = True
        match update() {
            Ok(v) => {
                self.crashed = false;
                self.updated = true;
                Some(v)
            }
            Err(_) => {
                self.crashed = true;
                None
            }
        }
    }
}

/// Port of `class KwThreadedSegment(ThreadedSegment)` from
/// `powerline/lib/threaded.py:173`.
///
/// Multi-key variant — tracks per-key cached states with a separate
/// crashed flag per key. The Python class stores all per-key data
/// under `self.updates` (a dict keyed by the `key()` return value);
/// the Rust port mirrors that structure with a HashMap.
pub struct KwThreadedSegment {
    pub base: ThreadedSegment,
    /// Python: `self.updates` (py:181).
    pub updates: std::collections::HashMap<String, KwUpdateState>,
}

/// Per-key state stored in `KwThreadedSegment.updates`. Mirrors the
/// Python dict shape per py:218-226 inner loop.
#[derive(Debug, Clone)]
pub struct KwUpdateState {
    pub crashed: bool,
    pub value: Option<String>,
}

impl Default for KwThreadedSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl KwThreadedSegment {
    /// Port of `KwThreadedSegment.__init__()` from
    /// `powerline/lib/threaded.py:178`.
    pub fn new() -> Self {
        Self {
            base: ThreadedSegment::new(),
            // py:181  self.updates = {}
            updates: std::collections::HashMap::new(),
        }
    }

    /// Port of `KwThreadedSegment.update_one()` from
    /// `powerline/lib/threaded.py:218`.
    ///
    /// Updates the state for a single key via the caller's update
    /// closure. Crashed updates leave the previous value intact.
    pub fn update_one<F>(&mut self, key: &str, mut update: F)
    where
        F: FnMut() -> Result<String, String>,
    {
        // py:220-227  try update; except: crashed = True
        let state = self
            .updates
            .entry(key.to_string())
            .or_insert(KwUpdateState {
                crashed: false,
                value: None,
            });
        match update() {
            Ok(v) => {
                state.crashed = false;
                state.value = Some(v);
            }
            Err(_) => {
                state.crashed = true;
            }
        }
    }
}

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

    #[test]
    fn threaded_segment_defaults_match_upstream() {
        // py:34-36 class attributes
        let s = ThreadedSegment::new();
        assert!((s.min_sleep_time - 0.1).abs() < 1e-9);
        assert!(s.update_first);
        assert_eq!(s.interval, 1.0);
        assert!(s.run_once);
        assert!(!s.crashed);
        assert!(!s.updated);
    }

    #[test]
    fn threaded_segment_set_interval_overrides_default() {
        let mut s = ThreadedSegment::new();
        s.set_interval(Some(5.0));
        assert_eq!(s.interval, 5.0);
    }

    #[test]
    fn threaded_segment_set_interval_none_preserves_default() {
        // py:114  interval = interval or self.interval
        let mut s = ThreadedSegment::new();
        s.set_interval(None);
        assert_eq!(s.interval, 1.0);
    }

    #[test]
    fn threaded_segment_set_state_sets_do_update_first() {
        // py:120-122
        let mut s = ThreadedSegment::new();
        s.set_state(Some(2.0), true);
        assert!(s.do_update_first);
        assert_eq!(s.interval, 2.0);
    }

    #[test]
    fn threaded_segment_set_state_false_clears_do_update_first() {
        let mut s = ThreadedSegment::new();
        s.set_state(None, false);
        assert!(!s.do_update_first);
        // py:122  self.updated = self.updated or not self.do_update_first
        assert!(s.updated);
    }

    #[test]
    fn threaded_segment_set_update_value_ok_clears_crashed() {
        let mut s = ThreadedSegment::new();
        s.crashed = true;
        let v = s.set_update_value(|| Ok("data".to_string()));
        assert_eq!(v, Some("data".to_string()));
        assert!(!s.crashed);
        assert!(s.updated);
    }

    #[test]
    fn threaded_segment_set_update_value_err_sets_crashed() {
        // py:74  except: crashed = True
        let mut s = ThreadedSegment::new();
        let v = s.set_update_value(|| Err::<String, String>("boom".to_string()));
        assert!(v.is_none());
        assert!(s.crashed);
    }

    #[test]
    fn threaded_segment_shutdown_signals_thread() {
        let s = ThreadedSegment::new();
        s.shutdown();
        // shutdown_event flips to true
        assert!(*s.base.shutdown_event.lock().unwrap());
    }

    #[test]
    fn kw_threaded_segment_new_empty() {
        let s = KwThreadedSegment::new();
        assert!(s.updates.is_empty());
    }

    #[test]
    fn kw_threaded_segment_update_one_inserts_key() {
        let mut s = KwThreadedSegment::new();
        s.update_one("foo", || Ok("v".to_string()));
        let state = s.updates.get("foo").unwrap();
        assert!(!state.crashed);
        assert_eq!(state.value.as_deref(), Some("v"));
    }

    #[test]
    fn kw_threaded_segment_update_one_crash_preserves_value() {
        // py:226  failed update leaves the cached value
        let mut s = KwThreadedSegment::new();
        s.update_one("foo", || Ok("good".to_string()));
        s.update_one("foo", || Err::<String, String>("boom".to_string()));
        let state = s.updates.get("foo").unwrap();
        assert!(state.crashed);
        assert_eq!(state.value.as_deref(), Some("good"));
    }

    #[test]
    fn kw_threaded_segment_update_one_recovery_clears_crashed() {
        let mut s = KwThreadedSegment::new();
        s.update_one("foo", || Err::<String, String>("boom".to_string()));
        s.update_one("foo", || Ok("recovered".to_string()));
        let state = s.updates.get("foo").unwrap();
        assert!(!state.crashed);
        assert_eq!(state.value.as_deref(), Some("recovered"));
    }

    #[test]
    fn kw_threaded_segment_multiple_keys_isolated() {
        let mut s = KwThreadedSegment::new();
        s.update_one("a", || Ok("av".to_string()));
        s.update_one("b", || Err::<String, String>("err".to_string()));
        let a = s.updates.get("a").unwrap();
        let b = s.updates.get("b").unwrap();
        assert!(!a.crashed);
        assert!(b.crashed);
    }
}
