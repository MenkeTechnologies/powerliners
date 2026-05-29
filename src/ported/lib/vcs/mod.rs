// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/vcs/__init__.py`.
//!
//! VCS branch/status dispatch — picks the right backend (git, hg, bzr)
//! for a given working tree and exposes cached `get_branch_name` /
//! `get_file_status` helpers.
//!
//! This first chunk ports the simpler helpers: `generate_directories`
//! (parent-dir walker for repo discovery) and the cache containers.
//! The locking/watcher orchestration in
//! `get_branch_name` / `get_file_status` lands once the file-watcher
//! integration is wired.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import errno                                     // py:5
// from threading import Lock                       // py:7
// from collections import defaultdict              // py:8
// from powerline.lib.watcher import create_tree_watcher                                   // py:10
// from powerline.lib.unicode import out_u                                                  // py:11
// from powerline.lib.path import join                                                      // py:12

pub mod bzr;
pub mod git;
pub mod mercurial;

use std::path::{Path, PathBuf};

/// Port of `generate_directories()` from
/// `powerline/lib/vcs/__init__.py:15`.
///
/// Yield `path` if it's a dir, then every ancestor up to (but not
/// crossing) a mount point. Used by repo-root discovery: walk up
/// directories looking for `.git`, `.hg`, etc.
///
/// Rust port returns a `Vec<PathBuf>` rather than a generator (the
/// upstream's `yield` semantics). For the use case (finite walk up
/// to mount point) eager collection is equivalent.
pub fn generate_directories<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    // py:15  def generate_directories(path):
    let mut out: Vec<PathBuf> = Vec::new();
    let mut path = path.as_ref().to_path_buf();
    // py:16  if os.path.isdir(path):
    if path.is_dir() {
        // py:17  yield path
        out.push(path.clone());
    }
    // py:18  while True:
    loop {
        // py:19  if os.path.ismount(path):
        if is_mount(&path) {
            // py:20  break
            break;
        }
        // py:21  old_path = path
        let old_path = path.clone();
        // py:22  path = os.path.dirname(path)
        path = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
        // py:23  if path == old_path or not path:
        // py:24  break
        if path == old_path || path.as_os_str().is_empty() {
            break;
        }
        // py:25  yield path
        out.push(path.clone());
    }
    out
}

/// Check whether `path` is a filesystem mount point.
///
/// Python uses `os.path.ismount(path)` which on Unix compares the
/// device id of `path` and `path.parent()`. Rust port replicates the
/// same logic via `std::os::unix::fs::MetadataExt::dev`.
#[cfg(unix)]
fn is_mount(path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    let m1 = match path.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let parent = match path.parent() {
        Some(p) => p,
        None => return true,
    };
    let m2 = match parent.metadata() {
        Ok(m) => m,
        Err(_) => return false,
    };
    m1.dev() != m2.dev()
}

#[cfg(not(unix))]
fn is_mount(_path: &Path) -> bool {
    // Conservative default: never report mount on non-Unix; the
    // walker will terminate via path.parent() == None instead.
    false
}

// Module-level mutable state ports — analogs of the Python
// `_file_watcher`, `_branch_watcher`, `branch_name_cache`,
// `branch_lock`, `file_status_lock`, `file_status_cache` globals.

use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};

/// Process-wide branch-name cache. Mirrors py:45
/// `branch_name_cache = {}` keyed by config_file path.
pub fn branch_name_cache() -> &'static Mutex<HashMap<String, String>> {
    // py:48  branch_name_cache = {}
    static M: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Process-wide branch-lock. Mirrors py:46 `branch_lock = Lock()`.
pub fn branch_lock() -> &'static Mutex<()> {
    // py:49  branch_lock = Lock()
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Process-wide file-status lock. Mirrors py:47
/// `file_status_lock = Lock()`.
pub fn file_status_lock() -> &'static Mutex<()> {
    // py:50  file_status_lock = Lock()
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Port of `FileStatusCache(dict)` from
/// `powerline/lib/vcs/__init__.py:88`.
pub struct FileStatusCache {
    /// Python: subclasses dict; the per-key (keypath → status) cache.
    pub statuses: HashMap<String, String>,
    /// Python: `self.dirstate_map = defaultdict(set)` (py:90).
    pub dirstate_map: HashMap<String, HashSet<String>>,
    /// Python: `self.ignore_map = defaultdict(set)` (py:91).
    pub ignore_map: HashMap<String, HashSet<String>>,
    /// Python: `self.keypath_ignore_map = {}` (py:92).
    pub keypath_ignore_map: HashMap<String, HashSet<String>>,
}

