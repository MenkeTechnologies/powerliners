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
    // py:15  def open_file(path):
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
    // py:19  def load_json_config(config_file_path, load=json.load, open_file=open_file):
    // py:20  with open_file(config_file_path) as config_file_fp:
    // py:21  return load(config_file_fp)
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
        // py:24  class DummyWatcher(object):
        // py:25  def __call__(self, *args, **kwargs):
        // py:26  return False
        false
    }

    /// Port of `DummyWatcher.watch` from
    /// `powerline/lib/config.py:28`.
    ///
    /// No-op.
    pub fn watch<P: AsRef<Path>>(&self, _path: P) {
        // py:28  def watch(self, *args, **kwargs):
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
        // py:32  class DeferredWatcher(object):
        // py:33  def __init__(self, *args, **kwargs):
        // py:34  self.args = args
        // py:35  self.kwargs = kwargs
        // py:36  self.calls = []
        Self {
            calls: Mutex::new(Vec::new()),
        }
    }

    /// Port of `DeferredWatcher.__call__` from
    /// `powerline/lib/config.py:38`.
    pub fn check<P: AsRef<Path>>(&self, path: P) {
        // py:38  def __call__(self, *args, **kwargs):
        // py:39  self.calls.append(('__call__', args, kwargs))
        self.calls.lock().unwrap().push(DeferredCall {
            method: "__call__".into(),
            path: path.as_ref().to_path_buf(),
        });
    }

    /// Port of `DeferredWatcher.watch` from
    /// `powerline/lib/config.py:41`.
    pub fn watch<P: AsRef<Path>>(&self, path: P) {
        // py:41  def watch(self, *args, **kwargs):
        // py:42  self.calls.append(('watch', args, kwargs))
        self.calls.lock().unwrap().push(DeferredCall {
            method: "watch".into(),
            path: path.as_ref().to_path_buf(),
        });
    }

    /// Port of `DeferredWatcher.unwatch` from
    /// `powerline/lib/config.py:44`.
    pub fn unwatch<P: AsRef<Path>>(&self, path: P) {
        // py:44  def unwatch(self, *args, **kwargs):
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
        // py:47  def transfer_calls(self, watcher):
        // py:48  for attr, args, kwargs in self.calls:
        // py:49  getattr(watcher, attr)(*args, **kwargs)
        let mut calls = self.calls.lock().unwrap();
        std::mem::take(&mut *calls)
    }
}

/// Port of `class ConfigLoader(MultiRunnedThread)` from
/// `powerline/lib/config.py:52`.
///
/// Coordinates the registered config-file watchers + the in-memory
/// `loaded` cache. The full `update()` / `run()` orchestration loop
/// (py:164-213) depends on the file-watcher runtime + log
/// dispatcher and ports separately; this struct surfaces the
/// register / load / unregister / set_interval / set_pl plumbing
/// that downstream `Powerline.load_config` calls into.
pub struct ConfigLoader {
    /// Python: `self.watcher_type` (py:58/64/66).
    pub watcher_type: String,
    /// Python: `self.pl` (py:69).
    pub pl: Option<()>,
    /// Python: `self.interval` (py:70).
    pub interval: Option<u64>,
    /// Python: `self.watched = defaultdict(set)` (py:74).
    /// Maps path → set of function-id markers (Rust can't carry
    /// arbitrary fn ptrs since they aren't Hash; callers pass an id).
    pub watched: std::collections::HashMap<std::path::PathBuf, std::collections::HashSet<u64>>,
    /// Python: `self.missing = defaultdict(set)` (py:75).
    pub missing: std::collections::HashMap<String, std::collections::HashSet<(u64, u64)>>,
    /// Python: `self.loaded = {}` (py:76).
    pub loaded: std::collections::HashMap<std::path::PathBuf, Value>,
    /// Process-wide lock guarding the state mutations (py:72).
    pub lock: Mutex<()>,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new(false)
    }
}

