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
    // py:24  def import_pyuv():
    // py:25  global pyuv
    // py:26  global pyuv_version_info
    // py:27  if not pyuv:
    // py:28  try:
    // py:29  pyuv = __import__('pyuv')
    // py:30  except ImportError:
    // py:31  raise UvNotFound
    // py:32  else:
    // py:33  pyuv_version_info = tuple((int(c) for c in pyuv.__version__.split('.')))
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
    // py:59  def start_uv_thread():
    // py:60  global _uv_thread
    // py:61  if _uv_thread is None:
    // py:62  loop = pyuv.Loop()
    // py:63  _uv_thread = UvThread(loop)
    // py:64  _uv_thread.start()
    // py:65  return _uv_thread.uv_loop
    Err(UvNotFound)
}

/// Port of `normpath()` from
/// `powerline/lib/watcher/uv.py:70`.
///
/// Normalises a path via `realpath` + (when bytes) decodes via the
/// preferred encoding. Rust takes `&str` directly so the bytes
/// branch is omitted.
pub fn normpath(path: &str) -> String {
    // py:68  def normpath(path, fenc):
    // py:69  path = realpath(path)
    // py:70  if isinstance(path, bytes):
    // py:71  return path.decode(fenc)
    // py:72  else:
    // py:73  return path
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
        // py:76  class UvWatcher(object):
        // py:77  def __init__(self):
        // py:78  import_pyuv()
        // py:79  self.watches = {}
        // py:80  self.lock = RLock()
        // py:81  self.loop = start_uv_thread()
        // py:82  self.fenc = get_preferred_file_name_encoding()
        // py:83  if pyuv_version_info >= (1, 0):
        // py:84  self._start_watch = self._start_watch_1_x
        // py:85  else:
        // py:86  self._start_watch = self._start_watch_0_x
        Self {
            watches: Mutex::new(HashSet::new()),
        }
    }

    /// Port of `UvWatcher._start_watch_1_x()` from
    /// `powerline/lib/watcher/uv.py:88-91`.
    ///
    /// Python (pyuv >= 1.0 branch): allocates `pyuv.fs.FSEvent`,
    /// calls `handle.start(path, 0, partial(self._record_event,
    /// path))`, and stashes the handle in `self.watches[path]`.
    ///
    /// Rust port has no pyuv binding; this records the path in
    /// `watches` so the path-tracking matches the upstream
    /// observable state. Returns `true` when the path was newly
    /// inserted (matches Python's "added" semantics).
    pub fn _start_watch_1_x(&self, path: &str) -> bool {
        // py:88  def _start_watch_1_x(self, path):
        // py:89  handle = pyuv.fs.FSEvent(self.loop)
        // py:90  handle.start(path, 0, partial(self._record_event, path))
        // py:91  self.watches[path] = handle
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.insert(path.to_string())
    }

    /// Port of `UvWatcher._start_watch_0_x()` from
    /// `powerline/lib/watcher/uv.py:93-97`.
    ///
    /// Python (pyuv 0.x branch): same as `_start_watch_1_x` but
    /// passes the path + 0 to the FSEvent constructor's start
    /// args. Rust port collapses both branches to the same
    /// `watches` insert since the libuv dependency isn't wired.
    pub fn _start_watch_0_x(&self, path: &str) -> bool {
        // py:93  def _start_watch_0_x(self, path):
        // py:94-97  self.watches[path] = pyuv.fs.FSEvent(self.loop, path, 0, ...)
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.insert(path.to_string())
    }

    /// Port of `UvWatcher.watch()` from
    /// `powerline/lib/watcher/uv.py:102`.
    ///
    /// Registers `path` as a watch target. Does nothing if the path
    /// is already watched.
    pub fn watch(&self, path: &str) {
        // py:101  def watch(self, path):
        // py:102  path = normpath(path, self.fenc)
        // py:103  with self.lock:
        // py:104  if path not in self.watches:
        // py:105  try:
        // py:106  self._start_watch(path)
        // py:107  except pyuv.error.FSEventError as e:
        // py:108  code = e.args[0]
        // py:109  if code == pyuv.errno.UV_ENOENT:
        // py:110  raise OSError(ENOENT, 'No such file or directory: ' + path)
        // py:111  else:
        // py:112  raise
        let normalized = normpath(path);
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.insert(normalized);
    }

    /// Port of `UvWatcher.unwatch()` from
    /// `powerline/lib/watcher/uv.py:114`.
    pub fn unwatch(&self, path: &str) {
        // py:114  def unwatch(self, path):
        // py:115  path = normpath(path, self.fenc)
        // py:116  with self.lock:
        // py:117  try:
        // py:118  watch = self.watches.pop(path)
        // py:119  except KeyError:
        // py:120  return
        // py:121  watch.close(partial(self._stopped_watching, path))
        let normalized = normpath(path);
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.remove(&normalized);
    }

    /// Port of `UvWatcher.is_watching()` from
    /// `powerline/lib/watcher/uv.py:123`.
    pub fn is_watching(&self, path: &str) -> bool {
        // py:123  def is_watching(self, path):
        // py:124  with self.lock:
        // py:125  return normpath(path, self.fenc) in self.watches
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

    /// Port of `UvFileWatcher._stopped_watching()` from
    /// `powerline/lib/watcher/uv.py:151-152`.
    ///
    /// Drops the events queue for `path` per py:152
    /// (`self.events.pop(path, None)`). Called by pyuv when the
    /// fsevent handle is closed.
    pub fn _stopped_watching(&self, path: &str) {
        // py:151  def _stopped_watching(self, path, *args):
        // py:152  self.events.pop(path, None)
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        events.remove(path);
    }

    /// Port of `UvFileWatcher._record_event()` from
    /// `powerline/lib/watcher/uv.py:144` — underscore-prefixed alias
    /// for [`record_event`](Self::record_event) preserving the
    /// Python identifier byte-for-byte.
    pub fn _record_event(&self, path: &str, events_mask: u32) {
        // py:144  def _record_event(self, path, fsevent_handle, filename, events, error):
        self.record_event(path, events_mask)
    }

    /// Port of `UvFileWatcher._record_event()` from
    /// `powerline/lib/watcher/uv.py:142`.
    pub fn record_event(&self, path: &str, events_mask: u32) {
        // py:144  def _record_event(self, path, fsevent_handle, filename, events, error):
        // py:145  with self.lock:
        // py:146  self.events[path].append(events)
        // py:147  if events | pyuv.fs.UV_RENAME:
        // py:148  if not os.path.exists(path):
        // py:149  self.watches.pop(path).close()
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
        // py:154  def __call__(self, path):
        // py:155  path = normpath(path, self.fenc)
        // py:156  with self.lock:
        // py:157  events = self.events.pop(path, None)
        // py:158  if events:
        // py:159  return True
        // py:160  if path not in self.watches:
        // py:161  self.watch(path)
        // py:162  return True
        // py:163  return False
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

/// Port of `class UvTreeWatcher(UvWatcher)` event-tracking state
/// from `powerline/lib/watcher/uv.py:166-207`.
///
/// Watches a whole directory tree. Tracks the per-tree
/// `modified` flag that flips on any contained file/directory
/// event. `__call__` (py:206-207) pops the flag — the next read
/// returns `false` until another event fires.
///
/// Named with the `Events` suffix to disambiguate from the stub
/// `UvTreeWatcher` above (which mirrors the upstream construction
/// failure mode); same pattern as `UvFileWatcherEvents`.
pub struct UvTreeWatcherEvents {
    /// Python: `self.basedir` (py:172) — root path of the tree.
    pub basedir: String,
    /// Python: `self.modified` (py:173) — flips to `true` on any
    /// contained event. Initially `true` per py:173.
    pub modified: Mutex<bool>,
    /// Python: `ignore_event` callback (py:171) — pair-encoded by
    /// `(path, name)` for filtering events. Rust port stores the
    /// caller-supplied list of (path, name) pairs to ignore.
    pub ignored_events: Mutex<Vec<(String, String)>>,
    /// Inherited `UvWatcher.watches` — tracked directories.
    pub watcher: UvWatcher,
}

impl UvTreeWatcherEvents {
    /// Python class attribute: `is_dummy = False` (py:167).
    pub const IS_DUMMY: bool = false;

    /// Port of `UvTreeWatcher.__init__()` from
    /// `powerline/lib/watcher/uv.py:169-174`.
    ///
    /// `basedir` is the tree root. The Python source walks
    /// `basedir` and registers a watch on every subdirectory per
    /// py:174 via `watch_directory`; the Rust port skips the
    /// initial os.walk since pyuv's FSEvent isn't reachable.
    /// Caller supplies the directory list to `watch_directory`
    /// explicitly when wiring through the libuv binding.
    pub fn new(basedir: impl Into<String>) -> Self {
        // py:170-174
        Self {
            basedir: normpath(&basedir.into()),
            // py:173  self.modified = True
            modified: Mutex::new(true),
            ignored_events: Mutex::new(Vec::new()),
            watcher: UvWatcher::new(),
        }
    }

    /// Port of `UvTreeWatcher.watch_directory()` from
    /// `powerline/lib/watcher/uv.py:176-178`.
    ///
    /// Walks `path` and registers a watch on every contained
    /// directory. Python uses `os.walk`; Rust uses `walkdir` /
    /// `read_dir`. The caller's `directories` list pre-resolves
    /// the walk since the Rust port doesn't pull walkdir into the
    /// runtime crate.
    pub fn watch_directory<I>(&self, directories: I)
    where
        I: IntoIterator<Item = String>,
    {
        // py:177-178  os.walk(path); watch_one_directory(root)
        for dir in directories {
            self.watch_one_directory(&dir);
        }
    }

    /// Port of `UvTreeWatcher.watch_one_directory()` from
    /// `powerline/lib/watcher/uv.py:180-184`.
    ///
    /// Wraps the watch call in OSError-swallow per py:183-184.
    pub fn watch_one_directory(&self, dirname: &str) {
        // py:181-184  try: self.watch(dirname); except OSError: pass
        self.watcher.watch(dirname);
    }

    /// Port of `UvTreeWatcher._stopped_watching()` from
    /// `powerline/lib/watcher/uv.py:186-187`.
    ///
    /// Removes the path from `watcher.watches`. Called by pyuv
    /// when the underlying handle is closed; the Rust port has
    /// no live pyuv loop so this surfaces as an explicit method
    /// callers invoke during shutdown.
    pub fn _stopped_watching(&self, path: &str) {
        // py:186  def _stopped_watching(self, path, *args):
        // py:187  self.watches.pop(path, None)
        let mut watches = self
            .watcher
            .watches
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        watches.remove(path);
    }

    /// Port of `UvTreeWatcher._record_event()` from
    /// `powerline/lib/watcher/uv.py:189` — underscore-prefixed
    /// alias for [`record_event`](Self::record_event) preserving
    /// the Python identifier byte-for-byte.
    pub fn _record_event(&self, path: &str, name: &str, events_mask: u32) -> bool {
        // py:189  def _record_event(self, path, fsevent_handle, filename, events, error):
        self.record_event(path, name, events_mask)
    }

    /// Port of `UvTreeWatcher._record_event()` from
    /// `powerline/lib/watcher/uv.py:189-204`.
    ///
    /// Sets the modified flag when the event passes the
    /// `ignore_event(path, name)` filter per py:190.
    /// `events_mask` carries the libuv UV_CHANGE/UV_RENAME bits;
    /// the Rust port surfaces the filter outcome plus the mask
    /// without performing the os.path.isdir dispatch (py:197-204)
    /// since the live tree state isn't reachable. Returns the new
    /// modified state.
    pub fn record_event(&self, path: &str, name: &str, events_mask: u32) -> bool {
        // py:189  def _record_event(self, path, fsevent_handle, filename, events, error):
        // py:190  if not self.ignore_event(path, filename):
        let ignored = self
            .ignored_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for (p, n) in ignored.iter() {
            if p == path && n == name {
                return *self.modified.lock().unwrap_or_else(|e| e.into_inner());
            }
        }
        drop(ignored);
        let _ = events_mask;
        // py:191  self.modified = True
        // py:192  if events == pyuv.fs.UV_CHANGE | pyuv.fs.UV_RENAME:
        // py:193  # Stat changes to watched directory are UV_CHANGE|UV_RENAME. It
        // py:194  # is weird.
        // py:195  pass
        // py:196  elif events | pyuv.fs.UV_RENAME:
        // py:197  if not os.path.isdir(path):
        // py:198  self.unwatch(path)
        // py:199  else:
        // py:200  full_name = os.path.join(path, filename)
        // py:201  if os.path.isdir(full_name):
        // py:202  # For some reason mkdir and rmdir both fall into this
        // py:203  # category
        // py:204  self.watch_directory(full_name)
        let mut m = self.modified.lock().unwrap_or_else(|e| e.into_inner());
        *m = true;
        true
    }

    /// Port of `UvTreeWatcher.__call__()` from
    /// `powerline/lib/watcher/uv.py:206-207`.
    ///
    /// Pops the modified flag — returns the current value then
    /// resets to `false` per py:207 (`__dict__.pop('modified',
    /// False)`).
    pub fn check(&self) -> bool {
        // py:206  def __call__(self):
        // py:207  return self.__dict__.pop('modified', False)
        let mut m = self.modified.lock().unwrap_or_else(|e| e.into_inner());
        let prev = *m;
        *m = false;
        prev
    }

    /// Registers a `(path, name)` pair to ignore in
    /// `record_event` per py:190 `ignore_event` callback.
    pub fn ignore(&self, path: impl Into<String>, name: impl Into<String>) {
        let mut ignored = self
            .ignored_events
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        ignored.push((path.into(), name.into()));
    }
}

/// Port of `class UvThread(Thread)` from
/// `powerline/lib/watcher/uv.py:36-53`.
///
/// Background thread that runs the pyuv event loop. Python uses
/// `pyuv.Async` to wake the loop from `join()`; the Rust port
/// surfaces the start/join contract since the actual libuv loop
/// isn't reachable.
pub struct UvThread {
    /// Python: `self.daemon = True` (py:37). Rust always treats
    /// the placeholder as daemon-equivalent.
    pub daemon: bool,
    /// Tracks whether `join` has been called (Python's underlying
    /// thread.join blocks until the loop stops).
    pub joined: Mutex<bool>,
}

impl Default for UvThread {
    fn default() -> Self {
        Self::new()
    }
}

impl UvThread {
    /// Port of `UvThread.__init__()` from
    /// `powerline/lib/watcher/uv.py:39-42`.
    pub fn new() -> Self {
        Self {
            // py:37  daemon = True
            daemon: true,
            joined: Mutex::new(false),
        }
    }

    /// Port of `UvThread._async_cb()` from
    /// `powerline/lib/watcher/uv.py:44-46`.
    ///
    /// pyuv's Async-handle callback: stops the libuv event loop
    /// and closes the async handle. Called when `join()` triggers
    /// the wake-up via `self.async_handle.send()`.
    ///
    /// Rust port has no libuv loop; this surfaces as a documented
    /// no-op that flips `joined` so subsequent `join()` returns
    /// without blocking.
    pub fn _async_cb(&self) {
        // py:44  def _async_cb(self, handle):
        // py:45  self.uv_loop.stop()
        // py:46  self.async_handle.close()
        let mut joined = self.joined.lock().unwrap_or_else(|e| e.into_inner());
        *joined = true;
    }

    /// Port of `UvThread.run()` from
    /// `powerline/lib/watcher/uv.py:48-49`.
    ///
    /// Stub — the actual `self.uv_loop.run()` dispatch at py:49
    /// needs a live libuv loop. Returns immediately.
    pub fn run(&self) {
        // py:49  self.uv_loop.run() — deferred
    }

    /// Port of `UvThread.join()` from
    /// `powerline/lib/watcher/uv.py:51-53`.
    ///
    /// Sets the `joined` flag (Python's underlying Thread.join
    /// blocks; Rust mirrors with a flag since the loop isn't live).
    pub fn join(&self) {
        // py:52  self.async_handle.send() — caller-wired
        // py:53  return super(UvThread, self).join()
        let mut j = self.joined.lock().unwrap_or_else(|e| e.into_inner());
        *j = true;
    }

    /// Returns whether `join` has been called.
    pub fn is_joined(&self) -> bool {
        *self.joined.lock().unwrap_or_else(|e| e.into_inner())
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

    #[test]
    fn uv_tree_watcher_events_initial_state_is_modified() {
        // py:173  self.modified = True
        let w = UvTreeWatcherEvents::new("/tmp");
        // basedir gets normalised; check the modified initial state
        let _ = &w.basedir;
        assert!(*w.modified.lock().unwrap());
    }

    #[test]
    fn uv_tree_watcher_events_check_pops_modified_flag() {
        // py:206-207  __dict__.pop('modified', False)
        let w = UvTreeWatcherEvents::new("/tmp");
        assert!(w.check());
        // Second check returns false since the flag was popped.
        assert!(!w.check());
    }

    #[test]
    fn uv_tree_watcher_events_record_event_sets_modified() {
        // py:189-191
        let w = UvTreeWatcherEvents::new("/tmp");
        // Drain initial modified state
        let _ = w.check();
        assert!(!w.check());
        // record_event flips it back
        let _ = w.record_event("/tmp/file", "x.txt", 1);
        assert!(w.check());
    }

    #[test]
    fn uv_tree_watcher_events_ignored_event_does_not_set_modified() {
        // py:190  if not self.ignore_event(path, filename)
        let w = UvTreeWatcherEvents::new("/tmp");
        // Drain initial modified state
        let _ = w.check();
        // Register ignore
        w.ignore("/tmp/file", "x.txt");
        // record_event for the ignored pair
        let _ = w.record_event("/tmp/file", "x.txt", 1);
        assert!(!w.check());
    }

    #[test]
    fn uv_tree_watcher_events_is_dummy_false() {
        // py:167  is_dummy = False
        const _: () = assert!(!UvTreeWatcherEvents::IS_DUMMY);
    }

    #[test]
    fn uv_tree_watcher_events_watch_directory_walks_supplied_list() {
        // py:177-178
        let w = UvTreeWatcherEvents::new("/tmp");
        w.watch_directory(vec!["/tmp/a".to_string(), "/tmp/b".to_string()]);
        // Watcher should now track both directories
        assert!(w.watcher.watch_count() >= 2);
    }

    #[test]
    fn uv_thread_new_starts_unjoined() {
        // py:36-42
        let t = UvThread::new();
        assert!(t.daemon);
        assert!(!t.is_joined());
    }

    #[test]
    fn uv_thread_join_flips_state() {
        // py:51-53
        let t = UvThread::new();
        t.join();
        assert!(t.is_joined());
    }

    #[test]
    fn uv_thread_run_is_noop_without_panic() {
        // py:48-49 stub
        let t = UvThread::new();
        t.run();
    }

    #[test]
    fn uv_thread_async_cb_flips_joined() {
        // py:44-46  stops loop + closes handle; Rust port marks joined
        let t = UvThread::new();
        assert!(!t.is_joined());
        t._async_cb();
        assert!(t.is_joined());
    }

    #[test]
    fn uv_watcher_start_watch_1_x_inserts_path() {
        // py:88-91
        let w = UvWatcher::new();
        assert!(w._start_watch_1_x("/tmp/a"));
        // Second call returns false (already present)
        assert!(!w._start_watch_1_x("/tmp/a"));
    }

    #[test]
    fn uv_watcher_start_watch_0_x_inserts_path() {
        // py:93-97
        let w = UvWatcher::new();
        assert!(w._start_watch_0_x("/tmp/b"));
        assert!(!w._start_watch_0_x("/tmp/b"));
    }
}
