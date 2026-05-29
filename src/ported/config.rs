// vim:fileencoding=utf-8:noet
//! Port of `powerline/config.py`.
//!
//! Path-discovery constants used by powerline-status to locate its
//! bundled tmux configs and binding scripts. Computed once at module
//! load time in Python by walking up from `__file__`; in Rust each is
//! a `OnceLock<PathBuf>` initialised lazily on first access via the
//! same-named accessor fns.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use std::path::PathBuf;                              // py:4  import os
use std::sync::OnceLock;

/// Storage backing `POWERLINE_ROOT` (`powerline/config.py:7`).
///
/// Direct module-level binding is impossible because the value depends
/// on `current_exe()` at runtime; accessor `POWERLINE_ROOT()` performs
/// the lazy fill.
#[allow(non_upper_case_globals)]
static POWERLINE_ROOT_CELL: OnceLock<PathBuf> = OnceLock::new();
#[allow(non_upper_case_globals)]
static BINDINGS_DIRECTORY_CELL: OnceLock<PathBuf> = OnceLock::new();
#[allow(non_upper_case_globals)]
static TMUX_CONFIG_DIRECTORY_CELL: OnceLock<PathBuf> = OnceLock::new();
#[allow(non_upper_case_globals)]
static DEFAULT_SYSTEM_CONFIG_DIR_CELL: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Port of module-level binding `POWERLINE_ROOT` from `powerline/config.py:7`.
///
/// Python: `POWERLINE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))`
///
/// Walks two directory levels up from the binary path (Python walks up
/// from `__file__`; for a Rust binary the analogue is `current_exe()`).
/// Falls back to `.` if `current_exe` returns an error.
#[allow(non_snake_case)]
pub fn POWERLINE_ROOT() -> &'static PathBuf {
    POWERLINE_ROOT_CELL.get_or_init(|| {           // py:7
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.parent()).map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("."))
    })
}

/// Port of module-level binding `BINDINGS_DIRECTORY` from `powerline/config.py:8`.
///
/// Python: `BINDINGS_DIRECTORY = os.path.join(POWERLINE_ROOT, 'powerline', 'bindings')`
#[allow(non_snake_case)]
pub fn BINDINGS_DIRECTORY() -> &'static PathBuf {
    BINDINGS_DIRECTORY_CELL.get_or_init(|| {       // py:8
        POWERLINE_ROOT().join("powerline").join("bindings")
    })
}

/// Port of module-level binding `TMUX_CONFIG_DIRECTORY` from `powerline/config.py:9`.
///
/// Python: `TMUX_CONFIG_DIRECTORY = os.path.join(BINDINGS_DIRECTORY, 'tmux')`
#[allow(non_snake_case)]
pub fn TMUX_CONFIG_DIRECTORY() -> &'static PathBuf {
    TMUX_CONFIG_DIRECTORY_CELL.get_or_init(|| {    // py:9
        BINDINGS_DIRECTORY().join("tmux")
    })
}

/// Port of module-level binding `DEFAULT_SYSTEM_CONFIG_DIR` from `powerline/config.py:10`.
///
/// Python literal: `None`. Build-time override slot; downstream packagers
/// (Arch, Debian) patch this to `/etc/xdg/powerline` so a system-wide
/// config dir is searched ahead of the user's. powerliners leaves it
/// `None` by default.
#[allow(non_snake_case)]
pub fn DEFAULT_SYSTEM_CONFIG_DIR() -> &'static Option<PathBuf> {
    DEFAULT_SYSTEM_CONFIG_DIR_CELL.get_or_init(|| None) // py:10
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All four constants are accessible and consistent with the upstream
    /// derivation order.
    #[test]
    fn config_constants_are_consistent() {
        let root = POWERLINE_ROOT();
        let bindings = BINDINGS_DIRECTORY();
        let tmux = TMUX_CONFIG_DIRECTORY();
        let system = DEFAULT_SYSTEM_CONFIG_DIR();

        assert_eq!(bindings, &root.join("powerline").join("bindings"));
        assert_eq!(tmux, &bindings.join("tmux"));
        // Python: `DEFAULT_SYSTEM_CONFIG_DIR = None` at py:10.
        assert!(system.is_none());
    }
}