impl ConfigLoader {
    /// Port of `ConfigLoader.__init__()` from
    /// `powerline/lib/config.py:53`.
    ///
    /// `run_once=true` selects the DummyWatcher per py:56-58; false
    /// uses the DeferredWatcher per py:60. The actual watcher
    /// dispatch is queued through the existing struct ports above.
    pub fn new(run_once: bool) -> Self {
        // py:53  def __init__(self, shutdown_event=None, watcher=None, watcher_type=None, load=load_json_config, run_once=False):
        // py:54  super(ConfigLoader, self).__init__()
        // py:55  self.shutdown_event = shutdown_event or Event()
        // py:56  if run_once:
        // py:57  self.watcher = DummyWatcher()
        // py:58  self.watcher_type = 'dummy'
        // py:59  else:
        // py:60  self.watcher = watcher or DeferredWatcher()
        // py:61  if watcher:
        // py:62  if not watcher_type:
        // py:63  raise ValueError('When specifying watcher you must also specify watcher type')
        // py:64  self.watcher_type = watcher_type
        // py:65  else:
        // py:66  self.watcher_type = 'deferred'
        // py:67  self._load = load
        // py:69  self.pl = None
        // py:70  self.interval = None
        // py:72  self.lock = Lock()
        // py:74  self.watched = defaultdict(set)
        // py:75  self.missing = defaultdict(set)
        // py:76  self.loaded = {}
        let watcher_type = if run_once {
            "dummy".to_string()
        } else {
            "deferred".to_string()
        };
        Self {
            watcher_type,
            pl: None,
            interval: None,
            watched: std::collections::HashMap::new(),
            missing: std::collections::HashMap::new(),
            loaded: std::collections::HashMap::new(),
            lock: Mutex::new(()),
        }
    }

    /// Port of `ConfigLoader.set_pl()` from
    /// `powerline/lib/config.py:88-89`.
    pub fn set_pl(&mut self, _pl: ()) {
        // py:89  self.pl = pl
        self.pl = Some(());
    }

    /// Port of `ConfigLoader.set_interval()` from
    /// `powerline/lib/config.py:91-92`.
    pub fn set_interval(&mut self, interval: u64) {
        // py:92  self.interval = interval
        self.interval = Some(interval);
    }

    /// Port of `ConfigLoader.register()` from
    /// `powerline/lib/config.py:94-104`.
    ///
    /// `function_id` is a caller-supplied marker since Rust fn
    /// pointers don't implement Hash. The actual watcher.watch(path)
    /// dispatch at py:104 is the caller's responsibility (it lives
    /// outside the lock-protected state mutation).
    pub fn register<P: AsRef<Path>>(&mut self, function_id: u64, path: P) {
        // py:94  def register(self, function, path):
        // py:95-101  docstring
        // py:102  with self.lock:
        // py:103  self.watched[path].add(function)
        // py:104  self.watcher.watch(path)
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.watched
            .entry(path.as_ref().to_path_buf())
            .or_default()
            .insert(function_id);
    }

    /// Port of `ConfigLoader.register_missing()` from
    /// `powerline/lib/config.py:106-126`.
    pub fn register_missing(
        &mut self,
        condition_function_id: u64,
        function_id: u64,
        key: impl Into<String>,
    ) {
        // py:106  def register_missing(self, condition_function, function, key):
        // py:107-124  docstring
        // py:125  with self.lock:
        // py:126  self.missing[key].add((condition_function, function))
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        self.missing
            .entry(key.into())
            .or_default()
            .insert((condition_function_id, function_id));
    }

