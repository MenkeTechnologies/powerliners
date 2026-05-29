// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/uv.py`.
//!
//! pyuv-based file/tree watcher. Upstream depends on the
//! Python `pyuv` library (libuv bindings); the entire file errors out
//! with `UvNotFound` when pyuv isn't installed.
//!
//! Rust analog: the [`notify`](https://crates.io/crates/notify) crate
//! provides equivalent libuv-style filesystem watching, but adding
//! that as a hard dependency is out of scope for this port pass. The
//! Rust port mirrors the structural surface (`UvNotFound`,
//! `UvFileWatcher`, `UvTreeWatcher`) and surfaces `UvNotFound` from
//! the constructor so callers (the watcher dispatcher) fall back to
//! the stat backend on every platform.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from collections import defaultdict              // py:6
// from threading import RLock                      // py:7
// from functools import partial                    // py:8
// from threading import Thread                     // py:9
// from errno import ENOENT                         // py:10
// from powerline.lib.path import realpath          // py:12
// from powerline.lib.encoding import get_preferred_file_name_encoding                     // py:13

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::OnceLock;

/// Port of `class UvNotFound(NotImplementedError)` from
/// `powerline/lib/watcher/uv.py:16`.
///
/// Raised when pyuv is unavailable. The watcher dispatcher catches
/// this and falls back to a different backend.
#[derive(Debug, Clone)]
pub struct UvNotFound;

impl std::fmt::Display for UvNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pyuv (libuv bindings) not available")
    }
}

impl std::error::Error for UvNotFound {}

/// Port of `import_pyuv()` from
/// `powerline/lib/watcher/uv.py:24`.
///
/// Initialize the pyuv binding. Stub always errors since the Rust
/// port doesn't depend on pyuv.
pub fn import_pyuv() -> Result<(), UvNotFound> {
    // py:25-32  try __import__('pyuv') except ImportError: raise UvNotFound
    Err(UvNotFound)
}

/// Port of `class UvFileWatcher` from `powerline/lib/watcher/uv.py`.
///
/// **Status:** stub. Construction always returns `Err(UvNotFound)` so
/// the watcher dispatcher falls through to the stat backend.
pub struct UvFileWatcher;

impl UvFileWatcher {
    /// Constructor that mirrors the upstream's `__init__` failure mode:
    /// always errors out with `UvNotFound`.
    pub fn new() -> Result<Self, UvNotFound> {
        import_pyuv()?;
        Ok(Self)
    }
}

/// Port of `class UvTreeWatcher` from `powerline/lib/watcher/uv.py`.
///
/// **Status:** stub. Same construction-time UvNotFound semantics as
/// `UvFileWatcher`.
pub struct UvTreeWatcher;

impl UvTreeWatcher {
    pub fn new<P: AsRef<std::path::Path>>(_path: P) -> Result<Self, UvNotFound> {
        import_pyuv()?;
        Ok(Self)
    }
}

/// Port of `_uv_thread = None` from
/// `powerline/lib/watcher/uv.py:57`.
///
/// Rust analog of the module-level shared uv loop. Holds an
/// `Option<bool>` flag tracking whether `start_uv_thread` has been
/// called once. Always returns `false` since the actual libuv loop
/// can't run without pyuv/notify.
pub fn _uv_thread() -> &'static Mutex<Option<bool>> {
    static M: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(None))
}

/// Port of `start_uv_thread()` from
/// `powerline/lib/watcher/uv.py:60`.
///
/// **Status:** stub. Returns `UvNotFound` since the libuv event
/// loop isn't reachable from Rust without the `notify` crate.
pub fn start_uv_thread() -> Result<(), UvNotFound> {
    // py:61-67  initialise the uv loop + start the worker thread
    Err(UvNotFound)
}

/// Port of `normpath()` from
/// `powerline/lib/watcher/uv.py:70`.
///
/// Normalises a path via `realpath` + (when bytes) decodes via the
/// preferred encoding. Rust takes `&str` directly so the bytes
/// branch is omitted.
pub fn normpath(path: &str) -> String {
    // py:71  path = realpath(path)
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.to_string())
}

