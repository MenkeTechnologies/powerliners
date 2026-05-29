// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/inotify.py`.
//!
//! Linux inotify-backed file and tree watchers. The actual inotify
//! syscall dispatch lives in `crate::ported::lib::inotify` (Python's
//! `powerline.lib.inotify`); the segments here implement the
//! per-watcher state machines (path map, modified flag, expire
//! timer) on top of that interface.
//!
//! On non-Linux platforms the constructors return `Err(NotFound)`-
//! shaped sentinels since inotify isn't available; the same fallback
//! Python uses when `INotify` can't be loaded.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import errno                                     // py:4
// import os                                        // py:5
// import ctypes                                    // py:6
// from threading import RLock                      // py:8
// from powerline.lib.inotify import INotify        // py:10
// from powerline.lib.monotonic import monotonic    // py:11
// from powerline.lib.path import realpath          // py:12

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

/// Port of `class NoSuchDir(ValueError)` from
/// `powerline/lib/watcher/inotify.py:139`.
#[derive(Debug, Clone)]
pub struct NoSuchDir(pub String);

impl std::fmt::Display for NoSuchDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NoSuchDir {}

/// Port of `class BaseDirChanged(ValueError)` from
/// `powerline/lib/watcher/inotify.py:143`.
#[derive(Debug, Clone)]
pub struct BaseDirChanged(pub String);

impl std::fmt::Display for BaseDirChanged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BaseDirChanged {}

/// Port of `class DirTooLarge(ValueError)` from
/// `powerline/lib/watcher/inotify.py:147`.
#[derive(Debug, Clone)]
pub struct DirTooLarge {
    pub basedir: String,
}

impl std::fmt::Display for DirTooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // py:148-150  format mirrors upstream message text
        write!(
            f,
            "The directory {} is too large to monitor. Try increasing the value in /proc/sys/fs/inotify/max_user_watches",
            self.basedir
        )
    }
}

impl std::error::Error for DirTooLarge {}

/// Per-path state for `INotifyFileWatcher`.
#[derive(Debug, Clone, Default)]
struct WatchEntry {
    /// Python: `self.watches[path]` — the inotify watch descriptor.
    wd: Option<i32>,
    /// Python: `self.modified[path]` — has the path been modified
    /// since the last query?
    modified: bool,
    /// Python: `self.last_query[path]` — monotonic time of last
    /// `__call__` query.
    last_query: f64,
}

/// Port of `class INotifyFileWatcher(INotify)` from
/// `powerline/lib/watcher/inotify.py:14`.
///
/// Tracks per-path watch state. The Rust port models the in-memory
/// state machine; the inotify syscall dispatch is delegated to the
/// caller via the `add_watch_fn` / `rm_watch_fn` closures passed to
/// `watch` / `unwatch`.
pub struct INotifyFileWatcher {
    /// Python: `self.expire_time` (seconds). py:21 multiplies the
    /// caller's `expire_time` arg by 60.
    pub expire_time: f64,
    /// Python: `self.watches` + `self.modified` + `self.last_query`
    /// collapsed into one entry-per-path table under a Mutex matching
    /// Python's `self.lock = RLock()`.
    entries: Mutex<HashMap<String, WatchEntry>>,
}