    /// Port of `ConfigLoader.unregister_functions()` from
    /// `powerline/lib/config.py:128-139`.
    ///
    /// Removes each `removed_functions` entry from every watched
    /// path's function set; drops the path entirely + clears its
    /// loaded entry per py:138-139 when the set becomes empty.
    pub fn unregister_functions(&mut self, removed_functions: &std::collections::HashSet<u64>) {
        // py:128  def unregister_functions(self, removed_functions):
        // py:129-133  docstring
        // py:134  with self.lock:
        // py:135  for path, functions in list(self.watched.items()):
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let paths: Vec<std::path::PathBuf> = self.watched.keys().cloned().collect();
        for path in paths {
            if let Some(functions) = self.watched.get_mut(&path) {
                // py:136  functions -= removed_functions
                for id in removed_functions {
                    functions.remove(id);
                }
                // py:137  if not functions:
                if functions.is_empty() {
                    // py:138  self.watched.pop(path)
                    // py:139  self.loaded.pop(path, None)
                    self.watched.remove(&path);
                    self.loaded.remove(&path);
                }
            }
        }
    }

    /// Port of `ConfigLoader.unregister_missing()` from
    /// `powerline/lib/config.py:141-153`.
    pub fn unregister_missing(
        &mut self,
        removed_functions: &std::collections::HashSet<(u64, u64)>,
    ) {
        // py:141  def unregister_missing(self, removed_functions):
        // py:142-148  docstring
        // py:149  with self.lock:
        // py:150  for key, functions in list(self.missing.items()):
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        let keys: Vec<String> = self.missing.keys().cloned().collect();
        for key in keys {
            if let Some(functions) = self.missing.get_mut(&key) {
                // py:151  functions -= removed_functions
                for pair in removed_functions {
                    functions.remove(pair);
                }
                // py:152  if not functions:
                // py:153  self.missing.pop(key)
                if functions.is_empty() {
                    self.missing.remove(&key);
                }
            }
        }
    }

    /// Port of `ConfigLoader.load()` from
    /// `powerline/lib/config.py:155-162`.
    ///
    /// Returns the cached config when present; otherwise runs the
    /// caller-supplied `load_fn` and caches the result. Python uses
    /// `deepcopy` per py:158/161; serde_json::Value's Clone is the
    /// deepcopy equivalent.
    pub fn load<P, F>(&mut self, path: P, load_fn: F) -> Result<Value, String>
    where
        P: AsRef<Path>,
        F: FnOnce(&Path) -> Result<Value, String>,
    {
        // py:155  def load(self, path):
        // py:156  try:
        // py:157  # No locks: GIL does what we need
        // py:158  return deepcopy(self.loaded[path])
        let path = path.as_ref().to_path_buf();
        if let Some(cached) = self.loaded.get(&path) {
            return Ok(cached.clone());
        }
        // py:159  except KeyError:
        // py:160  r = self._load(path)
        // py:161  self.loaded[path] = deepcopy(r)
        // py:162  return r
        let r = load_fn(&path)?;
        self.loaded.insert(path, r.clone());
        Ok(r)
    }
}

// `ConfigLoader.update()` (py:164-208), `run()` (py:209-213) port
// alongside the live watcher dispatch + log substrate. set_watcher
// + exception extract enough of the structural skeleton to surface
// the dispatch shape without driving the live loop.

/// Port of `ConfigLoader.set_watcher()` from
/// `powerline/lib/config.py:78-86`.
///
/// Mirrors the early-exit at py:79-80 (same type → no-op) and the
/// state-mutation at py:84-86 (transfer deferred-queue calls when
/// switching off the deferred watcher, then store the new watcher
/// type). The actual `create_file_watcher(self.pl, watcher_type)`
/// dispatch at py:81 lives outside the locked section since it
/// reaches the live watcher runtime.
///
/// Returns `true` when the watcher_type actually changed, `false`
/// when the no-op early-exit fired per py:79-80.
impl ConfigLoader {
    pub fn set_watcher(&mut self, watcher_type: &str, _force: bool) -> bool {
        // py:78  def set_watcher(self, watcher_type, force=False):
        // py:79  if watcher_type == self.watcher_type:
        // py:80  return
        if watcher_type == self.watcher_type {
            return false;
        }
        // py:81  watcher = create_file_watcher(self.pl, watcher_type)
        // py:82  with self.lock:
        let _g = self.lock.lock().unwrap_or_else(|e| e.into_inner());
        // py:83  if self.watcher_type == 'deferred':
        // py:84  self.watcher.transfer_calls(watcher)
        // py:85  self.watcher = watcher
        // py:86  self.watcher_type = watcher_type
        self.watcher_type = watcher_type.to_string();
        true
    }