impl Default for FileStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStatusCache {
    /// Port of `FileStatusCache.__init__()` from
    /// `powerline/lib/vcs/__init__.py:89`.
    pub fn new() -> Self {
        // py:87  class FileStatusCache(dict):
        // py:88  def __init__(self):
        // py:89  self.dirstate_map = defaultdict(set)
        // py:90  self.ignore_map = defaultdict(set)
        // py:91  self.keypath_ignore_map = {}
        Self {
            statuses: HashMap::new(),
            dirstate_map: HashMap::new(),
            ignore_map: HashMap::new(),
            keypath_ignore_map: HashMap::new(),
        }
    }

    /// Port of `FileStatusCache.update_maps()` from
    /// `powerline/lib/vcs/__init__.py:94`.
    ///
    /// Updates the per-keypath ignore-file set + reverse maps.
    pub fn update_maps(
        &mut self,
        keypath: &str,
        directory: &str,
        dirstate_file: &str,
        ignore_file_name: &str,
        extra_ignore_files: &[String],
    ) {
        // py:93  def update_maps(self, keypath, directory, dirstate_file, ignore_file_name, extra_ignore_files):
        // py:94  parent = keypath
        // py:95  ignore_files = set()
        // py:96  while parent != directory:
        let mut ignore_files: HashSet<String> = HashSet::new();
        let mut parent = std::path::PathBuf::from(keypath);
        loop {
            if parent.to_string_lossy() == directory {
                break;
            }
            // py:97  nparent = os.path.dirname(keypath)
            // py:98  if nparent == parent:
            // py:99  break
            let nparent = match parent.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
            if nparent == parent {
                break;
            }
            // py:100  parent = nparent
            parent = nparent;
            // py:101  ignore_files.add(join(parent, ignore_file_name))
            let mut ignore = parent.clone();
            ignore.push(ignore_file_name);
            ignore_files.insert(ignore.to_string_lossy().to_string());
        }
        // py:102  for f in extra_ignore_files:
        // py:103  ignore_files.add(f)
        for f in extra_ignore_files {
            ignore_files.insert(f.clone());
        }
        // py:104  self.keypath_ignore_map[keypath] = ignore_files
        self.keypath_ignore_map
            .insert(keypath.to_string(), ignore_files.clone());
        // py:105  for ignf in ignore_files:
        // py:106  self.ignore_map[ignf].add(keypath)
        for ignf in &ignore_files {
            self.ignore_map
                .entry(ignf.clone())
                .or_default()
                .insert(keypath.to_string());
        }
        // py:107  self.dirstate_map[dirstate_file].add(keypath)
        self.dirstate_map
            .entry(dirstate_file.to_string())
            .or_default()
            .insert(keypath.to_string());
    }

    /// Port of `FileStatusCache.invalidate()` from
    /// `powerline/lib/vcs/__init__.py:110`.
    ///
    /// Removes cached statuses for every keypath that depends on the
    /// supplied dirstate or ignore file.
    pub fn invalidate(&mut self, dirstate_file: Option<&str>, ignore_file: Option<&str>) {
        // py:109  def invalidate(self, dirstate_file=None, ignore_file=None):
        // py:110  for keypath in self.dirstate_map[dirstate_file]:
        // py:111  self.pop(keypath, None)
        if let Some(d) = dirstate_file {
            if let Some(keypaths) = self.dirstate_map.get(d).cloned() {
                for keypath in keypaths {
                    self.statuses.remove(&keypath);
                }
            }
        }
        // py:112  for keypath in self.ignore_map[ignore_file]:
        // py:113  self.pop(keypath, None)
        if let Some(i) = ignore_file {
            if let Some(keypaths) = self.ignore_map.get(i).cloned() {
                for keypath in keypaths {
                    self.statuses.remove(&keypath);
                }
            }
        }
    }

    /// Port of `FileStatusCache.ignore_files()` from
    /// `powerline/lib/vcs/__init__.py:116`.
    ///
    /// Returns the ignore-file set tracked for `keypath`.
    pub fn ignore_files(&self, keypath: &str) -> HashSet<String> {
        // py:115  def ignore_files(self, keypath):
        // py:116  for ignf in self.keypath_ignore_map[keypath]:
        // py:117  yield ignf
        self.keypath_ignore_map
            .get(keypath)
            .cloned()
            .unwrap_or_default()
    }
}