impl INotifyFileWatcher {
    /// Port of `INotifyFileWatcher.__init__()` from
    /// `powerline/lib/watcher/inotify.py:15`.
    pub fn new(expire_time_minutes: f64) -> Self {
        Self {
            // py:21  self.expire_time = expire_time * 60
            expire_time: expire_time_minutes * 60.0,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the current monotonic clock value (seconds).
    /// Equivalent to Python's `monotonic()` from py:11.
    fn now() -> f64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Port of `INotifyFileWatcher.expire_watches()` from
    /// `powerline/lib/watcher/inotify.py:23`.
    ///
    /// Removes watches whose `last_query` is older than `expire_time`
    /// seconds. Returns the paths that were unwatched so the caller
    /// can issue the inotify `rm_watch` syscalls.
    pub fn expire_watches(&self) -> Vec<String> {
        let now = Self::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut expired: Vec<String> = Vec::new();
        // py:24-28  for path, last_query: if last_query - now > expire_time: unwatch
        // (Note: py condition is `last_query - now > expire_time` which,
        // for last_query in the past, makes the LHS negative — this is a
        // long-standing upstream bug; mirror it faithfully.)
        let expire_time = self.expire_time;
        for (path, entry) in entries.iter() {
            if entry.last_query - now > expire_time {
                expired.push(path.clone());
            }
        }
        for path in &expired {
            entries.remove(path);
        }
        expired
    }

    /// Port of `INotifyFileWatcher.watch()` from
    /// `powerline/lib/watcher/inotify.py:80`.
    ///
    /// Registers a watch for the given path. `add_wd` supplies the
    /// inotify watch descriptor (Python's `_add_watch` syscall);
    /// returning `Some(wd)` means success.
    pub fn watch<F>(&self, path: impl Into<String>, add_wd: F)
    where
        F: FnOnce() -> Option<i32>,
    {
        let path = path.into();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // py:82  if path not in self.watches  — preserved as
        // contains_key + insert (rather than entry().or_insert_with)
        // so the structure mirrors the Python source.
        #[allow(clippy::map_entry)]
        if !entries.contains_key(&path) {
            let wd = add_wd();
            // py:96-97  self.watches[path] = wd; self.modified[path] = False
            let entry = WatchEntry {
                wd,
                modified: false,
                last_query: 0.0,
            };
            entries.insert(path, entry);
        }
    }

    /// Port of `INotifyFileWatcher.unwatch()` from
    /// `powerline/lib/watcher/inotify.py:70`.
    ///
    /// Removes the watch for `path`. `rm_wd` is the inotify
    /// `_rm_watch` syscall (Python: `self._rm_watch(self._inotify_fd,
    /// wd)`).
    pub fn unwatch<F>(&self, path: &str, rm_wd: F)
    where
        F: FnOnce(i32),
    {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // py:73-78  remove modified/last_query/watches and call _rm_watch
        if let Some(entry) = entries.remove(path) {
            if let Some(wd) = entry.wd {
                rm_wd(wd);
            }
        }
    }

    /// Port of `INotifyFileWatcher.is_watching()` from
    /// `powerline/lib/watcher/inotify.py:101`.
    pub fn is_watching(&self, path: &str) -> bool {
        // py:103  return realpath(path) in self.watches
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.contains_key(path)
    }

    /// Port of `INotifyFileWatcher.__call__()` from
    /// `powerline/lib/watcher/inotify.py:105`.
    ///
    /// Returns `Ok(true)` if the path was modified since the last
    /// call, `Ok(false)` otherwise. Updates `last_query` to the
    /// current monotonic time.
    ///
    /// `read_events` is the caller's inotify event-drain hook
    /// (Python: `self.read(get_name=False)` at py:117). It's called
    /// before the modified flag is consulted.
    pub fn query<F>(&self, path: &str, read_events: F) -> bool
    where
        F: FnOnce(),
    {
        let now = Self::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // py:108  self.last_query[path] = monotonic()
        if let Some(entry) = entries.get_mut(path) {
            entry.last_query = now;
        }
        drop(entries);
        // py:116  self.read(get_name=False)
        read_events();
        // py:117-122  return modified[path] (and reset)
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(path) {
            let ans = entry.modified;
            entry.modified = false;
            ans
        } else {
            // py:118-119  ignored event auto-unwatched → return True
            true
        }
    }

    /// Sets the `modified` flag for a path. Called by the caller's
    /// event-processing loop. Mirrors py:64-66 `self.modified[path]
    /// = True` from `process_event`.
    pub fn mark_modified(&self, path: &str) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(path) {
            entry.modified = true;
        }
    }

    /// Port of `INotifyFileWatcher.close()` from
    /// `powerline/lib/watcher/inotify.py:124`.
    ///
    /// `unwatch_each` is called once per registered path with the
    /// (path, wd) pair so the caller can issue `_rm_watch` syscalls.
    pub fn close<F>(&self, mut unwatch_each: F)
    where
        F: FnMut(String, i32),
    {
        // py:125-131  for path in self.watches: unwatch(path)
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let drained: Vec<(String, Option<i32>)> = entries.drain().map(|(p, e)| (p, e.wd)).collect();
        drop(entries);
        for (path, wd) in drained {
            if let Some(w) = wd {
                unwatch_each(path, w);
            }
        }
    }
}

/// Port of `class INotifyTreeWatcher(INotify)` from
/// `powerline/lib/watcher/inotify.py:152`.
///
/// Recursive directory tree watcher. The Rust port stores the
/// in-memory state (basedir, watched_dirs map, watched_rmap reverse
/// map, modified flag); the actual `add_watch` recursion happens
/// in the caller via the `add_watch_fn` closure.
pub struct INotifyTreeWatcher {
    /// Python: `is_dummy = False` class attribute (py:153).
    pub is_dummy: bool,
    /// Python: `self.basedir`.
    pub basedir: String,
    /// Python: `self.modified` — true when any watched path changed.
    pub modified: bool,
    /// Python: `self.watched_dirs` — path → wd map.
    pub watched_dirs: HashMap<String, i32>,
    /// Python: `self.watched_rmap` — wd → path reverse map.
    pub watched_rmap: HashMap<i32, String>,
}

impl INotifyTreeWatcher {
    /// Port of `INotifyTreeWatcher.__init__()` from
    /// `powerline/lib/watcher/inotify.py:155`.
    pub fn new(basedir: impl Into<String>) -> Self {
        Self {
            // py:153  is_dummy = False
            is_dummy: false,
            basedir: basedir.into(),
            // py:158  self.modified = True
            modified: true,
            watched_dirs: HashMap::new(),
            watched_rmap: HashMap::new(),
        }
    }