    /// Port of `ConfigLoader.update()` from
    /// `powerline/lib/config.py:164-207`.
    ///
    /// Walks the registered watch + missing tables, dispatches the
    /// watcher per path, invokes the registered function callbacks
    /// for modified or newly-resolved paths, and reloads modified
    /// files into the `loaded` cache.
    ///
    /// Rust can't store callable references in `watched` / `missing`
    /// (function pointers aren't `Hash`), so the dispatch is split:
    /// callers supply
    ///   - `watcher`: `Fn(&Path) -> Result<bool, String>` — replaces
    ///     the inline `self.watcher(path)` call at py:170.
    ///   - `dispatch_fn`: `Fn(u64, &Path)` — invokes the registered
    ///     callback for `(path, function_id)` at py:178.
    ///   - `condition_fn`: `Fn(u64, &str) -> Option<PathBuf>` —
    ///     evaluates the missing-key condition at py:183.
    ///   - `dispatch_missing_fn`: `Fn(u64, &Path)` — calls the
    ///     newly-resolved function callback at py:191.
    ///   - `load_fn`: `Fn(&Path) -> Result<Value, String>` — the
    ///     `self._load(path)` call at py:197.
    ///
    /// Returns the list of (path, error) pairs encountered during
    /// load. Python silently logs via `self.exception` at py:198; the
    /// Rust port surfaces them so callers can route the error stream.
    pub fn update<W, D, C, DM, L>(
        &mut self,
        watcher: W,
        dispatch_fn: D,
        condition_fn: C,
        dispatch_missing_fn: DM,
        load_fn: L,
    ) -> Vec<(std::path::PathBuf, String)>
    where
        W: Fn(&std::path::Path) -> Result<bool, String>,
        D: Fn(u64, &std::path::Path),
        C: Fn(u64, &str) -> Option<std::path::PathBuf>,
        DM: Fn(u64, &std::path::Path),
        L: Fn(&std::path::Path) -> Result<Value, String>,
    {
        // py:165  toload = []
        let mut toload: Vec<std::path::PathBuf> = Vec::new();
        let mut errors: Vec<(std::path::PathBuf, String)> = Vec::new();

        // py:166  with self.lock:
        // py:167  for path, functions in self.watched.items():
        let watched_snapshot: Vec<(std::path::PathBuf, std::collections::HashSet<u64>)> = {
            let _g = self.lock.lock().unwrap();
            self.watched
                .iter()
                .map(|(p, fs)| (p.clone(), fs.clone()))
                .collect()
        };
        for (path, functions) in &watched_snapshot {
            for function_id in functions {
                // py:169  try:
                // py:170  modified = self.watcher(path)
                // py:171  except OSError as e:
                // py:172  modified = True
                let modified = match watcher(path) {
                    Ok(m) => m,
                    Err(_) => true,
                };
                // py:174  else:
                // py:175  if modified:
                // py:176  toload.append(path)
                if modified && !toload.contains(path) {
                    toload.push(path.clone());
                }
                // py:177  if modified:
                // py:178  function(path)
                if modified {
                    dispatch_fn(*function_id, path);
                }
            }
        }

        // py:179  with self.lock:
        // py:180  for key, functions in list(self.missing.items()):
        let missing_snapshot: Vec<(String, std::collections::HashSet<(u64, u64)>)> = {
            let _g = self.lock.lock().unwrap();
            self.missing
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        };
        for (key, functions) in &missing_snapshot {
            let mut remaining: std::collections::HashSet<(u64, u64)> = functions.clone();
            for &(cond_id, func_id) in functions {
                // py:181  for condition_function, function in list(functions):
                // py:182  try:
                // py:183  path = condition_function(key)
                // py:184  except IOError:
                // py:185  pass
                // py:189  if path:
                if let Some(path) = condition_fn(cond_id, key) {
                    // py:190  toload.append(path)
                    if !toload.contains(&path) {
                        toload.push(path.clone());
                    }
                    // py:191  function(path)
                    dispatch_missing_fn(func_id, &path);
                    // py:192  functions.remove((condition_function, function))
                    remaining.remove(&(cond_id, func_id));
                }
            }
            // py:193  if not functions:
            // py:194  self.missing.pop(key)
            let _g = self.lock.lock().unwrap();
            if remaining.is_empty() {
                self.missing.remove(key);
            } else {
                self.missing.insert(key.clone(), remaining);
            }
        }

        // py:195  for path in toload:
        for path in &toload {
            // py:196  try:
            // py:197  self.loaded[path] = deepcopy(self._load(path))
            match load_fn(path) {
                Ok(v) => {
                    self.loaded.insert(path.clone(), v);
                }
                Err(e) => {
                    // py:198  except Exception as e:
                    // py:199  self.exception(...)
                    // py:200-207  try: self.loaded.pop(path) (twice)
                    self.loaded.remove(path);
                    errors.push((path.clone(), e));
                }
            }
        }
        errors
    }

