// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/tree.py`.
//!
//! Recursive directory tree watcher with backend dispatch
//! (inotify / pyuv / dummy / stat / auto). Used by VCS segments to
//! invalidate caches on any change inside a working tree.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// from powerline.lib.monotonic import monotonic    // py:6
// from powerline.lib.inotify import INotifyError   // py:7
// from powerline.lib.path import realpath          // py:8
// from powerline.lib.watcher.inotify import INotifyTreeWatcher, DirTooLarge, NoSuchDir, BaseDirChanged  // py:9
// from powerline.lib.watcher.uv import UvTreeWatcher, UvNotFound                                          // py:10

use crate::ported::lib::monotonic::monotonic;
use crate::ported::lib::path::realpath;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Port of `class DummyTreeWatcher` from
/// `powerline/lib/watcher/tree.py:13`.
///
/// Always-false watcher used as a fallback when no real backend is
/// available (e.g. macOS with no inotify, no libuv).
pub struct DummyTreeWatcher {
    pub basedir: PathBuf,
    /// Python class attribute: `is_dummy = True` — py:14
    pub is_dummy: bool,
}

impl DummyTreeWatcher {
    /// Port of `DummyTreeWatcher.__init__()` from
    /// `powerline/lib/watcher/tree.py:16`.
    pub fn new<P: AsRef<Path>>(basedir: P) -> Self {
        Self {
            basedir: realpath(basedir), // py:17
            is_dummy: true,             // py:14
        }
    }

    /// Port of `DummyTreeWatcher.__call__()` from
    /// `powerline/lib/watcher/tree.py:19`.
    ///
    /// Always returns false (never reports a change).
    pub fn check(&self) -> bool {
        false // py:20
    }
}

/// Port of `class TreeWatcher` from
/// `powerline/lib/watcher/tree.py:23`.
///
/// Backend-dispatching tree watcher.
pub struct TreeWatcher {
    /// Python: `self.watches` — path → DummyTreeWatcher for now.
    pub watches: Mutex<HashMap<PathBuf, DummyTreeWatcher>>,
    /// Python: `self.last_query_times` — path → monotonic() at last call.
    pub last_query_times: Mutex<HashMap<PathBuf, f64>>,
    /// Python: `self.expire_time` (in seconds; upstream multiplies by 60).
    pub expire_time: f64,
    /// Python: `self.watcher_type` — `inotify` / `uv` / `dummy` / `stat` / `auto`.
    pub watcher_type: String,
}

impl TreeWatcher {
    /// Port of `TreeWatcher.__init__()` from
    /// `powerline/lib/watcher/tree.py:24`.
    pub fn new(watcher_type: impl Into<String>, expire_time_min: f64) -> Self {
        Self {
            watches: Mutex::new(HashMap::new()),          // py:25
            last_query_times: Mutex::new(HashMap::new()), // py:26
            expire_time: expire_time_min * 60.0,          // py:27  expire_time * 60
            watcher_type: watcher_type.into(),            // py:29
        }
    }

    /// Port of `TreeWatcher.get_watcher()` from
    /// `powerline/lib/watcher/tree.py:31`.
    ///
    /// Backend dispatch. Until inotify/uv are wired, every branch
    /// falls back to `DummyTreeWatcher` — the safe never-reports-change
    /// behaviour for which VCS segments will just re-query on every
    /// render.
    pub fn get_watcher<P: AsRef<Path>>(&self, path: P) -> Result<DummyTreeWatcher, String> {
        match self.watcher_type.as_str() {
            // py:31  def get_watcher(self, path, ignore_event):
            // py:32  if self.watcher_type == 'inotify':
            // py:33  return INotifyTreeWatcher(path, ignore_event=ignore_event)
            "inotify" => Ok(DummyTreeWatcher::new(path)),
            // py:34  if self.watcher_type == 'uv':
            // py:35  return UvTreeWatcher(path, ignore_event=ignore_event)
            "uv" => Ok(DummyTreeWatcher::new(path)),
            // py:36  if self.watcher_type == 'dummy':
            // py:37  return DummyTreeWatcher(path)
            "dummy" => Ok(DummyTreeWatcher::new(path)),
            // py:38  # FIXME
            // py:39  if self.watcher_type == 'stat':
            // py:40  return DummyTreeWatcher(path)
            "stat" => Ok(DummyTreeWatcher::new(path)),
            // py:41  if self.watcher_type == 'auto':
            // py:42  if sys.platform.startswith('linux'):
            // py:43  try:
            // py:44  return INotifyTreeWatcher(path, ignore_event=ignore_event)
            // py:45  except (INotifyError, DirTooLarge) as e:
            // py:46  if not isinstance(e, INotifyError):
            // py:47  self.pl.warn('Failed to watch path: {0} with error: {1}'.format(path, e))
            // py:48  try:
            // py:49  return UvTreeWatcher(path, ignore_event=ignore_event)
            // py:50  except UvNotFound:
            // py:51  pass
            // py:52  return DummyTreeWatcher(path)
            "auto" => Ok(DummyTreeWatcher::new(path)),
            // py:53  else:
            // py:54  raise ValueError('Unknown watcher type: {0}'.format(self.watcher_type))
            other => Err(format!("Unknown watcher type: {}", other)),
        }
    }

