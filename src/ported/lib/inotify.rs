// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/inotify.py`.
//!
//! Upstream is a 185-line ctypes wrapper around Linux's `inotify(7)`
//! syscalls (`inotify_init1`, `inotify_add_watch`, `inotify_rm_watch`)
//! plus a per-instance polling loop. It exists so the higher-level
//! `lib/watcher/inotify.py` `INotifyFileWatcher` can avoid the
//! `pyinotify` dep.
//!
//! Rust port strategy: when the `inotify` Rust crate gets added as a
//! dependency, the real port will land. Until then we expose:
//!   - the `INotifyError` shape so dispatchers can `catch` it
//!   - `INotify` and `load_inotify` shims that always return
//!     `INotifyError`, causing the watcher dispatcher
//!     (`lib/watcher/__init__.py:51-56`) to fall through to the stat
//!     backend on every platform.
//!
//! Behavioural parity: macOS upstream raises `INotifyError("INotify
//! not available on OS X")` immediately (py:37); the stat fallback is
//! the default. Our stub matches that behaviour on every platform.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import os                                        // py:5
// import errno                                     // py:6
// import ctypes                                    // py:7
// import struct                                    // py:8
// from ctypes.util import find_library             // py:10
// from powerline.lib.encoding import get_preferred_file_name_encoding  // py:12

// py:15  __copyright__ = '2013, Kovid Goyal <kovid at kovidgoyal.net>'
// py:16  __docformat__ = 'restructuredtext en'

/// Port of `class INotifyError(Exception)` from
/// `powerline/lib/inotify.py:19`.
///
/// Raised by `load_inotify()` when inotify is unavailable. Callers
/// (the watcher dispatcher) catch this and fall back to stat-based
/// polling.
#[derive(Debug, Clone)]
pub struct INotifyError(pub String);

impl std::fmt::Display for INotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INotifyError: {}", self.0)
    }
}

impl std::error::Error for INotifyError {}

/// Port of `load_inotify()` from `powerline/lib/inotify.py:25`.
///
/// Initialize the inotify library.
///
/// Rust stub: always returns `Err(INotifyError)` so the watcher
/// dispatcher falls back to stat. Matches upstream's macOS branch
/// (py:37) behaviour on every platform.
pub fn load_inotify() -> Result<INotify, INotifyError> {
    // py:30-37  platform checks: windows raises, darwin raises
    // Until the `inotify` crate is added, we treat every platform like
    // the macOS branch.
    Err(INotifyError(
        "INotify support not yet wired in the Rust port — using stat fallback".into(),
    ))
}

/// Port of `class INotify` from `powerline/lib/inotify.py` (the body
/// starts after `load_inotify` returns).
///
/// **Status:** opaque placeholder. The full class needs the `inotify`
/// crate; until then, no instance is ever constructed (`load_inotify`
/// always errors).
pub struct INotify {
    // Hide a zero-sized field so we can't accidentally construct one
    // without going through `load_inotify`.
    _private: (),
}

impl INotify {
    /// Port of `INotify.handle_error()` from
    /// `powerline/lib/inotify.py:133` — placeholder.
    pub fn handle_error(&self) -> Result<(), INotifyError> {
        Err(INotifyError("INotify stub".into()))
    }

    /// Port of `INotify.close()` from
    /// `powerline/lib/inotify.py:149`.
    pub fn close(&self) {
        // py:150-155  close fd, drop ctypes refs — no-op in stub
    }

    /// Port of `INotify.read()` from
    /// `powerline/lib/inotify.py:157`.
    pub fn read(&self, _get_name: bool) -> Vec<(i32, u32, u32, Option<String>)> {
        // py:158-181  poll inotify fd; stub returns no events.
        Vec::new()
    }

    /// Port of `INotify.process_event()` from
    /// `powerline/lib/inotify.py:183` — raises NotImplementedError.
    pub fn process_event(&self, _wd: i32, _mask: u32, _cookie: u32, _name: Option<&str>) {
        // py:184  raise NotImplementedError()
        // Rust analog: panic (this fn is meant to be overridden by
        // subclasses; the stub-level fallthrough is unreachable).
        unimplemented!("INotify.process_event must be overridden");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_inotify_returns_error_in_stub() {
        let r = load_inotify();
        assert!(r.is_err());
    }

    #[test]
    fn inotify_error_implements_display() {
        let e = INotifyError("test".into());
        assert!(e.to_string().contains("INotifyError"));
        assert!(e.to_string().contains("test"));
    }
}