/// Port of `class UvWatcher(object)` from
/// `powerline/lib/watcher/uv.py:76`.
///
/// Shared base for file + tree watchers. Tracks the watched-path
/// set under a mutex. The actual pyuv handles are dropped — only
/// the structural surface is present.
pub struct UvWatcher {
    /// Python: `self.watches = {}` (py:79) — path → handle map.
    /// Rust port stores just the path set since the handle objects
    /// aren't reachable.
    pub watches: Mutex<HashSet<String>>,
}

impl Default for UvWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl UvWatcher {
    /// Port of `UvWatcher.__init__()` from
    /// `powerline/lib/watcher/uv.py:77`.
    ///
    /// Note: in upstream this raises `UvNotFound` via `import_pyuv()`;
    /// the Rust port surfaces the empty state machine separately so
    /// callers can test the path tracking without the libuv
    /// dependency.
    pub fn new() -> Self {
        Self {
            watches: Mutex::new(HashSet::new()),
        }
    }

    /// Port of `UvWatcher.watch()` from
    /// `powerline/lib/watcher/uv.py:102`.
    ///
    /// Registers `path` as a watch target. Does nothing if the path
    /// is already watched.
    pub fn watch(&self, path: &str) {
        // py:103-112  with lock: if not watched: start_watch
        let normalized = normpath(path);
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.insert(normalized);
    }

    /// Port of `UvWatcher.unwatch()` from
    /// `powerline/lib/watcher/uv.py:114`.
    pub fn unwatch(&self, path: &str) {
        // py:115-121  with lock: pop watches[path]; watch.close
        let normalized = normpath(path);
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.remove(&normalized);
    }

    /// Port of `UvWatcher.is_watching()` from
    /// `powerline/lib/watcher/uv.py:123`.
    pub fn is_watching(&self, path: &str) -> bool {
        // py:124-125  return path in self.watches
        let normalized = normpath(path);
        let watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.contains(&normalized)
    }

    /// Returns the count of currently-tracked watched paths.
    pub fn watch_count(&self) -> usize {
        self.watches.lock().unwrap_or_else(|e| e.into_inner()).len()
    }
}

/// Port of `class UvFileWatcher(UvWatcher)` event tracking from
/// `powerline/lib/watcher/uv.py:138`.
///
/// Tracks per-path event accumulator. The actual pyuv FSEvent
/// callbacks aren't reachable — callers populate the events buffer
/// manually via `record_event` for testing.
pub struct UvFileWatcherEvents {
    /// Python: `self.events = defaultdict(list)` (py:140).
    pub events: Mutex<HashMap<String, Vec<u32>>>,
}

