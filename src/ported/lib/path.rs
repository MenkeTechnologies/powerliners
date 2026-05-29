// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/path.py`.
//!
//! Two path-manipulation helpers used by powerline-status's config
//! loader. Both wrap stdlib path operations with no powerline-specific
//! logic — straightforward 1:1 ports.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use std::path::{Path, PathBuf}; // py:4  import os

/// Port of `realpath()` from `powerline/lib/path.py:7`.
///
/// Python:
/// ```python
/// def realpath(path):
///     return os.path.abspath(os.path.realpath(path))
/// ```
///
/// `os.path.realpath` follows symlinks; `os.path.abspath` makes the
/// result absolute. `std::fs::canonicalize` does both atomically and
/// returns an error if the path does not exist; Python's chain works
/// on non-existent paths too (resolving as much as it can). For
/// missing paths we fall back to manual abspath() — matching Python's
/// best-effort behaviour.
pub fn realpath<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        // py:8  fallback when path doesn't yet exist on disk
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    })
}

/// Port of `join()` from `powerline/lib/path.py:11`.
///
/// Python:
/// ```python
/// def join(*components):
///     if any((isinstance(p, bytes) for p in components)):
///         return os.path.join(*[
///             p if isinstance(p, bytes) else p.encode('ascii')
///             for p in components
///         ])
///     else:
///         return os.path.join(*components)
/// ```
///
/// The bytes/str distinction matters in Python 2/3 — joining a `str`
/// and a `bytes` path raises `TypeError`. Rust paths are byte-oriented
/// on Unix (`OsStr` is `[u8]`-shaped); the unified port accepts any
/// path-like input and joins them with platform separators.
pub fn join<I, P>(components: I) -> PathBuf
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut iter = components.into_iter(); // py:11
    let mut acc = match iter.next() {
        Some(first) => first.as_ref().to_path_buf(),
        None => return PathBuf::new(),
    };
    for c in iter {
        // py:13-16  os.path.join over remaining components
        acc.push(c.as_ref());
    }
    acc // py:13 / py:18
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realpath_of_cwd_is_absolute() {
        let p = realpath(".");
        assert!(
            p.is_absolute(),
            "realpath('.') should be absolute, got {:?}",
            p
        );
    }

    #[test]
    fn realpath_of_missing_path_is_absolute_join_with_cwd() {
        let p = realpath("does-not-exist-powerliners-test");
        assert!(
            p.is_absolute(),
            "realpath of missing path should still be absolute, got {:?}",
            p
        );
    }

    /// `join(['a', 'b', 'c'])` == `'a/b/c'` on Unix.
    #[test]
    fn join_concatenates_components_in_order() {
        let p = join(["a", "b", "c"]);
        assert_eq!(p, PathBuf::from("a").join("b").join("c"));
    }

    /// Empty input returns empty path (matches Python: `os.path.join()` raises but
    /// we lean toward "useful default" — empty path is the no-op identity).
    #[test]
    fn join_empty_returns_empty_path() {
        let p: PathBuf = join(Vec::<&str>::new());
        assert_eq!(p, PathBuf::new());
    }

    /// An absolute component anywhere in the list resets the accumulator —
    /// matches `os.path.join('a', '/b', 'c')` == `'/b/c'`.
    #[test]
    fn join_absolute_component_resets() {
        let p = join(["a", "/b", "c"]);
        assert_eq!(p, PathBuf::from("/b").join("c"));
    }
}
