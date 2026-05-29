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

// Module-level mutable state (`_file_watcher`, `_branch_watcher`,
// `branch_name_cache`, `branch_lock`, `file_status_lock`,
// `file_status_cache`) is deferred — these are bucket-2 shared
// state per PORT_PLAN.md and land alongside the
// `get_branch_name`/`get_file_status` orchestrators.

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
}