    /// Port of `TreeWatcher.watch()` from
    /// `powerline/lib/watcher/tree.py:56`.
    pub fn watch<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let path = realpath(path); // py:57
        let w = self.get_watcher(&path)?; // py:58
        self.watches.lock().unwrap().insert(path, w); // py:59
        Ok(())
    }

    /// Port of `TreeWatcher.expire_old_queries()` from
    /// `powerline/lib/watcher/tree.py:62`.
    pub fn expire_old_queries(&self) {
        // py:63-68  walk last_query_times, drop entries older than expire_time
        let now = monotonic();
        let mut times = self.last_query_times.lock().unwrap();
        times.retain(|_, lt| now - *lt <= self.expire_time);
    }

    /// Port of `TreeWatcher.__call__()` from
    /// `powerline/lib/watcher/tree.py:70`.
    ///
    /// Returns true if the tree has changed since last call (or first
    /// observation), false otherwise.
    pub fn check<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = realpath(path); // py:71
        self.expire_old_queries(); // py:72
        self.last_query_times // py:73
            .lock()
            .unwrap()
            .insert(path.clone(), monotonic());
        let watches = self.watches.lock().unwrap();
        // py:74-79  if path not in watches: watch + return True
        if !watches.contains_key(&path) {
            drop(watches);
            let _ = self.watch(&path);
            return true; // py:79
        }
        // py:80-85  check the watcher; if BaseDirChanged → drop + True;
        //          DirTooLarge → swap for dummy + False
        if let Some(w) = watches.get(&path) {
            w.check() // py:81
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_tree_watcher_always_false() {
        let w = DummyTreeWatcher::new(".");
        assert!(!w.check());
        assert!(w.is_dummy);
    }

    #[test]
    fn tree_watcher_unknown_type_errors() {
        let t = TreeWatcher::new("xyz", 10.0);
        assert!(t.get_watcher(".").is_err());
    }

    #[test]
    fn tree_watcher_first_call_returns_true_and_records_watch() {
        let t = TreeWatcher::new("auto", 10.0);
        let cwd = std::env::current_dir().unwrap();
        // First call on cwd → unwatched → returns true + watches it.
        assert!(t.check(&cwd));
        // Second call on cwd → watched → dummy returns false.
        assert!(!t.check(&cwd));
    }

    #[test]
    fn tree_watcher_expire_time_is_minutes_to_seconds() {
        let t = TreeWatcher::new("auto", 5.0);
        // py:27  expire_time = expire_time * 60
        assert!((t.expire_time - 300.0).abs() < 1e-9);
    }

    #[test]
    fn expire_old_queries_drops_stale_entries() {
        let t = TreeWatcher::new("auto", 0.0); // 0 minutes = expire immediately
        let cwd = std::env::current_dir().unwrap();
        t.last_query_times
            .lock()
            .unwrap()
            .insert(cwd.clone(), monotonic() - 1000.0);
        t.expire_old_queries();
        assert!(t.last_query_times.lock().unwrap().is_empty());
    }
}