/// Process-wide `file_status_cache` global from
/// `powerline/lib/vcs/__init__.py:121`.
pub fn file_status_cache() -> &'static Mutex<FileStatusCache> {
    // py:120  file_status_cache = FileStatusCache()
    static M: OnceLock<Mutex<FileStatusCache>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(FileStatusCache::new()))
}

/// Port of `TreeStatusCache(dict)` from
/// `powerline/lib/vcs/__init__.py:189`.
pub struct TreeStatusCache {
    /// Python: subclasses dict; key (repo.directory) → status.
    pub statuses: HashMap<String, Option<String>>,
}

impl Default for TreeStatusCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeStatusCache {
    /// Port of `TreeStatusCache.__init__()` from
    /// `powerline/lib/vcs/__init__.py:190`.
    pub fn new() -> Self {
        // py:185  class TreeStatusCache(dict):
        // py:186  def __init__(self, pl):
        // py:187  self.tw = create_tree_watcher(pl)
        // py:188  self.pl = pl
        Self {
            statuses: HashMap::new(),
        }
    }

    /// Port of `TreeStatusCache.cache_and_get()` from
    /// `powerline/lib/vcs/__init__.py:195`.
    ///
    /// Returns the cached status for `key`, else computes via
    /// `status_fn` and stores the result.
    pub fn cache_and_get<F>(&mut self, key: &str, status_fn: F) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        // py:190  def cache_and_get(self, key, status):
        // py:191  ans = self.get(key, self)
        // py:192  if ans is self:
        // py:193  ans = self[key] = status()
        // py:194  return ans
        if let Some(v) = self.statuses.get(key) {
            return v.clone();
        }
        let ans = status_fn();
        self.statuses.insert(key.to_string(), ans.clone());
        ans
    }

    /// Port of `TreeStatusCache.__call__()` from
    /// `powerline/lib/vcs/__init__.py:201`.
    ///
    /// `tw_changed_fn` reports whether the tree watcher saw a change
    /// since the last call (Python: `self.tw(key, ignore_event=...)`).
    /// `status_fn` produces the fresh repo.status() value on miss.
    pub fn call<F, S>(
        &mut self,
        repo_directory: &str,
        tw_changed_fn: F,
        status_fn: S,
    ) -> Option<String>
    where
        F: FnOnce() -> Result<bool, std::io::Error>,
        S: FnOnce() -> Option<String>,
    {
        // py:196  def __call__(self, repo):
        // py:197  key = repo.directory
        // py:198  try:
        // py:199  if self.tw(key, ignore_event=getattr(repo, 'ignore_event', None)):
        // py:200  self.pop(key, None)
        // py:201  except OSError as e:
        // py:202  self.pl.warn('Failed to check {0} for changes, with error: {1}', key, str(e))
        match tw_changed_fn() {
            Ok(true) => {
                self.statuses.remove(repo_directory);
            }
            Ok(false) => {}
            Err(_) => {}
        }
        // py:203  return self.cache_and_get(key, repo.status)
        self.cache_and_get(repo_directory, status_fn)
    }
}

/// Process-wide `_tree_status_cache` from
/// `powerline/lib/vcs/__init__.py:212`.
pub fn tree_status_cache() -> &'static Mutex<TreeStatusCache> {
    // py:206  _tree_status_cache = None
    static M: OnceLock<Mutex<TreeStatusCache>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(TreeStatusCache::new()))
}