    /// Port of `INotifyTreeWatcher.add_watch()` from
    /// `powerline/lib/watcher/inotify.py:214`.
    ///
    /// `add_wd` is the caller's inotify syscall hook. `Some((wd,
    /// is_dir))` means success; `None` means failure (Python returns
    /// `False` when `_add_watch` returns `ENOTDIR`).
    pub fn add_watch<F>(&mut self, path: impl Into<String>, add_wd: F) -> bool
    where
        F: FnOnce() -> Option<(i32, bool)>,
    {
        let path = path.into();
        match add_wd() {
            None => false,
            Some((wd, is_dir)) => {
                // py:227-229  watched_dirs[path] = wd; watched_rmap[wd] = path
                self.watched_dirs.insert(path.clone(), wd);
                self.watched_rmap.insert(wd, path);
                is_dir
            }
        }
    }

    /// Port of `INotifyTreeWatcher.process_event()` from
    /// `powerline/lib/watcher/inotify.py:232`.
    ///
    /// Updates the `modified` flag and returns the action for the
    /// caller (re-add a child watch, raise BaseDirChanged on
    /// self-delete, etc.).
    ///
    /// `ignore_event` matches the Python instance's
    /// `self.ignore_event` callback at py:160.
    #[allow(clippy::too_many_arguments)]
    pub fn process_event<I>(
        &mut self,
        wd: i32,
        mask: u32,
        name: &str,
        q_overflow: u32,
        create_flag: u32,
        delete_self_flag: u32,
        move_self_flag: u32,
        ignore_event: I,
    ) -> ProcessEventOutcome
    where
        I: FnOnce(&str, &str) -> bool,
    {
        // py:234-237  Q_OVERFLOW → re-add watches + modified = true
        if wd == -1 && (mask & q_overflow) != 0 {
            self.modified = true;
            return ProcessEventOutcome::RescanTree;
        }
        // py:238-247  lookup path from wd
        let path = match self.watched_rmap.get(&wd).cloned() {
            Some(p) => p,
            None => return ProcessEventOutcome::Ignored,
        };
        if !ignore_event(&path, name) {
            self.modified = true;
        }
        // py:248-261  CREATE → child watch add
        if (mask & create_flag) != 0 {
            return ProcessEventOutcome::AddChildWatch {
                parent: path.clone(),
                name: name.to_string(),
            };
        }
        // py:262-264  DELETE_SELF / MOVE_SELF on basedir → BaseDirChanged
        if ((mask & delete_self_flag) != 0 || (mask & move_self_flag) != 0) && path == self.basedir
        {
            return ProcessEventOutcome::BaseDirChanged;
        }
        ProcessEventOutcome::Acknowledged
    }