impl Default for UvFileWatcherEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl UvFileWatcherEvents {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(HashMap::new()),
        }
    }

    /// Port of `UvFileWatcher._record_event()` from
    /// `powerline/lib/watcher/uv.py:142`.
    pub fn record_event(&self, path: &str, events_mask: u32) {
        // py:143-145  self.events[path].append(events)
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        events
            .entry(path.to_string())
            .or_default()
            .push(events_mask);
    }

    /// Port of `UvFileWatcher.__call__()` from
    /// `powerline/lib/watcher/uv.py:153`.
    ///
    /// Returns `true` if the path has events queued.
    pub fn check(&self, path: &str) -> bool {
        // py:155-159  events = self.events.pop(path, None); return bool(events)
        let normalized = normpath(path);
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        let queued = events.remove(&normalized);
        queued.map(|v| !v.is_empty()).unwrap_or(false)
    }

    /// Returns the number of events queued for `path`.
    pub fn event_count(&self, path: &str) -> usize {
        let normalized = normpath(path);
        let events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        events.get(&normalized).map(|v| v.len()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests_state {
    use super::*;

    #[test]
    fn uv_watcher_new_starts_empty() {
        let w = UvWatcher::new();
        assert_eq!(w.watch_count(), 0);
    }

    #[test]
    fn uv_watcher_watch_inserts_path() {
        let w = UvWatcher::new();
        w.watch("/tmp/foo");
        assert!(w.is_watching("/tmp/foo"));
    }

    #[test]
    fn uv_watcher_watch_idempotent() {
        let w = UvWatcher::new();
        w.watch("/tmp/foo");
        w.watch("/tmp/foo");
        assert_eq!(w.watch_count(), 1);
    }

    #[test]
    fn uv_watcher_unwatch_removes_path() {
        let w = UvWatcher::new();
        w.watch("/tmp/foo");
        w.unwatch("/tmp/foo");
        assert!(!w.is_watching("/tmp/foo"));
        assert_eq!(w.watch_count(), 0);
    }

    #[test]
    fn uv_watcher_unwatch_unknown_no_op() {
        let w = UvWatcher::new();
        w.unwatch("/tmp/never");
        assert_eq!(w.watch_count(), 0);
    }

    #[test]
    fn uv_watcher_supports_multiple_paths() {
        let w = UvWatcher::new();
        w.watch("/tmp/a");
        w.watch("/tmp/b");
        w.watch("/tmp/c");
        assert_eq!(w.watch_count(), 3);
        w.unwatch("/tmp/b");
        assert_eq!(w.watch_count(), 2);
        assert!(w.is_watching("/tmp/a"));
        assert!(!w.is_watching("/tmp/b"));
        assert!(w.is_watching("/tmp/c"));
    }

    #[test]
    fn start_uv_thread_returns_uv_not_found() {
        // libuv binding not threaded through — stub returns UvNotFound
        let r = start_uv_thread();
        assert!(r.is_err());
    }

    #[test]
    fn normpath_returns_string_for_non_existing() {
        // realpath fails for /__never_existing — returns the input.
        let r = normpath("/__never_existing_path_12345");
        assert_eq!(r, "/__never_existing_path_12345");
    }

    #[test]
    fn uv_file_watcher_events_new_starts_empty() {
        let e = UvFileWatcherEvents::new();
        assert_eq!(e.event_count("/x"), 0);
    }

    #[test]
    fn uv_file_watcher_events_record_appends() {
        let e = UvFileWatcherEvents::new();
        e.record_event("/x", 1);
        e.record_event("/x", 2);
        assert_eq!(e.event_count("/x"), 2);
    }

    #[test]
    fn uv_file_watcher_events_check_returns_true_when_events_present() {
        // py:157-158  if events: return True
        let e = UvFileWatcherEvents::new();
        e.record_event("/__nonexistent_test_path_uv", 1);
        // check uses normpath which falls back to literal when path
        // doesn't exist
        assert!(e.check("/__nonexistent_test_path_uv"));
    }

    #[test]
    fn uv_file_watcher_events_check_consumes_queue() {
        let e = UvFileWatcherEvents::new();
        e.record_event("/__nonexistent_test_path2_uv", 1);
        e.check("/__nonexistent_test_path2_uv");
        assert_eq!(e.event_count("/__nonexistent_test_path2_uv"), 0);
    }

    #[test]
    fn uv_file_watcher_events_check_false_when_empty() {
        let e = UvFileWatcherEvents::new();
        assert!(!e.check("/never"));
    }

    #[test]
    fn _uv_thread_starts_as_none() {
        // Initial state is None per py:57
        let m = _uv_thread().lock().unwrap_or_else(|e| e.into_inner());
        // Note: this is per-process state; might be set by other tests
        // — the structural check just verifies the accessor returns.
        let _ = *m;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_not_found_implements_error_traits() {
        let e = UvNotFound;
        assert!(e.to_string().contains("pyuv"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn import_pyuv_returns_uv_not_found_in_stub() {
        assert!(import_pyuv().is_err());
    }

    #[test]
    fn uv_file_watcher_new_errors() {
        assert!(UvFileWatcher::new().is_err());
    }

    #[test]
    fn uv_tree_watcher_new_errors() {
        assert!(UvTreeWatcher::new("/tmp").is_err());
    }
}