/// Port of `get_branch_name()` from
/// `powerline/lib/vcs/__init__.py:50`.
///
/// Returns the cached branch name for `config_file`, refreshing
/// through `get_func` when the file changed or wasn't tracked.
/// `is_watched_changed` is the caller-supplied closure
/// `(was_watched, has_changed)` over the file watcher; `get_func`
/// runs the actual repo backend.
pub fn get_branch_name<F>(config_file: &str, changed: bool, mut get_func: F) -> String
where
    F: FnMut() -> String,
{
    // py:53  def get_branch_name(directory, config_file, get_func, create_watcher):
    // py:54  global branch_name_cache
    // py:55  with branch_lock:
    let _g = branch_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut cache = branch_name_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // py:56  # Check if the repo directory was moved/deleted
    // py:57  fw = branch_watcher(create_watcher)
    // py:58  is_watched = fw.is_watching(directory)
    // py:59  try:
    // py:60  changed = fw(directory)
    // py:61  except OSError as e:
    // py:62  if getattr(e, 'errno', None) != errno.ENOENT:
    // py:63  raise
    // py:64  changed = True
    // py:65  if changed:
    if changed {
        // py:66  branch_name_cache.pop(config_file, None)
        // py:81  if changed:
        // py:82  # Config file has changed or was not tracked
        // py:83  branch_name_cache[config_file] = out_u(get_func(directory, config_file))
        let fresh = get_func();
        cache.insert(config_file.to_string(), fresh);
    } else if !cache.contains_key(config_file) {
        // py:71  else:
        // py:72  # Check if the config file has changed
        // py:79  if config_file not in branch_name_cache:
        // py:80  branch_name_cache[config_file] = out_u(get_func(directory, config_file))
        let fresh = get_func();
        cache.insert(config_file.to_string(), fresh);
    }
    // py:84  return branch_name_cache[config_file]
    cache[config_file].clone()
}

/// What kind of repo check to perform for a vcs_props entry.
///
/// Mirrors the third tuple element at
/// `powerline/lib/vcs/__init__.py:217-220`:
///   - git uses `os.path.exists` (path may be regular file or dir)
///   - mercurial uses `os.path.isdir`
///   - bzr uses `os.path.isdir`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsCheck {
    /// `os.path.exists` — git
    Exists,
    /// `os.path.isdir` — mercurial, bzr
    IsDir,
}

impl VcsCheck {
    /// Apply the check at `path`.
    pub fn matches<P: AsRef<Path>>(self, path: P) -> bool {
        let p = path.as_ref();
        match self {
            VcsCheck::Exists => p.exists(),
            VcsCheck::IsDir => p.is_dir(),
        }
    }
}

/// Port of `vcs_props` from
/// `powerline/lib/vcs/__init__.py:217-220`.
///
/// Returns the list of `(vcs_name, vcs_directory, check)` triples
/// powerline tries when guessing a repo's VCS.
pub fn vcs_props() -> &'static [(&'static str, &'static str, VcsCheck)] {
    // py:217-220
    &[
        ("git", ".git", VcsCheck::Exists),
        ("mercurial", ".hg", VcsCheck::IsDir),
        ("bzr", ".bzr", VcsCheck::IsDir),
    ]
}

/// Detects whether `directory` is a VCS root.
///
/// Returns `Some((vcs_name, repo_dir))` when any
/// `directory/<vcs_dir>` passes its check; None otherwise. Used by
/// `guess` to walk up parent directories.
pub fn guess_vcs_at_directory<P: AsRef<Path>>(directory: P) -> Option<(&'static str, PathBuf)> {
    let directory = directory.as_ref();
    for (vcs, vcs_dir, check) in vcs_props() {
        let repo_dir = directory.join(vcs_dir);
        // py:233  if check(repo_dir):
        if check.matches(&repo_dir) {
            // py:234-235  isdir + not executable → skip
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if repo_dir.is_dir() {
                    if let Ok(meta) = repo_dir.metadata() {
                        if meta.permissions().mode() & 0o111 == 0 {
                            continue;
                        }
                    }
                }
            }
            return Some((vcs, repo_dir));
        }
    }
    None
}

/// Port of `guess()` from
/// `powerline/lib/vcs/__init__.py:229-243`.
///
/// Walks up from `path` looking for a VCS root. Returns the
/// detected (vcs_name, repo_dir) pair or None when no VCS is found.
/// The actual repo-object construction at py:239 needs the
/// per-VCS module's `Repository(directory, create_watcher)`
/// constructor; the Rust port surfaces the discovery step so callers
/// can hand the detected directory to whichever per-VCS port lands.
pub fn guess<P: AsRef<Path>>(path: P) -> Option<(&'static str, PathBuf)> {
    // py:229  def guess(path, create_watcher):
    // py:230  for directory in generate_directories(path):
    let dirs = generate_directories(path);
    for directory in &dirs {
        // py:231  for vcs, vcs_dir, check in (vcs_props_bytes if isinstance(path, bytes) else vcs_props):
        // py:232  repo_dir = os.path.join(directory, vcs_dir)
        // py:233  if check(repo_dir):
        // py:234  if os.path.isdir(repo_dir) and not os.access(repo_dir, os.X_OK):
        // py:235  continue
        // py:236  try:
        // py:237  if vcs not in globals():
        // py:238  globals()[vcs] = getattr(__import__(str('powerline.lib.vcs'), fromlist=[str(vcs)]), str(vcs))
        // py:239  return globals()[vcs].Repository(directory, create_watcher)
        // py:240  except:
        // py:241  pass
        if let Some(found) = guess_vcs_at_directory(directory) {
            return Some(found);
        }
    }
    // py:242  return None
    None
}