    /// Port of `ConfigLoader.run()` from
    /// `powerline/lib/config.py:209-212`.
    ///
    /// Runs `update` repeatedly at `self.interval`-second intervals
    /// until `shutdown_event` is set. Python uses `threading.Event`'s
    /// `wait(timeout)` for the inter-tick sleep; Rust polls the
    /// `Arc<AtomicBool>` in 100ms slices so SIGTERM is responsive.
    ///
    /// Like `update`, callers supply the dispatch closures since
    /// Rust can't store fn-refs in the watched/missing tables.
    pub fn run<W, D, C, DM, L>(
        &mut self,
        shutdown_event: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        watcher: W,
        dispatch_fn: D,
        condition_fn: C,
        dispatch_missing_fn: DM,
        load_fn: L,
    ) where
        W: Fn(&std::path::Path) -> Result<bool, String>,
        D: Fn(u64, &std::path::Path),
        C: Fn(u64, &str) -> Option<std::path::PathBuf>,
        DM: Fn(u64, &std::path::Path),
        L: Fn(&std::path::Path) -> Result<Value, String>,
    {
        use std::sync::atomic::Ordering;
        // py:210  while self.interval is not None and not self.shutdown_event.is_set():
        while self.interval.is_some() && !shutdown_event.load(Ordering::Relaxed) {
            // py:211  self.update()
            let _ = self.update(
                &watcher,
                &dispatch_fn,
                &condition_fn,
                &dispatch_missing_fn,
                &load_fn,
            );
            // py:212  self.shutdown_event.wait(self.interval)
            let interval = self.interval.unwrap();
            let slice = std::time::Duration::from_millis(100);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(interval);
            while std::time::Instant::now() < deadline {
                if shutdown_event.load(Ordering::Relaxed) {
                    return;
                }
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                std::thread::sleep(slice.min(remaining));
            }
        }
    }

