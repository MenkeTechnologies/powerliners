// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/__init__.py`.
//!
//! File-watcher factory: dispatches to `inotify` (Linux), `pyuv`, or
//! `stat` backend based on platform availability. Used by the config
//! loader to re-read user/system config files on change, and by VCS
//! segments to invalidate caches when a working-tree file is touched.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// from powerline.lib.watcher.stat import StatFileWatcher                                  // py:6
// from powerline.lib.watcher.inotify import INotifyFileWatcher                            // py:7
// from powerline.lib.watcher.tree import TreeWatcher                                      // py:8
// from powerline.lib.watcher.uv import UvFileWatcher, UvNotFound                          // py:9
// from powerline.lib.inotify import INotifyError                                          // py:10

pub mod inotify;
pub mod stat;
pub mod tree;
pub mod uv;

use crate::ported::lib::watcher::stat::StatFileWatcher;

/// File-watcher trait — the contract every backend (`StatFileWatcher`,
/// `INotifyFileWatcher`, `UvFileWatcher`) honours.
///
/// Mirrors the Python duck-typed protocol where each watcher exposes
/// `watch(path)`, `unwatch(path)`, `__call__(path)` (returns bool),
/// and `close()`.
pub trait FileWatcher {
    fn watch(&self, path: &std::path::Path);
    fn unwatch(&self, path: &std::path::Path);
    fn check(&self, path: &std::path::Path) -> bool;
    fn close(&self);
}

impl FileWatcher for StatFileWatcher {
    fn watch(&self, path: &std::path::Path) {
        StatFileWatcher::watch(self, path);
    }
    fn unwatch(&self, path: &std::path::Path) {
        StatFileWatcher::unwatch(self, path);
    }
    fn check(&self, path: &std::path::Path) -> bool {
        StatFileWatcher::check(self, path)
    }
    fn close(&self) {
        StatFileWatcher::close(self);
    }
}

/// Port of `create_file_watcher()` from
/// `powerline/lib/watcher/__init__.py:13`.
///
/// Create an object that can watch for changes to specified files.
///
/// Use `.check()` method (Python: `__call__`) of the returned object
/// to start watching the file or check whether file has changed since
/// last call.
///
/// Use `.unwatch()` method of the returned object to stop watching
/// the file.
///
/// Uses inotify if available, then pyuv, otherwise tracks mtimes.
/// `expire_time` is the number of minutes after the last query for a
/// given path for the inotify watch for that path to be automatically
/// removed. This conserves kernel resources.
///
/// :param str watcher_type:
///     One of `inotify` (linux only), `uv`, `stat`, `auto`.
/// :param int expire_time:
///     Number of minutes since last `check()` before inotify watcher
///     will stop watching given file.
pub fn create_file_watcher(
    _pl: &(),
    watcher_type: &str,
    _expire_time: i32,
) -> Box<dyn FileWatcher + Send + Sync> {
    match watcher_type {
        // py:39-41  explicit stat request
        "stat" => Box::new(StatFileWatcher::new()),
        // py:42-45  explicit inotify request — not yet implemented in Rust port.
        // The faithful port will dispatch to INotifyFileWatcher when that lands;
        // for now we fall through to stat so the dispatch shape stays usable.
        "inotify" => Box::new(StatFileWatcher::new()),
        // py:46-49  explicit uv request — same fallback note.
        "uv" => Box::new(StatFileWatcher::new()),
        // py:51-61  auto: try inotify, then uv, then stat (fallback).
        // Rust port: stat is the only backend implemented so far; always
        // returns it for the auto path.
        _ => Box::new(StatFileWatcher::new()), // py:63  StatFileWatcher() fallback
    }
}

/// Port of `create_tree_watcher()` from
/// `powerline/lib/watcher/__init__.py:66`.
///
/// Create an object that can watch for changes in specified
/// directories.
///
/// **Status:** stub — `TreeWatcher` (lib/watcher/tree.py) hasn't been
/// ported yet; this returns a passthrough that routes calls into a
/// stat-based fallback. Callers that need recursive directory watching
/// will require the real `TreeWatcher` port (Phase 3 of PORT_PLAN.md).
pub fn create_tree_watcher(
    _pl: &(),
    _watcher_type: &str,
    _expire_time: i32,
) -> Box<dyn FileWatcher + Send + Sync> {
    // py:74  return TreeWatcher(pl, watcher_type, expire_time)
    Box::new(StatFileWatcher::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_file_watcher_returns_a_watcher() {
        let w = create_file_watcher(&(), "auto", 10);
        // Smoke test the trait surface.
        let p = std::env::temp_dir().join("powerliners-create-test.tmp");
        std::fs::write(&p, "x").unwrap();
        assert!(w.check(&p));
        w.unwatch(&p);
        w.close();
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn create_file_watcher_stat_explicit() {
        let _w = create_file_watcher(&(), "stat", 10);
        // No panic = pass.
    }

    #[test]
    fn create_file_watcher_inotify_falls_back_to_stat_on_non_linux() {
        let _w = create_file_watcher(&(), "inotify", 10);
        // No panic = pass.
    }
}