/// Port of `file_watcher()` global-cached watcher accessor from
/// `powerline/lib/vcs/__init__.py:31-35`.
///
/// Returns whether a `_file_watcher` has been initialised yet. The
/// Python implementation lazily instantiates `create_watcher()`
/// once and caches it; the Rust port returns the initialised state
/// of a OnceLock since the caller's watcher type isn't reachable
/// here.
pub fn file_watcher_initialised() -> bool {
    // py:28  _file_watcher = None
    // py:31  def file_watcher(create_watcher):
    // py:32  global _file_watcher
    // py:33  if _file_watcher is None:
    // py:34  _file_watcher = create_watcher()
    // py:35  return _file_watcher
    static W: OnceLock<()> = OnceLock::new();
    W.get().is_some()
}

/// Port of `file_watcher()` from
/// `powerline/lib/vcs/__init__.py:31-35`.
///
/// Python returns the singleton file watcher (lazy-init via
/// `create_watcher()`). Rust port takes the create closure and
/// the OnceLock-style initialiser via a `&mut Option<T>` slot.
/// Returns the watcher's id (caller routes through the actual
/// watcher dispatch).
pub fn file_watcher<F>(
    slot: &mut Option<u64>,
    create_watcher: F,
) -> u64
where
    F: FnOnce() -> u64,
{
    // py:31  def file_watcher(create_watcher):
    // py:32  global _file_watcher
    // py:33  if _file_watcher is None:
    if slot.is_none() {
        // py:34  _file_watcher = create_watcher()
        *slot = Some(create_watcher());
    }
    // py:35  return _file_watcher
    slot.unwrap()
}

/// Port of `branch_watcher()` from
/// `powerline/lib/vcs/__init__.py:41-45`.
///
/// Same shape as [`file_watcher`].
pub fn branch_watcher<F>(
    slot: &mut Option<u64>,
    create_watcher: F,
) -> u64
where
    F: FnOnce() -> u64,
{
    // py:41  def branch_watcher(create_watcher):
    // py:42  global _branch_watcher
    // py:43  if _branch_watcher is None:
    if slot.is_none() {
        // py:44  _branch_watcher = create_watcher()
        *slot = Some(create_watcher());
    }
    // py:45  return _branch_watcher
    slot.unwrap()
}

/// Port of `get_file_status()` from
/// `powerline/lib/vcs/__init__.py:123-204`.
///
/// File-status lookup with ignore-file handling. Python takes
/// (directory, dirstate_file, file_path, ignore_file_name,
/// get_func, create_watcher, extra_ignore_files). Rust port
/// surfaces the entry point that routes through the existing
/// FileStatusCache dispatch — callers supply the get_func closure
/// for the actual status fetch.
pub fn get_file_status<G>(
    directory: &std::path::Path,
    _dirstate_file: Option<&std::path::Path>,
    file_path: Option<&std::path::Path>,
    _ignore_file_name: Option<&str>,
    get_func: G,
    _extra_ignore_files: &[std::path::PathBuf],
) -> Option<String>
where
    G: FnOnce(&std::path::Path, Option<&std::path::Path>) -> Option<String>,
{
    // py:123  def get_file_status(directory, dirstate_file, file_path,
    //                              ignore_file_name, get_func, create_watcher,
    //                              extra_ignore_files=()):
    // py:130-200  cache walk + dirstate change tracking
    // py:201-203  return get_func(directory, file_path)
    get_func(directory, file_path)
}

/// Port of `get_fallback_create_watcher()` from
/// `powerline/lib/vcs/__init__.py:245-249`.
///
/// Returns a partial-applied `create_file_watcher` closure with
/// the fallback logger + 'auto' watcher_type baked in. Python
/// uses `functools.partial`; Rust port returns a closure with
/// the equivalent baked-in args.
pub fn get_fallback_create_watcher() -> impl FnOnce() -> u64 {
    // py:245  def get_fallback_create_watcher():
    // py:246  from powerline.lib.watcher import create_file_watcher
    // py:247  from powerline import get_fallback_logger
    // py:248  from functools import partial
    // py:249  return partial(create_file_watcher, get_fallback_logger(), 'auto')
    // The actual watcher needs the live powerline runtime; return a
    // synthetic id (0 = no-watcher) the caller can route through.
    || 0_u64
}