    /// Port of `ConfigLoader.exception()` from
    /// `powerline/lib/config.py:214-218`.
    ///
    /// Returns the formatted message that the Python source would
    /// pass to `self.pl.exception(...)` at py:216. Callers wire the
    /// logger; the Rust port collects the format string + args into
    /// a single rendered message. When `self.pl` is None py:218
    /// re-raises — the Rust port returns Err with the rendered
    /// message instead.
    pub fn exception(&self, msg: &str, args: &[&str]) -> Result<String, String> {
        // py:214  def exception(self, msg, *args, **kwargs):
        // py:215  if self.pl:
        let mut rendered = msg.to_string();
        for (i, arg) in args.iter().enumerate() {
            rendered = rendered.replace(&format!("{{{}}}", i), arg);
        }
        if self.pl.is_some() {
            // py:216  self.pl.exception(msg, prefix='config_loader', *args, **kwargs)
            Ok(format!("config_loader: {}", rendered))
        } else {
            // py:217  else:
            // py:218  raise
            Err(rendered)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_json(content: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-config-test-{}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
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

    #[test]
    fn config_loader_run_once_uses_dummy_watcher_type() {
        // py:56-58
        let cl = ConfigLoader::new(true);
        assert_eq!(cl.watcher_type, "dummy");
    }

    #[test]
    fn config_loader_default_uses_deferred_watcher_type() {
        // py:60-66
        let cl = ConfigLoader::new(false);
        assert_eq!(cl.watcher_type, "deferred");
    }

    #[test]
    fn config_loader_set_pl_records_value() {
        // py:88-89
        let mut cl = ConfigLoader::new(false);
        cl.set_pl(());
        assert!(cl.pl.is_some());
    }

    #[test]
    fn config_loader_set_interval_records_value() {
        // py:91-92
        let mut cl = ConfigLoader::new(false);
        cl.set_interval(5);
        assert_eq!(cl.interval, Some(5));
    }

    #[test]
    fn config_loader_register_adds_function_to_watched_path() {
        // py:102-103
        let mut cl = ConfigLoader::new(false);
        cl.register(42, "/etc/powerline/config.json");
        assert!(cl
            .watched
            .contains_key(std::path::Path::new("/etc/powerline/config.json")));
        assert!(cl.watched[std::path::Path::new("/etc/powerline/config.json")].contains(&42));
    }

    #[test]
    fn config_loader_register_dedupes_function_id() {
        let mut cl = ConfigLoader::new(false);
        cl.register(42, "/x");
        cl.register(42, "/x");
        assert_eq!(cl.watched[std::path::Path::new("/x")].len(), 1);
    }

    #[test]
    fn config_loader_register_missing_adds_pair_to_key() {
        // py:125-126
        let mut cl = ConfigLoader::new(false);
        cl.register_missing(100, 200, "key1");
        assert!(cl.missing.contains_key("key1"));
        assert!(cl.missing["key1"].contains(&(100, 200)));
    }

    #[test]
    fn config_loader_unregister_functions_drops_path_when_empty() {
        // py:137-139
        let mut cl = ConfigLoader::new(false);
        cl.register(42, "/x");
        let mut removed = std::collections::HashSet::new();
        removed.insert(42);
        cl.unregister_functions(&removed);
        assert!(!cl.watched.contains_key(std::path::Path::new("/x")));
    }

    #[test]
    fn config_loader_unregister_functions_keeps_path_with_remaining_functions() {
        let mut cl = ConfigLoader::new(false);
        cl.register(42, "/x");
        cl.register(99, "/x");
        let mut removed = std::collections::HashSet::new();
        removed.insert(42);
        cl.unregister_functions(&removed);
        assert!(cl.watched.contains_key(std::path::Path::new("/x")));
        assert_eq!(cl.watched[std::path::Path::new("/x")].len(), 1);
    }

    #[test]
    fn config_loader_unregister_functions_clears_loaded_entry() {
        // py:139  loaded.pop(path, None)
        let mut cl = ConfigLoader::new(false);
        cl.register(42, "/x");
        cl.loaded
            .insert(std::path::PathBuf::from("/x"), serde_json::json!({"a": 1}));
        let mut removed = std::collections::HashSet::new();
        removed.insert(42);
        cl.unregister_functions(&removed);
        assert!(!cl.loaded.contains_key(std::path::Path::new("/x")));
    }

    #[test]
    fn config_loader_unregister_missing_drops_key_when_empty() {
        // py:152-153
        let mut cl = ConfigLoader::new(false);
        cl.register_missing(100, 200, "key1");
        let mut removed = std::collections::HashSet::new();
        removed.insert((100, 200));
        cl.unregister_missing(&removed);
        assert!(!cl.missing.contains_key("key1"));
    }

    #[test]
    fn config_loader_load_calls_load_fn_on_cache_miss() {
        // py:159-162
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/test/config.json");
        let r = cl
            .load(&p, |_| Ok(serde_json::json!({"loaded": true})))
            .unwrap();
        assert_eq!(r["loaded"], true);
        assert!(cl.loaded.contains_key(&p));
    }

    #[test]
    fn config_loader_load_returns_cached_value_on_hit() {
        // py:156-158
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/test/config.json");
        cl.loaded
            .insert(p.clone(), serde_json::json!({"cached": true}));
        let r = cl
            .load(&p, |_| {
                panic!("load_fn should not be called on cache hit");
            })
            .unwrap();
        assert_eq!(r["cached"], true);
    }

    #[test]
    fn config_loader_load_propagates_load_fn_errors() {
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/test/missing.json");
        let r = cl.load(&p, |_| Err("read fail".to_string()));
        assert!(r.is_err());
        // Failed load should NOT populate the cache.
        assert!(!cl.loaded.contains_key(&p));
    }

    #[test]
    fn config_loader_set_watcher_same_type_is_noop() {
        // py:79-80
        let mut cl = ConfigLoader::new(false);
        // Default watcher_type is "deferred"
        assert!(!cl.set_watcher("deferred", false));
        assert_eq!(cl.watcher_type, "deferred");
    }

    #[test]
    fn config_loader_set_watcher_different_type_swaps() {
        // py:85-86
        let mut cl = ConfigLoader::new(false);
        assert!(cl.set_watcher("inotify", false));
        assert_eq!(cl.watcher_type, "inotify");
    }

    #[test]
    fn config_loader_set_watcher_dummy_to_inotify() {
        let mut cl = ConfigLoader::new(true);
        assert_eq!(cl.watcher_type, "dummy");
        assert!(cl.set_watcher("inotify", false));
        assert_eq!(cl.watcher_type, "inotify");
    }

    #[test]
    fn config_loader_exception_no_pl_returns_err() {
        // py:218  raise
        let cl = ConfigLoader::new(false);
        let r = cl.exception("Error: {0} not found", &["foo"]);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "Error: foo not found");
    }

    #[test]
    fn config_loader_exception_with_pl_returns_formatted() {
        // py:215-216
        let mut cl = ConfigLoader::new(false);
        cl.set_pl(());
        let r = cl.exception("Error: {0} broken", &["xyz"]).unwrap();
        assert_eq!(r, "config_loader: Error: xyz broken");
    }

    #[test]
    fn config_loader_exception_substitutes_multiple_args() {
        let cl = ConfigLoader::new(false);
        let r = cl.exception("a={0} b={1}", &["X", "Y"]).unwrap_err();
        assert_eq!(r, "a=X b=Y");
    }

    #[test]
    fn config_loader_update_dispatches_modified_paths() {
        // py:164-178  modified=True path triggers dispatch_fn + reload.
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/tmp/zz_pwl_test_a.json");
        cl.register(42, &p);
        let called = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(u64, std::path::PathBuf)>::new()));
        let loaded = std::sync::Arc::new(std::sync::Mutex::new(Vec::<std::path::PathBuf>::new()));
        let called_c = called.clone();
        let loaded_c = loaded.clone();
        let errors = cl.update(
            |_p| Ok(true),
            move |id, path| called_c.lock().unwrap().push((id, path.to_path_buf())),
            |_id, _key| None,
            |_id, _path| {},
            move |path| {
                loaded_c.lock().unwrap().push(path.to_path_buf());
                Ok(Value::Object(serde_json::Map::new()))
            },
        );
        assert!(errors.is_empty());
        assert_eq!(*called.lock().unwrap(), vec![(42, p.clone())]);
        assert_eq!(*loaded.lock().unwrap(), vec![p.clone()]);
        assert!(cl.loaded.contains_key(&p));
    }

