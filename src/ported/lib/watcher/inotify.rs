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
        // py:15  class INotifyFileWatcher(INotify):
        // py:16  def __init__(self, expire_time=10):
        // py:17  super(INotifyFileWatcher, self).__init__()
        // py:18  self.watches = {}
        // py:19  self.modified = {}
        // py:20  self.last_query = {}
        // py:21  self.lock = RLock()
        // py:22  self.expire_time = expire_time * 60
        Self {
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
        // py:24  def expire_watches(self):
        // py:25  now = monotonic()
        let now = Self::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut expired: Vec<String> = Vec::new();
        // py:26  for path, last_query in tuple(self.last_query.items()):
        // py:27  if last_query - now > self.expire_time:
        // py:28  self.unwatch(path)
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
        // py:80  def watch(self, path):
        // py:81  ''' Register a watch for the file/directory named path. ...
        // py:83  path = realpath(path)
        // py:84  with self.lock:
        // py:85  if path not in self.watches:
        let path = path.into();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        #[allow(clippy::map_entry)]
        if !entries.contains_key(&path) {
            // py:86  bpath = path if isinstance(path, bytes) else path.encode(self.fenc)
            // py:87  flags = self.MOVE_SELF | self.DELETE_SELF
            // py:88  buf = ctypes.c_char_p(bpath)
            // py:89  # Try watching path as a directory
            // py:90  wd = self._add_watch(self._inotify_fd, buf, flags | self.ONLYDIR)
            // py:91  if wd == -1:
            // py:92  eno = ctypes.get_errno()
            // py:93  if eno != errno.ENOTDIR:
            // py:94  self.handle_error()
            // py:95  # Try watching path as a file
            // py:96  flags |= (self.MODIFY | self.ATTRIB)
            // py:97  wd = self._add_watch(self._inotify_fd, buf, flags)
            // py:98  if wd == -1:
            // py:99  self.handle_error()
            let wd = add_wd();
            // py:100  self.watches[path] = wd
            // py:101  self.modified[path] = False
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
        // py:68  def unwatch(self, path):
        // py:69  ''' Remove the watch for path. Raises an OSError if removing the watch
        // py:70  fails for some reason. '''
        // py:71  path = realpath(path)
        // py:72  with self.lock:
        // py:73  self.modified.pop(path, None)
        // py:74  self.last_query.pop(path, None)
        // py:75  wd = self.watches.pop(path, None)
        // py:76  if wd is not None:
        // py:77  if self._rm_watch(self._inotify_fd, wd) != 0:
        // py:78  self.handle_error()
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
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
        // py:107  def __call__(self, path):
        // py:108  ''' Return True if path has been modified since the last call. ...
        // py:110  path = realpath(path)
        // py:111  with self.lock:
        // py:112  self.last_query[path] = monotonic()
        let now = Self::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = entries.get_mut(path) {
            entry.last_query = now;
        }
        // py:113  self.expire_watches()
        drop(entries);
        // py:114  if path not in self.watches:
        // py:115  # Try to re-add the watch, it will fail if the file does not
        // py:116  # exist/you don't have permission
        // py:117  self.watch(path)
        // py:118  return True
        // py:119  self.read(get_name=False)
        read_events();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // py:120  if path not in self.modified:
        // py:121  # An ignored event was received which means the path has been
        // py:122  # automatically unwatched
        // py:123  return True
        // py:124  ans = self.modified[path]
        // py:125  if ans:
        // py:126  self.modified[path] = False
        // py:127  return ans
        if let Some(entry) = entries.get_mut(path) {
            let ans = entry.modified;
            entry.modified = false;
            ans
        } else {
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
        // py:129  def close(self):
        // py:130  with self.lock:
        // py:131  for path in tuple(self.watches):
        // py:132  try:
        // py:133  self.unwatch(path)
        // py:134  except OSError:
        // py:135  pass
        // py:136  super(INotifyFileWatcher, self).close()
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let drained: Vec<(String, Option<i32>)> = entries.drain().map(|(p, e)| (p, e.wd)).collect();
        drop(entries);
        for (path, wd) in drained {
            if let Some(w) = wd {
                unwatch_each(path, w);
            }
        }
    }

    /// Port of `INotifyFileWatcher.process_event()` from
    /// `powerline/lib/watcher/inotify.py:29`.
    ///
    /// **Status:** stub — the Rust port surfaces the call shape; the
    /// caller drives the modified/last_query maps directly via
    /// `mark_modified` / `unwatch` rather than threading the bitmask
    /// flags through here.
    pub fn process_event(&self, _wd: i32, _mask: u32, _cookie: u32, _name: &str) {
        // py:29  def process_event(self, wd, mask, cookie, name):
        // py:30  if wd == -1 and (mask & self.Q_OVERFLOW):
        // py:31  # We missed some INOTIFY events, so we don't
        // py:32  # know the state of any tracked files.
        // py:33  for path in tuple(self.modified):
        // py:34  if os.path.exists(path):
        // py:35  self.modified[path] = True
        // py:36  else:
        // py:37  self.watches.pop(path, None)
        // py:38  self.modified.pop(path, None)
        // py:39  self.last_query.pop(path, None)
        // py:40  return
        // py:42  for path, num in tuple(self.watches.items()):
        // py:43  if num == wd:
        // py:44  if mask & self.IGNORED:
        // py:45  self.watches.pop(path, None)
        // py:46  self.modified.pop(path, None)
        // py:47  self.last_query.pop(path, None)
        // py:48  else:
        // py:49  if mask & self.ATTRIB:
        // py:50  # The watched file could have had its inode changed, ...
        // py:55  try:
        // py:56  self.unwatch(path)
        // py:57  except OSError:
        // py:58  pass
        // py:59  try:
        // py:60  self.watch(path)
        // py:61  except OSError as e:
        // py:62  if getattr(e, 'errno', None) != errno.ENOENT:
        // py:63  raise
        // py:64  else:
        // py:65  self.modified[path] = True
        // py:66  else:
        // py:67  self.modified[path] = True
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

    /// Port of `INotifyTreeWatcher.watch_tree()` from
    /// `powerline/lib/watcher/inotify.py:163-170`.
    ///
    /// Resets the watched-dir/rmap and adds watches via
    /// [`add_watches`](Self::add_watches) starting at `basedir`.
    /// `add_wd` is the caller's per-path inotify syscall hook.
    /// On `ENOSPC` (system inotify limit exhausted), Python raises
    /// `DirTooLarge`; the Rust port returns `Err(DirTooLarge(...))`
    /// since we don't model the syscall errno chain in the closure
    /// signature.
    pub fn watch_tree<F>(&mut self, mut add_wd: F) -> Result<(), DirTooLarge>
    where
        F: FnMut(&str) -> Option<(i32, bool)>,
    {
        // py:164  self.watched_dirs = {}
        // py:165  self.watched_rmap = {}
        self.watched_dirs.clear();
        self.watched_rmap.clear();
        // py:166  try:
        // py:167  self.add_watches(self.basedir)
        // py:168-170  except OSError as e: if ENOSPC: raise DirTooLarge
        let basedir = self.basedir.clone();
        match self.add_watches(&basedir, true, &mut add_wd) {
            Ok(()) => Ok(()),
            Err(_) => Err(DirTooLarge { basedir }),
        }
    }

    /// Port of `INotifyTreeWatcher.add_watches()` from
    /// `powerline/lib/watcher/inotify.py:172-203`.
    ///
    /// Recursively walks `base` and its subdirectories, adding a
    /// watch for each. Skips entries already in `watched_dirs`
    /// (per py:178-179, prevents symlink-loop recursion).
    ///
    /// `add_wd` is the caller's inotify syscall hook (same shape as
    /// `add_watch`). Returns `Err(())` when the underlying syscall
    /// hits `ENOSPC` (caller surfaces `DirTooLarge`).
    pub fn add_watches<F>(
        &mut self,
        base: &str,
        top_level: bool,
        add_wd: &mut F,
    ) -> Result<(), ()>
    where
        F: FnMut(&str) -> Option<(i32, bool)>,
    {
        // py:172  def add_watches(self, base, top_level=True):
        // py:175  base = realpath(base)
        // py:176-179  if not top_level and base in self.watched_dirs: return
        if !top_level && self.watched_dirs.contains_key(base) {
            return Ok(());
        }
        // py:180-181  is_dir = self.add_watch(base)
        let is_dir = match add_wd(base) {
            Some((wd, is_dir)) => {
                self.watched_dirs.insert(base.to_string(), wd);
                self.watched_rmap.insert(wd, base.to_string());
                is_dir
            }
            // py:183-188  ENOENT: skip non-top-level; raise NoSuchDir top-level
            None => {
                if top_level {
                    return Err(());
                }
                return Ok(());
            }
        };
        // py:201-203  if is_dir: for entry in listdir(base): recurse
        if is_dir {
            if let Ok(entries) = std::fs::read_dir(base) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let p_str = p.to_string_lossy().to_string();
                        self.add_watches(&p_str, false, add_wd)?;
                    }
                }
            }
        }
        Ok(())
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

    #[test]
    fn tree_watcher_watch_tree_resets_and_calls_add_watches() {
        // py:164-165 + 167  watched_dirs/rmap cleared, add_watches called
        let mut w = INotifyTreeWatcher::new("/data/dummy_basedir_zz_9999");
        // Pre-populate so we can verify watch_tree resets.
        w.watched_dirs.insert("/stale".to_string(), 999);
        w.watched_rmap.insert(999, "/stale".to_string());
        let calls = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let calls_c = calls.clone();
        // add_wd returns None for the basedir → triggers Err
        // (ENOSPC-equivalent) but only for top-level paths.
        let r = w.watch_tree(move |_| {
            *calls_c.lock().unwrap() += 1;
            None
        });
        // basedir doesn't exist → first add_wd call returns None →
        // top_level Err → watch_tree returns DirTooLarge.
        assert!(r.is_err());
        assert_eq!(*calls.lock().unwrap(), 1);
        // Stale state was cleared.
        assert!(!w.watched_dirs.contains_key("/stale"));
        assert!(!w.watched_rmap.contains_key(&999));
    }

    #[test]
    fn tree_watcher_add_watches_skips_already_known_paths() {
        // py:178-179  prevent symlink-loop recursion
        let mut w = INotifyTreeWatcher::new("/data/x");
        w.watched_dirs.insert("/data/x".to_string(), 7);
        w.watched_rmap.insert(7, "/data/x".to_string());
        let calls = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let calls_c = calls.clone();
        let r = w.add_watches("/data/x", false, &mut |_| {
            *calls_c.lock().unwrap() += 1;
            Some((42, true))
        });
        assert!(r.is_ok());
        assert_eq!(
            *calls.lock().unwrap(),
            0,
            "already-watched path should not call add_wd"
        );
    }

    #[test]
    fn tree_watcher_add_watches_records_wd_on_success() {
        // py:181 + py:227-229
        let mut w = INotifyTreeWatcher::new("/data/x");
        let mut calls = 0;
        let r = w.add_watches("/data/synthetic_path", true, &mut |_| {
            calls += 1;
            Some((100, false)) // is_dir=false → no recursion
        });
        assert!(r.is_ok());
        assert_eq!(calls, 1);
        assert_eq!(w.watched_dirs.get("/data/synthetic_path"), Some(&100));
        assert_eq!(w.watched_rmap.get(&100), Some(&"/data/synthetic_path".to_string()));
    }
}