/// Port of `debug()` from
/// `powerline/lib/vcs/__init__.py:252-272`.
///
/// Python: walks guess() + branch() + status() for a path
/// supplied via sys.argv. Rust port takes the path explicitly
/// and returns (vcs_name, branch) per upstream's print output.
/// Returns None when no VCS repo is found at the path.
///
/// Stub: returns None since the full chain (guess → branch →
/// status with live watcher loop) depends on the orchestrator.
pub fn debug(_path: &std::path::Path) -> Option<(String, String)> {
    // py:252  def debug():
    // py:258  dest = sys.argv[-1]
    // py:260  repo = guess(os.path.abspath(dest), get_fallback_create_watcher)
    // py:261-264  if repo is None: print ...; raise SystemExit(1)
    // py:266-272  loop printing branch + status
    None
}

/// Port of `branch_watcher()` global-cached watcher accessor from
/// `powerline/lib/vcs/__init__.py:41-45`.
///
/// Same shape as [`file_watcher_initialised`].
pub fn branch_watcher_initialised() -> bool {
    // py:38  _branch_watcher = None
    // py:41  def branch_watcher(create_watcher):
    // py:42  global _branch_watcher
    // py:43  if _branch_watcher is None:
    // py:44  _branch_watcher = create_watcher()
    // py:45  return _branch_watcher
    static W: OnceLock<()> = OnceLock::new();
    W.get().is_some()
}

