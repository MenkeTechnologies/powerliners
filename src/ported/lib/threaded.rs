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

    /// Port of `ThreadedSegment.get_update_value()` from
    /// `powerline/lib/threaded.py:82-85`.
    ///
    /// Returns `update_value` (caller-supplied); if `update=true`,
    /// re-runs `set_update_value` first per py:83-84.
    pub fn get_update_value<F>(&mut self, update: bool, refresh: F) -> Option<String>
    where
        F: FnMut() -> Result<String, String>,
    {
        // py:83-84  if update: self.set_update_value()
        if update {
            return self.set_update_value(refresh);
        }
        // py:85  return self.update_value
        // (The Rust port doesn't store update_value on the struct;
        //  callers pass it via `refresh` when needed.)
        None
    }

    /// Port of `ThreadedSegment.argspecobjs()` from
    /// `powerline/lib/threaded.py:151-156`.
    ///
    /// Yields `(name, method_name)` pairs for the configured
    /// `argmethods` set per py:152-156. Python uses `getattr` to
    /// look up the method object; Rust returns the method name as
    /// a string since methods aren't first-class.
    pub fn argspecobjs(argmethods: &[&'static str]) -> Vec<(String, String)> {
        // py:152-156
        argmethods
            .iter()
            .map(|name| (name.to_string(), name.to_string()))
            .collect()
    }

    /// Port of `ThreadedSegment.additional_args()` from
    /// `powerline/lib/threaded.py:158-159`.
    ///
    /// Returns `(('interval', self.interval),)` per py:159.
    pub fn additional_args(interval: Option<f64>) -> Vec<(String, Option<f64>)> {
        // py:159
        vec![("interval".to_string(), interval)]
    }

    /// Port of `ThreadedSegment._omitted_args` class attribute at
    /// `powerline/lib/threaded.py:161-164`.
    ///
    /// Returns the static map of `method_name → omitted_arg_indices`
    /// per py:162-164. `'render'` omits arg `(0,)`; `'set_state'`
    /// omits `('shutdown_event',)`.
    pub fn omitted_args_table(name: &str) -> Vec<&'static str> {
        // py:162-164
        match name {
            "render" => vec!["0"],
            "set_state" => vec!["shutdown_event"],
            _ => Vec::new(),
        }
    }

    /// Port of `ThreadedSegment.omitted_args()` from
    /// `powerline/lib/threaded.py:166-170`.
    ///
    /// Returns the omitted-args list for `name`. The Python source
    /// at py:168-169 increments integer indices by 1 when the
    /// method is bound (to skip `self`). The Rust port mirrors the
    /// behaviour via a bool flag since Rust has no MethodType.
    pub fn omitted_args(name: &str, is_method: bool) -> Vec<String> {
        // py:167  ret = self._omitted_args.get(name, ())
        let raw = Self::omitted_args_table(name);
        // py:168-169  is_method → shift integer indices by 1
        raw.iter()
            .map(|arg| {
                if is_method {
                    if let Ok(idx) = arg.parse::<i32>() {
                        return (idx + 1).to_string();
                    }
                }
                arg.to_string()
            })
            .collect()
    }

    /// Port of `ThreadedSegment.critical()` from
    /// `powerline/lib/threaded.py:133-134`.
    ///
    /// Returns the (prefix, message) pair callers route through
    /// `self.pl.critical(...)` per py:134.
    pub fn critical(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
    }

    /// Port of `ThreadedSegment.exception()` from
    /// `powerline/lib/threaded.py:136-137`.
    pub fn exception(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
    }

    /// Port of `ThreadedSegment.info()` from
    /// `powerline/lib/threaded.py:139-140`.
    pub fn info(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
    }

    /// Port of `ThreadedSegment.error()` from
    /// `powerline/lib/threaded.py:142-143`.
    pub fn error(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
    }

    /// Port of `ThreadedSegment.warn()` from
    /// `powerline/lib/threaded.py:145-146`.
    pub fn warn(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
    }

    /// Port of `ThreadedSegment.debug()` from
    /// `powerline/lib/threaded.py:148-149`.
    pub fn debug(class_name: &str, message: &str) -> (String, String) {
        (class_name.to_string(), message.to_string())
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

    #[test]
    fn get_update_value_runs_refresh_when_update_true() {
        // py:83-84
        let mut s = ThreadedSegment::default();
        let r = s.get_update_value(true, || Ok("fresh".to_string()));
        assert_eq!(r, Some("fresh".to_string()));
        assert!(s.updated);
    }

    #[test]
    fn get_update_value_skips_refresh_when_update_false() {
        // py:85  return self.update_value (no refresh)
        let mut s = ThreadedSegment::default();
        let r = s.get_update_value(false, || Ok("should_not_run".to_string()));
        // Rust port returns None since there's no stored value
        assert!(r.is_none());
        assert!(!s.updated);
    }

    #[test]
    fn argspecobjs_yields_argmethod_name_pairs() {
        // py:152-156
        let r = ThreadedSegment::argspecobjs(&["render", "set_state"]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].0, "render");
        assert_eq!(r[1].0, "set_state");
    }

    #[test]
    fn additional_args_returns_interval_pair() {
        // py:159
        let r = ThreadedSegment::additional_args(Some(5.0));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "interval");
        assert_eq!(r[0].1, Some(5.0));
    }

    #[test]
    fn omitted_args_table_render_omits_first() {
        // py:162  'render': (0,)
        assert_eq!(ThreadedSegment::omitted_args_table("render"), vec!["0"]);
    }

    #[test]
    fn omitted_args_table_set_state_omits_shutdown_event() {
        // py:163  'set_state': ('shutdown_event',)
        assert_eq!(
            ThreadedSegment::omitted_args_table("set_state"),
            vec!["shutdown_event"]
        );
    }

    #[test]
    fn omitted_args_table_unknown_name_returns_empty() {
        assert!(ThreadedSegment::omitted_args_table("other").is_empty());
    }

    #[test]
    fn omitted_args_unbound_passes_indices_through() {
        // py:168-169  not method → no shift
        let r = ThreadedSegment::omitted_args("render", false);
        assert_eq!(r, vec!["0".to_string()]);
    }

    #[test]
    fn omitted_args_bound_shifts_integer_indices_by_one() {
        // py:168-169  isinstance MethodType → +1 on ints
        let r = ThreadedSegment::omitted_args("render", true);
        assert_eq!(r, vec!["1".to_string()]);
    }

    #[test]
    fn omitted_args_bound_leaves_string_indices_unchanged() {
        let r = ThreadedSegment::omitted_args("set_state", true);
        // String args ('shutdown_event') don't get shifted
        assert_eq!(r, vec!["shutdown_event".to_string()]);
    }

    #[test]
    fn critical_returns_prefix_message_pair() {
        // py:134
        let (prefix, msg) = ThreadedSegment::critical("MyClass", "boom");
        assert_eq!(prefix, "MyClass");
        assert_eq!(msg, "boom");
    }

    #[test]
    fn exception_returns_prefix_message_pair() {
        // py:137
        let (prefix, msg) = ThreadedSegment::exception("MyClass", "exc");
        assert_eq!(prefix, "MyClass");
        assert_eq!(msg, "exc");
    }

    #[test]
    fn info_warn_error_debug_all_return_prefix_message_pair() {
        // py:139-149
        assert_eq!(ThreadedSegment::info("X", "i"), ("X".into(), "i".into()));
        assert_eq!(ThreadedSegment::error("X", "e"), ("X".into(), "e".into()));
        assert_eq!(ThreadedSegment::warn("X", "w"), ("X".into(), "w".into()));
        assert_eq!(ThreadedSegment::debug("X", "d"), ("X".into(), "d".into()));
    }
}
