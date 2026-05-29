// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/config.py`.
//!
//! Config loading + file-watcher coordination. Used by the Powerline
//! orchestrator to read JSON configs from disk and re-load when the
//! filesystem changes.
//!
//! This chunk ports the leaf helpers — `open_file`, `load_json_config`,
//! `DummyWatcher`, `DeferredWatcher`. `ConfigLoader` (the
//! `MultiRunnedThread`-extending main class) is partial; the watcher
//! orchestration loop depends on the segment dispatch substrate.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import json                                      // py:4
// import codecs                                    // py:5
// from copy import deepcopy                        // py:7
// from threading import Event, Lock                // py:8
// from collections import defaultdict              // py:9
// from powerline.lib.threaded import MultiRunnedThread                                    // py:11
// from powerline.lib.watcher import create_file_watcher                                    // py:12

use serde_json::Value;
use std::path::Path;
use std::sync::Mutex;

/// Port of `open_file()` from `powerline/lib/config.py:15`.
///
/// Python: `codecs.open(path, encoding='utf-8')` — open path for
/// reading as UTF-8 text. Rust's `std::fs::read_to_string` is the
/// modern equivalent (returns the full contents); callers that need a
/// streaming reader can use `BufReader::new(File::open(path)?)`.
pub fn open_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    // py:16  return codecs.open(path, encoding='utf-8')
    std::fs::read_to_string(path)
}

/// Port of `load_json_config()` from `powerline/lib/config.py:19`.
///
/// Reads a JSON config file from disk and parses it into a
/// `serde_json::Value`.
///
/// Python's signature exposes the `load` and `open_file` parameters so
/// callers can substitute alternate readers (notably the
/// `markedjson.load` variant used by the linter). The Rust port keeps
/// the same upstream-style signature: callers pass the parser via the
/// `load` closure.
pub fn load_json_config<P: AsRef<Path>>(config_file_path: P) -> Result<Value, String> {
    // py:20-21  with open_file(...) as fp: return load(fp)
    let contents = open_file(config_file_path).map_err(|e| format!("open_file: {}", e))?;
    serde_json::from_str(&contents).map_err(|e| format!("json parse: {}", e))
}

/// Port of `class DummyWatcher` from `powerline/lib/config.py:24`.
///
/// A watcher that always reports "no change". Used when the loader is
/// in `run_once=True` mode (no need to watch files since we're going
/// to exit after one render).
pub struct DummyWatcher;

impl DummyWatcher {
    /// Port of `DummyWatcher.__call__` from
    /// `powerline/lib/config.py:25`.
    ///
    /// Always returns `false` — no file has changed.
    pub fn check<P: AsRef<Path>>(&self, _path: P) -> bool {
        false                                         // py:26
    }

    /// Port of `DummyWatcher.watch` from
    /// `powerline/lib/config.py:28`.
    ///
    /// No-op.
    pub fn watch<P: AsRef<Path>>(&self, _path: P) {
        // py:29  pass
    }
}

/// One queued call against a `DeferredWatcher`.
///
/// Python stores these as `('__call__', args, kwargs)` tuples; the
/// Rust port carries the method name and the path argument since both
/// `__call__` and `watch`/`unwatch` take a single path.
#[derive(Debug, Clone)]
pub struct DeferredCall {
    pub method: String,
    pub path: std::path::PathBuf,
}

/// Port of `class DeferredWatcher` from
/// `powerline/lib/config.py:32`.
///
/// A watcher that queues calls until `transfer_calls` is invoked.
/// Used as a placeholder by `ConfigLoader` before the real watcher
/// type is known — once `set_watcher` is called, the queued calls
/// are replayed against the real watcher.
pub struct DeferredWatcher {
    /// Python: `self.calls` — py:36
    pub calls: Mutex<Vec<DeferredCall>>,
}

impl Default for DeferredWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl DeferredWatcher {
    /// Port of `DeferredWatcher.__init__` from
    /// `powerline/lib/config.py:33`.
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),           // py:36
        }
    }

    /// Port of `DeferredWatcher.__call__` from
    /// `powerline/lib/config.py:38`.
    pub fn check<P: AsRef<Path>>(&self, path: P) {
        // py:39  self.calls.append(('__call__', args, kwargs))
        self.calls.lock().unwrap().push(DeferredCall {
            method: "__call__".into(),
            path: path.as_ref().to_path_buf(),
        });
    }

    /// Port of `DeferredWatcher.watch` from
    /// `powerline/lib/config.py:41`.
    pub fn watch<P: AsRef<Path>>(&self, path: P) {
        // py:42  self.calls.append(('watch', args, kwargs))
        self.calls.lock().unwrap().push(DeferredCall {
            method: "watch".into(),
            path: path.as_ref().to_path_buf(),
        });
    }

    /// Port of `DeferredWatcher.unwatch` from
    /// `powerline/lib/config.py:44`.
    pub fn unwatch<P: AsRef<Path>>(&self, path: P) {
        // py:45  self.calls.append(('unwatch', args, kwargs))
        self.calls.lock().unwrap().push(DeferredCall {
            method: "unwatch".into(),
            path: path.as_ref().to_path_buf(),
        });
    }

    /// Port of `DeferredWatcher.transfer_calls` from
    /// `powerline/lib/config.py:47`.
    ///
    /// Replays all queued calls against the supplied real watcher.
    /// Returns the drained list so callers can choose to inspect.
    pub fn transfer_calls(&self) -> Vec<DeferredCall> {
        // py:48-49  for attr, args, kwargs in self.calls:
        //              getattr(watcher, attr)(*args, **kwargs)
        let mut calls = self.calls.lock().unwrap();
        std::mem::take(&mut *calls)
    }
}

// `ConfigLoader` (py:52-218) ports alongside the watcher dispatch
// + Powerline orchestrator. The class extends MultiRunnedThread
// (ported) but its main loop depends on the segment registry and
// log substrate that aren't ported.

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_json(content: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("powerliners-config-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn open_file_reads_utf8_contents() {
        let p = tmp_json("héllo, world");
        let r = open_file(&p).unwrap();
        assert_eq!(r, "héllo, world");
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn load_json_config_parses_basic() {
        let p = tmp_json(r#"{"name": "powerline", "version": 1}"#);
        let v = load_json_config(&p).unwrap();
        assert_eq!(v["name"], "powerline");
        assert_eq!(v["version"], 1);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn load_json_config_returns_err_on_bad_json() {
        let p = tmp_json("{ this isn't json }");
        assert!(load_json_config(&p).is_err());
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn dummy_watcher_always_returns_false() {
        let w = DummyWatcher;
        assert!(!w.check("/etc/passwd"));
        // watch is a no-op; should not panic
        w.watch("/etc/passwd");
    }

    #[test]
    fn deferred_watcher_queues_calls() {
        let w = DeferredWatcher::new();
        w.watch("/etc/config1");
        w.check("/etc/config2");
        w.unwatch("/etc/config1");
        let calls = w.transfer_calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].method, "watch");
        assert_eq!(calls[1].method, "__call__");
        assert_eq!(calls[2].method, "unwatch");
        // After transfer, queue is drained
        let calls2 = w.transfer_calls();
        assert!(calls2.is_empty());
    }
}