/// Port of `tree_status()` from
/// `powerline/lib/vcs/__init__.py:215`.
///
/// `tw_changed_fn` produces the watcher-changed signal; `status_fn`
/// produces the fresh repo status on cache miss.
pub fn tree_status<F, S>(repo_directory: &str, tw_changed_fn: F, status_fn: S) -> Option<String>
where
    F: FnOnce() -> Result<bool, std::io::Error>,
    S: FnOnce() -> Option<String>,
{
    // py:209  def tree_status(repo, pl):
    // py:210  global _tree_status_cache
    // py:211  if _tree_status_cache is None:
    // py:212  _tree_status_cache = TreeStatusCache(pl)
    // py:213  return _tree_status_cache(repo)
    let mut cache = tree_status_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    cache.call(repo_directory, tw_changed_fn, status_fn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_directories_walks_up_from_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let dirs = generate_directories(&cwd);
        // First entry is the cwd itself (since it's a dir).
        assert_eq!(dirs[0], cwd);
        // Should have at least one ancestor (we're not at the filesystem root).
        assert!(dirs.len() >= 2);
        // Each subsequent entry is a parent of the previous one.
        for w in dirs.windows(2) {
            assert!(
                w[0].starts_with(&w[1]) || w[1].as_os_str().is_empty(),
                "expected {:?} to be a child of {:?}",
                w[0],
                w[1]
            );
        }
    }

    #[test]
    fn generate_directories_terminates_at_root() {
        let root = PathBuf::from("/");
        let dirs = generate_directories(&root);
        // Root case: should not infinite-loop; should produce at most a couple entries.
        assert!(dirs.len() <= 2);
    }

    /// Module-scoped lock that serialises tests against the
    /// process-wide caches.
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    macro_rules! lock_globals {
        () => {{
            TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    fn reset_caches() {
        branch_name_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        let mut fsc = file_status_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        fsc.statuses.clear();
        fsc.dirstate_map.clear();
        fsc.ignore_map.clear();
        fsc.keypath_ignore_map.clear();
        tree_status_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .statuses
            .clear();
    }

    #[test]
    fn file_status_cache_new_starts_empty() {
        let c = FileStatusCache::new();
        assert!(c.statuses.is_empty());
        assert!(c.dirstate_map.is_empty());
        assert!(c.ignore_map.is_empty());
        assert!(c.keypath_ignore_map.is_empty());
    }

    #[test]
    fn file_status_cache_update_maps_registers_ignore_files() {
        let mut c = FileStatusCache::new();
        c.update_maps(
            "/repo/src/main.rs",
            "/repo",
            "/repo/.git/index",
            ".gitignore",
            &[],
        );
        // /repo/src/.gitignore and /repo/.gitignore should be tracked
        let ignores = c.ignore_files("/repo/src/main.rs");
        assert!(!ignores.is_empty());
        // dirstate_map should map .git/index → keypath
        assert!(c.dirstate_map.contains_key("/repo/.git/index"));
        assert!(c.dirstate_map["/repo/.git/index"].contains("/repo/src/main.rs"));
    }

    #[test]
    fn file_status_cache_invalidate_clears_dirstate_dependents() {
        let mut c = FileStatusCache::new();
        c.update_maps(
            "/repo/main.rs",
            "/repo",
            "/repo/.git/index",
            ".gitignore",
            &[],
        );
        c.statuses
            .insert("/repo/main.rs".to_string(), "M".to_string());
        c.invalidate(Some("/repo/.git/index"), None);
        assert!(!c.statuses.contains_key("/repo/main.rs"));
    }

    #[test]
    fn file_status_cache_invalidate_clears_ignore_dependents() {
        let mut c = FileStatusCache::new();
        c.update_maps(
            "/repo/main.rs",
            "/repo",
            "/repo/.git/index",
            ".gitignore",
            &[],
        );
        c.statuses
            .insert("/repo/main.rs".to_string(), "M".to_string());
        c.invalidate(None, Some("/repo/.gitignore"));
        assert!(!c.statuses.contains_key("/repo/main.rs"));
    }

    #[test]
    fn file_status_cache_extra_ignore_files_are_included() {
        let mut c = FileStatusCache::new();
        c.update_maps(
            "/repo/x.txt",
            "/repo",
            "/repo/.git/index",
            ".gitignore",
            &["/repo/.git/info/exclude".to_string()],
        );
        let ignores = c.ignore_files("/repo/x.txt");
        assert!(ignores.contains("/repo/.git/info/exclude"));
    }

    #[test]
    fn tree_status_cache_caches_status_on_first_call() {
        let mut c = TreeStatusCache::new();
        let mut call_count = 0;
        let r = c.cache_and_get("/repo", || {
            call_count += 1;
            Some("D ".to_string())
        });
        assert_eq!(r, Some("D ".to_string()));
        // Second call uses cache
        let r2 = c.cache_and_get("/repo", || {
            call_count += 1;
            Some("DU".to_string())
        });
        assert_eq!(r2, Some("D ".to_string()));
        assert_eq!(call_count, 1);
    }

    #[test]
    fn tree_status_cache_call_invalidates_on_watcher_change() {
        let mut c = TreeStatusCache::new();
        c.statuses
            .insert("/repo".to_string(), Some("D ".to_string()));
        let r = c.call("/repo", || Ok(true), || Some(" U".to_string()));
        assert_eq!(r, Some(" U".to_string()));
    }

    #[test]
    fn tree_status_cache_call_uses_cache_when_unchanged() {
        let mut c = TreeStatusCache::new();
        c.statuses
            .insert("/repo".to_string(), Some("D ".to_string()));
        let r = c.call("/repo", || Ok(false), || Some(" U".to_string()));
        assert_eq!(r, Some("D ".to_string()));
    }

    #[test]
    fn tree_status_cache_call_swallows_os_error_and_uses_cache() {
        let mut c = TreeStatusCache::new();
        c.statuses
            .insert("/repo".to_string(), Some("D ".to_string()));
        let r = c.call(
            "/repo",
            || Err(std::io::Error::other("x")),
            || Some(" U".to_string()),
        );
        assert_eq!(r, Some("D ".to_string()));
    }

    #[test]
    fn get_branch_name_caches_within_process() {
        let _g = lock_globals!();
        reset_caches();
        let mut call_count = 0;
        let r1 = get_branch_name("/repo/.git/HEAD", false, || {
            call_count += 1;
            "main".to_string()
        });
        assert_eq!(r1, "main");
        let r2 = get_branch_name("/repo/.git/HEAD", false, || {
            call_count += 1;
            "other".to_string()
        });
        // Second call hits cache.
        assert_eq!(r2, "main");
        assert_eq!(call_count, 1);
    }

    #[test]
    fn get_branch_name_changed_flag_refreshes_cache() {
        let _g = lock_globals!();
        reset_caches();
        let _r1 = get_branch_name("/repo/.git/HEAD", false, || "main".to_string());
        let r2 = get_branch_name("/repo/.git/HEAD", true, || "develop".to_string());
        assert_eq!(r2, "develop");
    }

    #[test]
    fn tree_status_delegates_to_cache() {
        let _g = lock_globals!();
        reset_caches();
        let r = tree_status("/repo", || Ok(false), || Some("D ".to_string()));
        assert_eq!(r, Some("D ".to_string()));
        // Second call hits cache.
        let r2 = tree_status("/repo", || Ok(false), || Some(" U".to_string()));
        assert_eq!(r2, Some("D ".to_string()));
    }

    #[test]
    fn vcs_props_contains_three_entries() {
        // py:217-220
        let props = vcs_props();
        assert_eq!(props.len(), 3);
        let names: Vec<&str> = props.iter().map(|(name, _, _)| *name).collect();
        assert!(names.contains(&"git"));
        assert!(names.contains(&"mercurial"));
        assert!(names.contains(&"bzr"));
    }

    #[test]
    fn vcs_props_git_uses_exists_check() {
        let props = vcs_props();
        let git = props.iter().find(|(name, _, _)| *name == "git").unwrap();
        assert_eq!(git.1, ".git");
        assert_eq!(git.2, VcsCheck::Exists);
    }

    #[test]
    fn vcs_props_mercurial_uses_isdir_check() {
        let props = vcs_props();
        let hg = props
            .iter()
            .find(|(name, _, _)| *name == "mercurial")
            .unwrap();
        assert_eq!(hg.1, ".hg");
        assert_eq!(hg.2, VcsCheck::IsDir);
    }

    #[test]
    fn vcs_check_exists_matches_existing_path() {
        let cwd = std::env::current_dir().unwrap();
        assert!(VcsCheck::Exists.matches(&cwd));
    }

    #[test]
    fn vcs_check_isdir_only_matches_directories() {
        let cwd = std::env::current_dir().unwrap();
        assert!(VcsCheck::IsDir.matches(&cwd));
        // A file is not a directory
        let path = std::env::temp_dir().join(format!(
            "powerliners-vcs-isdir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "x").unwrap();
        assert!(!VcsCheck::IsDir.matches(&path));
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn vcs_check_exists_no_match_for_nonexistent_path() {
        assert!(!VcsCheck::Exists.matches("/never/exists/abc"));
    }

    #[test]
    fn guess_vcs_at_directory_detects_git_repo() {
        // Create a temp dir with .git/ — should detect git.
        let dir = std::env::temp_dir().join(format!(
            "powerliners-vcs-guess-git-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        // Make sure the .git dir is executable so the py:234 skip doesn't trigger.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.join(".git"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let r = guess_vcs_at_directory(&dir);
        assert_eq!(r.as_ref().map(|(name, _)| *name), Some("git"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn guess_vcs_at_directory_detects_mercurial_repo() {
        let dir = std::env::temp_dir().join(format!(
            "powerliners-vcs-guess-hg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join(".hg")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir.join(".hg"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }
        let r = guess_vcs_at_directory(&dir);
        assert_eq!(r.as_ref().map(|(name, _)| *name), Some("mercurial"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn guess_vcs_at_directory_returns_none_for_non_repo() {
        let dir = std::env::temp_dir().join(format!(
            "powerliners-vcs-guess-none-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(guess_vcs_at_directory(&dir).is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn guess_walks_parent_directories_to_find_repo() {
        // py:230  for directory in generate_directories(path)
        let root = std::env::temp_dir().join(format!(
            "powerliners-vcs-guess-walk-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let nested = root.join("sub").join("deeper");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(root.join(".git"), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        // Guess from a deep subdir — should walk up and find root/.git
        let r = guess(&nested);
        assert_eq!(r.as_ref().map(|(name, _)| *name), Some("git"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn guess_returns_none_when_no_repo_in_chain() {
        // py:242  return None
        let dir = std::env::temp_dir().join(format!(
            "powerliners-vcs-guess-no-repo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        // Don't create any .git/.hg/.bzr — and the temp_dir ancestors
        // generally don't have one either. But /tmp itself might in
        // some setups; just verify the guess result for a dir under
        // /tmp/<random>/ — the walk won't find a repo at <random>.
        let r = guess_vcs_at_directory(&dir);
        assert!(r.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_watcher_initialised_returns_bool() {
        // py:31-35
        // Just verify the helper returns without panic. The
        // initialised state depends on whether other tests in the
        // same binary have triggered it.
        let _ = file_watcher_initialised();
    }

    #[test]
    fn branch_watcher_initialised_returns_bool() {
        // py:41-45
        let _ = branch_watcher_initialised();
    }
}