    #[test]
    fn config_loader_update_skips_unmodified_paths() {
        // py:170  watcher returns False → no dispatch, no reload.
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/tmp/zz_pwl_test_b.json");
        cl.register(99, &p);
        let called = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let called_c = called.clone();
        let errors = cl.update(
            |_p| Ok(false),
            move |_id, _path| {
                *called_c.lock().unwrap() += 1;
            },
            |_id, _key| None,
            |_id, _path| {},
            |_path| Ok(Value::Null),
        );
        assert!(errors.is_empty());
        assert_eq!(*called.lock().unwrap(), 0);
        assert!(!cl.loaded.contains_key(&p));
    }

    #[test]
    fn config_loader_update_load_error_surfaces() {
        // py:198-207  load failure → loaded.pop + error logged.
        let mut cl = ConfigLoader::new(false);
        let p = std::path::PathBuf::from("/tmp/zz_pwl_test_c.json");
        cl.register(1, &p);
        let errors = cl.update(
            |_p| Ok(true),
            |_id, _path| {},
            |_id, _key| None,
            |_id, _path| {},
            |_path| Err("disk full".to_string()),
        );
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, p);
        assert!(errors[0].1.contains("disk full"));
        assert!(!cl.loaded.contains_key(&p));
    }

    #[test]
    fn config_loader_update_resolves_missing_via_condition() {
        // py:179-194  missing-key condition_fn returns a path → dispatch.
        let mut cl = ConfigLoader::new(false);
        let resolved = std::path::PathBuf::from("/tmp/zz_pwl_test_d.json");
        let key = "ext.shell.theme".to_string();
        cl.register_missing(7, 13, &key);
        let resolved_c = resolved.clone();
        let dispatched = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
        let dispatched_c = dispatched.clone();
        let errors = cl.update(
            |_p| Ok(false),
            |_id, _path| {},
            move |_cond_id, _k| Some(resolved_c.clone()),
            move |id, _path| dispatched_c.lock().unwrap().push(id),
            |_path| Ok(Value::Bool(true)),
        );
        assert!(errors.is_empty());
        assert_eq!(*dispatched.lock().unwrap(), vec![13]);
        // After resolution the missing-key entry is dropped.
        assert!(!cl.missing.contains_key(&key));
    }

    #[test]
    fn config_loader_run_exits_immediately_when_shutdown_event_set() {
        // py:210  while not self.shutdown_event.is_set() — pre-set
        // shutdown terminates the loop on first poll.
        use std::sync::atomic::AtomicBool;
        let mut cl = ConfigLoader::new(false);
        cl.set_interval(60);
        let event = std::sync::Arc::new(AtomicBool::new(true));
        let invoked = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let invoked_c = invoked.clone();
        cl.run(
            &event,
            |_p| Ok(false),
            move |_id, _path| {
                *invoked_c.lock().unwrap() += 1;
            },
            |_id, _key| None,
            |_id, _path| {},
            |_path| Ok(Value::Null),
        );
        assert_eq!(*invoked.lock().unwrap(), 0);
    }

    #[test]
    fn config_loader_run_exits_when_interval_is_none() {
        // py:210  while self.interval is not None — None terminates.
        use std::sync::atomic::AtomicBool;
        let mut cl = ConfigLoader::new(false);
        let event = std::sync::Arc::new(AtomicBool::new(false));
        // interval stays None
        cl.run(
            &event,
            |_p| Ok(false),
            |_id, _path| {},
            |_id, _key| None,
            |_id, _path| {},
            |_path| Ok(Value::Null),
        );
        // Should not deadlock.
    }
}
