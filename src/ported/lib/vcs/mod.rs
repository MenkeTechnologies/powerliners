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
    let mut out: Vec<PathBuf> = Vec::new();
    let mut path = path.as_ref().to_path_buf();
    if path.is_dir() {
        // py:16
        out.push(path.clone()); // py:17
    }
    loop {
        // py:18
        if is_mount(&path) {
            // py:19
            break; // py:20
        }
        let old_path = path.clone();
        // py:22  path = os.path.dirname(path)
        path = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
        // py:23-24  if path == old_path or not path: break
        if path == old_path || path.as_os_str().is_empty() {
            break;
        }
        out.push(path.clone()); // py:25  yield path
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
    static M: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Process-wide branch-lock. Mirrors py:46 `branch_lock = Lock()`.
pub fn branch_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Process-wide file-status lock. Mirrors py:47
/// `file_status_lock = Lock()`.
pub fn file_status_lock() -> &'static Mutex<()> {
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
        // py:95-104  walk parents up to `directory`, collecting
        // <parent>/<ignore_file_name>
        let mut ignore_files: HashSet<String> = HashSet::new();
        let mut parent = std::path::PathBuf::from(keypath);
        loop {
            if parent.to_string_lossy() == directory {
                break;
            }
            // py:97-100  nparent = dirname(keypath); if nparent == parent: break
            let nparent = match parent.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
            if nparent == parent {
                break;
            }
            parent = nparent;
            // py:101  ignore_files.add(join(parent, ignore_file_name))
            let mut ignore = parent.clone();
            ignore.push(ignore_file_name);
            ignore_files.insert(ignore.to_string_lossy().to_string());
        }
        // py:102-103  extra_ignore_files
        for f in extra_ignore_files {
            ignore_files.insert(f.clone());
        }
        // py:105  self.keypath_ignore_map[keypath] = ignore_files
        self.keypath_ignore_map
            .insert(keypath.to_string(), ignore_files.clone());
        // py:106-107  ignore_map[ignf].add(keypath)
        for ignf in &ignore_files {
            self.ignore_map
                .entry(ignf.clone())
                .or_default()
                .insert(keypath.to_string());
        }
        // py:108  dirstate_map[dirstate_file].add(keypath)
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
        // py:111-112  dirstate_file path
        if let Some(d) = dirstate_file {
            if let Some(keypaths) = self.dirstate_map.get(d).cloned() {
                for keypath in keypaths {
                    self.statuses.remove(&keypath);
                }
            }
        }
        // py:113-114  ignore_file path
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
        // py:117-118  yield from keypath_ignore_map[keypath]
        self.keypath_ignore_map
            .get(keypath)
            .cloned()
            .unwrap_or_default()
    }
}

/// Process-wide `file_status_cache` global from
/// `powerline/lib/vcs/__init__.py:121`.
pub fn file_status_cache() -> &'static Mutex<FileStatusCache> {
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
        // py:196-199  ans = self.get(key, self); if ans is self: ans = status()
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
        // py:202-208  if tw(key): self.pop(key); except OSError: warn
        match tw_changed_fn() {
            Ok(true) => {
                self.statuses.remove(repo_directory);
            }
            Ok(false) => {}
            Err(_) => {
                // py:206-207  pl.warn(...) — Rust port omits the log
            }
        }
        // py:209  return self.cache_and_get(key, repo.status)
        self.cache_and_get(repo_directory, status_fn)
    }
}

/// Process-wide `_tree_status_cache` from
/// `powerline/lib/vcs/__init__.py:212`.
pub fn tree_status_cache() -> &'static Mutex<TreeStatusCache> {
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
    // py:51-53  with branch_lock: ...
    let _g = branch_lock().lock().unwrap_or_else(|e| e.into_inner());
    let mut cache = branch_name_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // py:62-66  changed path
    if changed {
        // py:62  branch_name_cache.pop(config_file, None)
        // py:84  branch_name_cache[config_file] = get_func(...)
        let fresh = get_func();
        cache.insert(config_file.to_string(), fresh);
    } else if !cache.contains_key(config_file) {
        // py:80  if config_file not in branch_name_cache: get_func
        let fresh = get_func();
        cache.insert(config_file.to_string(), fresh);
    }
    cache[config_file].clone()
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
    // py:216-219  initialise + delegate to TreeStatusCache.__call__
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
            || Err(std::io::Error::new(std::io::ErrorKind::Other, "x")),
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
}
