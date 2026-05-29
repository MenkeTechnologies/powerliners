// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/stat.py`.
//!
//! Stat-based file watcher: tracks mtimes of watched paths and reports
//! changes by polling. The portable fallback when `inotify` / `libuv`
//! are unavailable; on macOS this is the default since `inotify` is
//! Linux-only.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from threading import RLock                      // py:6
// from powerline.lib.path import realpath          // py:8

use crate::ported::lib::path::realpath;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Port of `class StatFileWatcher` from `powerline/lib/watcher/stat.py:11`.
///
/// Tracks mtimes of watched files; reports a file as changed when its
/// stored mtime differs from the current one. First-time observations
/// are reported as changed (matches Python's "no entry → assume new").
pub struct StatFileWatcher {
    // py:11
    /// Python: `self.watches` (`dict[path → mtime]`) — py:13
    /// Bucket-2 per PORT_PLAN.md: shared across watcher invocations
    /// in different threads (VCS segments call from a Rayon pool).
    pub watches: Mutex<HashMap<PathBuf, SystemTime>>,
    // py:14  self.lock = RLock() — Rust uses Mutex for the same
    // serialization; held only across single map operations so the
    // performance characteristic matches Python's RLock-around-dict.
}

impl Default for StatFileWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl StatFileWatcher {
    /// Port of `StatFileWatcher.__init__()` from
    /// `powerline/lib/watcher/stat.py:12`.
    pub fn new() -> Self {
        // py:12
        Self {
            watches: Mutex::new(HashMap::new()), // py:13
        }
    }

    /// Port of `StatFileWatcher.watch()` from
    /// `powerline/lib/watcher/stat.py:16`.
    ///
    /// Begin watching `path` — records its current mtime.
    pub fn watch<P: AsRef<Path>>(&self, path: P) {
        let path = realpath(path); // py:17
        let mtime = path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        let mut watches = self.watches.lock().unwrap(); // py:18  with self.lock
        watches.insert(path, mtime); // py:19  self.watches[path] = ...
    }

    /// Port of `StatFileWatcher.unwatch()` from
    /// `powerline/lib/watcher/stat.py:21`.
    pub fn unwatch<P: AsRef<Path>>(&self, path: P) {
        let path = realpath(path); // py:22
        let mut watches = self.watches.lock().unwrap(); // py:23
        watches.remove(&path); // py:24  self.watches.pop(path, None)
    }

    /// Port of `StatFileWatcher.is_watching()` from
    /// `powerline/lib/watcher/stat.py:26`.
    pub fn is_watching<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = realpath(path);
        let watches = self.watches.lock().unwrap(); // py:27
        watches.contains_key(&path) // py:28
    }

    /// Port of `StatFileWatcher.__call__()` from
    /// `powerline/lib/watcher/stat.py:30`.
    ///
    /// Returns `true` if the file is new to the watcher OR its mtime
    /// has changed since last call. Side effect: records the new
    /// mtime on every call (so subsequent checks compare against the
    /// most recent observation, matching Python behaviour at py:38).
    pub fn check<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = realpath(path); // py:31
        let current_mtime = path
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut watches = self.watches.lock().unwrap(); // py:32
        match watches.get(&path).copied() {
            None => {
                // py:33  if path not in self.watches
                watches.insert(path, current_mtime); // py:34
                true // py:35  return True
            }
            Some(stored) => {
                // py:36
                if current_mtime != stored {
                    // py:37  if mtime != self.watches[path]
                    watches.insert(path, current_mtime); // py:38
                    true // py:39  return True
                } else {
                    false // py:40  return False
                }
            }
        }
    }

    /// Port of `StatFileWatcher.close()` from
    /// `powerline/lib/watcher/stat.py:42`.
    pub fn close(&self) {
        let mut watches = self.watches.lock().unwrap(); // py:43
        watches.clear(); // py:44
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file_with_content(content: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-stat-test-{}-{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn watch_then_check_returns_false_unchanged() {
        let p = tmp_file_with_content("a");
        let w = StatFileWatcher::new();
        w.watch(&p);
        // Initial mtime stored by `watch`; first `check` sees the same mtime → false.
        assert!(!w.check(&p));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn check_returns_true_when_unwatched() {
        let p = tmp_file_with_content("a");
        let w = StatFileWatcher::new();
        // First call without `watch` → records & returns true (matches py:33-35).
        assert!(w.check(&p));
        // Second call → mtime unchanged → false.
        assert!(!w.check(&p));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn check_returns_true_after_mtime_change() {
        let p = tmp_file_with_content("a");
        let w = StatFileWatcher::new();
        w.watch(&p);
        // Force a different mtime by sleeping past filesystem mtime resolution
        // (most modern Linux fs is nanosecond; macOS APFS is microsecond).
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&p, "b").unwrap();
        assert!(w.check(&p));
        // Subsequent unchanged check returns false.
        assert!(!w.check(&p));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn unwatch_removes_from_set() {
        let p = tmp_file_with_content("a");
        let w = StatFileWatcher::new();
        w.watch(&p);
        assert!(w.is_watching(&p));
        w.unwatch(&p);
        assert!(!w.is_watching(&p));
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn close_clears_all_watches() {
        let p1 = tmp_file_with_content("a");
        let p2 = tmp_file_with_content("b");
        let w = StatFileWatcher::new();
        w.watch(&p1);
        w.watch(&p2);
        assert!(w.is_watching(&p1));
        assert!(w.is_watching(&p2));
        w.close();
        assert!(!w.is_watching(&p1));
        assert!(!w.is_watching(&p2));
        std::fs::remove_file(&p1).ok();
        std::fs::remove_file(&p2).ok();
    }
}
