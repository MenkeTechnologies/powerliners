// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/watcher/uv.py`.
//!
//! pyuv-based file/tree watcher. Upstream depends on the
//! Python `pyuv` library (libuv bindings); the entire file errors out
//! with `UvNotFound` when pyuv isn't installed.
//!
//! Rust analog: the [`notify`](https://crates.io/crates/notify) crate
//! provides equivalent libuv-style filesystem watching, but adding
//! that as a hard dependency is out of scope for this port pass. The
//! Rust port mirrors the structural surface (`UvNotFound`,
//! `UvFileWatcher`, `UvTreeWatcher`) and surfaces `UvNotFound` from
//! the constructor so callers (the watcher dispatcher) fall back to
//! the stat backend on every platform.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from collections import defaultdict              // py:6
// from threading import RLock                      // py:7
// from functools import partial                    // py:8
// from threading import Thread                     // py:9
// from errno import ENOENT                         // py:10
// from powerline.lib.path import realpath          // py:12
// from powerline.lib.encoding import get_preferred_file_name_encoding                     // py:13

/// Port of `class UvNotFound(NotImplementedError)` from
/// `powerline/lib/watcher/uv.py:16`.
///
/// Raised when pyuv is unavailable. The watcher dispatcher catches
/// this and falls back to a different backend.
#[derive(Debug, Clone)]
pub struct UvNotFound;

impl std::fmt::Display for UvNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pyuv (libuv bindings) not available")
    }
}

impl std::error::Error for UvNotFound {}

/// Port of `import_pyuv()` from
/// `powerline/lib/watcher/uv.py:24`.
///
/// Initialize the pyuv binding. Stub always errors since the Rust
/// port doesn't depend on pyuv.
pub fn import_pyuv() -> Result<(), UvNotFound> {
    // py:25-32  try __import__('pyuv') except ImportError: raise UvNotFound
    Err(UvNotFound)
}

/// Port of `class UvFileWatcher` from `powerline/lib/watcher/uv.py`.
///
/// **Status:** stub. Construction always returns `Err(UvNotFound)` so
/// the watcher dispatcher falls through to the stat backend.
pub struct UvFileWatcher;

impl UvFileWatcher {
    /// Constructor that mirrors the upstream's `__init__` failure mode:
    /// always errors out with `UvNotFound`.
    pub fn new() -> Result<Self, UvNotFound> {
        import_pyuv()?;
        Ok(Self)
    }
}

/// Port of `class UvTreeWatcher` from `powerline/lib/watcher/uv.py`.
///
/// **Status:** stub. Same construction-time UvNotFound semantics as
/// `UvFileWatcher`.
pub struct UvTreeWatcher;

impl UvTreeWatcher {
    pub fn new<P: AsRef<std::path::Path>>(_path: P) -> Result<Self, UvNotFound> {
        import_pyuv()?;
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_not_found_implements_error_traits() {
        let e = UvNotFound;
        assert!(e.to_string().contains("pyuv"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn import_pyuv_returns_uv_not_found_in_stub() {
        assert!(import_pyuv().is_err());
    }

    #[test]
    fn uv_file_watcher_new_errors() {
        assert!(UvFileWatcher::new().is_err());
    }

    #[test]
    fn uv_tree_watcher_new_errors() {
        assert!(UvTreeWatcher::new("/tmp").is_err());
    }
}