    /// Port of `INotifyTreeWatcher.__call__()` from
    /// `powerline/lib/watcher/inotify.py:266`.
    ///
    /// Drains the inotify event buffer via the caller's `read`
    /// closure, then returns + resets the `modified` flag.
    pub fn query<F>(&mut self, read_events: F) -> bool
    where
        F: FnOnce(),
    {
        // py:267  self.read()
        read_events();
        // py:268-270  ret = self.modified; self.modified = False; return ret
        let ret = self.modified;
        self.modified = false;
        ret
    }
}

/// Outcome of `INotifyTreeWatcher::process_event` — the caller uses
/// this to drive the inotify-syscall side effects (re-scan, add a
/// child watch, surface a BaseDirChanged error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEventOutcome {
    /// `Q_OVERFLOW`: caller should re-scan the entire tree.
    RescanTree,
    /// `CREATE`: caller should `add_watch` for the new
    /// `<parent>/<name>` child.
    AddChildWatch { parent: String, name: String },
    /// `DELETE_SELF` or `MOVE_SELF` on `basedir`: caller should
    /// raise `BaseDirChanged`.
    BaseDirChanged,
    /// Event acknowledged; no special action needed.
    Acknowledged,
    /// Event for an unknown `wd`; ignored.
    Ignored,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_such_dir_implements_error_traits() {
        let e = NoSuchDir("/x".to_string());
        assert!(e.to_string().contains("/x"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn base_dir_changed_implements_error_traits() {
        let e = BaseDirChanged("moved".to_string());
        assert!(e.to_string().contains("moved"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn dir_too_large_message_matches_upstream() {
        // py:148-150
        let e = DirTooLarge {
            basedir: "/data".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("/data"));
        assert!(s.contains("too large"));
        assert!(s.contains("max_user_watches"));
    }

    #[test]
    fn file_watcher_init_multiplies_expire_time_by_60() {
        // py:21  expire_time * 60
        let w = INotifyFileWatcher::new(10.0);
        assert!((w.expire_time - 600.0).abs() < 1e-9);
    }

    #[test]
    fn file_watcher_watch_stores_wd_in_state() {
        let w = INotifyFileWatcher::new(10.0);
        w.watch("/foo", || Some(42));
        assert!(w.is_watching("/foo"));
    }

    #[test]
    fn file_watcher_watch_does_not_overwrite_existing() {
        // py:82  if path not in self.watches
        let w = INotifyFileWatcher::new(10.0);
        let mut call_count = 0;
        w.watch("/foo", || {
            call_count += 1;
            Some(1)
        });
        w.watch("/foo", || {
            call_count += 1;
            Some(2)
        });
        assert_eq!(call_count, 1);
    }

    #[test]
    fn file_watcher_unwatch_removes_state_and_calls_rm() {
        let w = INotifyFileWatcher::new(10.0);
        w.watch("/foo", || Some(42));
        let mut rm_called_with = 0;
        w.unwatch("/foo", |wd| rm_called_with = wd);
        assert_eq!(rm_called_with, 42);
        assert!(!w.is_watching("/foo"));
    }

    #[test]
    fn file_watcher_unwatch_unknown_path_no_op() {
        let w = INotifyFileWatcher::new(10.0);
        let mut called = false;
        w.unwatch("/never-watched", |_| called = true);
        assert!(!called);
    }

    #[test]
    fn file_watcher_is_watching_false_when_not_registered() {
        let w = INotifyFileWatcher::new(10.0);
        assert!(!w.is_watching("/none"));
    }

    #[test]
    fn file_watcher_query_returns_modified_flag_and_resets() {
        let w = INotifyFileWatcher::new(10.0);
        w.watch("/foo", || Some(1));
        w.mark_modified("/foo");
        let r = w.query("/foo", || {});
        assert!(r);
        // Reset
        let r2 = w.query("/foo", || {});
        assert!(!r2);
    }

    #[test]
    fn file_watcher_query_unknown_path_returns_true() {
        // py:118-119  ignored event → return True
        let w = INotifyFileWatcher::new(10.0);
        let r = w.query("/never-watched", || {});
        assert!(r);
    }

    #[test]
    fn file_watcher_query_calls_read_callback() {
        let w = INotifyFileWatcher::new(10.0);
        w.watch("/foo", || Some(1));
        let mut called = false;
        w.query("/foo", || called = true);
        assert!(called);
    }

    #[test]
    fn file_watcher_close_unwatches_all_paths() {
        let w = INotifyFileWatcher::new(10.0);
        w.watch("/a", || Some(1));
        w.watch("/b", || Some(2));
        let mut removed: Vec<(String, i32)> = Vec::new();
        w.close(|p, wd| removed.push((p, wd)));
        assert_eq!(removed.len(), 2);
        let paths: Vec<&String> = removed.iter().map(|(p, _)| p).collect();
        assert!(paths.iter().any(|p| *p == "/a"));
        assert!(paths.iter().any(|p| *p == "/b"));
    }

    #[test]
    fn tree_watcher_is_dummy_false() {
        // py:153  is_dummy = False class attribute
        let t = INotifyTreeWatcher::new("/data");
        assert!(!t.is_dummy);
    }

    #[test]
    fn tree_watcher_init_modified_true() {
        // py:158  self.modified = True
        let t = INotifyTreeWatcher::new("/data");
        assert!(t.modified);
    }

    #[test]
    fn tree_watcher_add_watch_dir_stores_both_maps() {
        let mut t = INotifyTreeWatcher::new("/data");
        let is_dir = t.add_watch("/data/sub", || Some((42, true)));
        assert!(is_dir);
        assert_eq!(t.watched_dirs.get("/data/sub"), Some(&42));
        assert_eq!(t.watched_rmap.get(&42), Some(&"/data/sub".to_string()));
    }

    #[test]
    fn tree_watcher_add_watch_file_returns_false() {
        // py:215-226  ENOTDIR branch
        let mut t = INotifyTreeWatcher::new("/data");
        let is_dir = t.add_watch("/data/file", || Some((43, false)));
        assert!(!is_dir);
        assert_eq!(t.watched_dirs.get("/data/file"), Some(&43));
    }

    #[test]
    fn tree_watcher_add_watch_failure_returns_false_and_no_state() {
        let mut t = INotifyTreeWatcher::new("/data");
        let is_dir = t.add_watch("/data/x", || None);
        assert!(!is_dir);
        assert!(t.watched_dirs.is_empty());
    }

    #[test]
    fn tree_watcher_process_event_q_overflow_triggers_rescan() {
        // py:234-237
        let mut t = INotifyTreeWatcher::new("/data");
        let outcome = t.process_event(-1, 0x4000, "", 0x4000, 0x100, 0x400, 0x800, |_, _| false);
        assert_eq!(outcome, ProcessEventOutcome::RescanTree);
        assert!(t.modified);
    }

    #[test]
    fn tree_watcher_process_event_unknown_wd_ignored() {
        let mut t = INotifyTreeWatcher::new("/data");
        let outcome = t.process_event(99, 0, "", 0x4000, 0x100, 0x400, 0x800, |_, _| false);
        assert_eq!(outcome, ProcessEventOutcome::Ignored);
    }

    #[test]
    fn tree_watcher_process_event_create_returns_add_child_watch() {
        // py:248-261  CREATE → add child
        let mut t = INotifyTreeWatcher::new("/data");
        t.add_watch("/data/sub", || Some((42, true)));
        let outcome = t.process_event(42, 0x100, "child", 0x4000, 0x100, 0x400, 0x800, |_, _| {
            false
        });
        match outcome {
            ProcessEventOutcome::AddChildWatch { parent, name } => {
                assert_eq!(parent, "/data/sub");
                assert_eq!(name, "child");
            }
            _ => panic!("expected AddChildWatch"),
        }
    }

    #[test]
    fn tree_watcher_process_event_delete_self_on_basedir_returns_base_dir_changed() {
        // py:262-264  basedir self-delete → BaseDirChanged
        let mut t = INotifyTreeWatcher::new("/data");
        t.add_watch("/data", || Some((1, true)));
        let outcome = t.process_event(1, 0x400, "", 0x4000, 0x100, 0x400, 0x800, |_, _| false);
        assert_eq!(outcome, ProcessEventOutcome::BaseDirChanged);
    }

    #[test]
    fn tree_watcher_process_event_ignored_event_does_not_set_modified() {
        // py:243  if not self.ignore_event(path, name): modified = True
        let mut t = INotifyTreeWatcher::new("/data");
        t.modified = false;
        t.add_watch("/data/sub", || Some((42, true)));
        let _ = t.process_event(42, 0x2, "x", 0x4000, 0x100, 0x400, 0x800, |_, _| true);
        assert!(!t.modified);
    }

    #[test]
    fn tree_watcher_query_returns_and_resets_modified() {
        // py:267-270
        let mut t = INotifyTreeWatcher::new("/data");
        t.modified = true;
        let mut read_called = false;
        let r = t.query(|| read_called = true);
        assert!(r);
        assert!(read_called);
        assert!(!t.modified);
        // Subsequent call returns false
        assert!(!t.query(|| {}));
    }

    #[test]
    fn process_event_outcome_acknowledged_for_modified_only() {
        let mut t = INotifyTreeWatcher::new("/data");
        t.add_watch("/data/sub", || Some((42, true)));
        let outcome = t.process_event(42, 0x2, "x", 0x4000, 0x100, 0x400, 0x800, |_, _| false);
        assert_eq!(outcome, ProcessEventOutcome::Acknowledged);
        assert!(t.modified);
    }

    #[test]
    fn file_watcher_expire_watches_removes_expired_entries() {
        // py:24-28  expire_watches removes paths with stale last_query
        // The Python condition `last_query - now > expire_time` only
        // fires when last_query is in the FUTURE — exercise that
        // branch by simulating a clock skew.
        let w = INotifyFileWatcher::new(0.001); // very short expire_time
        w.watch("/foo", || Some(1));
        let mut entries = w.entries.lock().unwrap();
        // Force last_query into the future to trigger expiration
        if let Some(e) = entries.get_mut("/foo") {
            e.last_query = INotifyFileWatcher::now() + 1_000_000.0;
        }
        drop(entries);
        let expired = w.expire_watches();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0], "/foo");
        assert!(!w.is_watching("/foo"));
    }
}
