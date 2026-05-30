// vim:fileencoding=utf-8:noet
//! Port of `powerline/config.py`.
//!
//! Path-discovery constants used by powerline-status to locate its
//! bundled tmux configs and binding scripts. Computed once at module
//! load time in Python by walking up from `__file__`; in Rust each is
//! a `OnceLock<PathBuf>` initialised lazily on first access via the
//! same-named accessor fns.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use std::path::PathBuf; // py:4  import os
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
    POWERLINE_ROOT_CELL.get_or_init(|| {
        // py:7
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
    BINDINGS_DIRECTORY_CELL.get_or_init(|| {
        // py:8
        POWERLINE_ROOT().join("powerline").join("bindings")
    })
}

/// Port of module-level binding `TMUX_CONFIG_DIRECTORY` from `powerline/config.py:9`.
///
/// Python: `TMUX_CONFIG_DIRECTORY = os.path.join(BINDINGS_DIRECTORY, 'tmux')`
///
/// Upstream Python ships `powerline-base.conf` + version-specific
/// `powerline_tmux_*.conf` files alongside the `bindings/tmux/__init__.py`
/// in the installed package, so the `BINDINGS_DIRECTORY/tmux` derivation
/// at py:9 always finds them.
///
/// In the Rust port the binary may be installed anywhere (`cargo install`
/// to `~/.cargo/bin/`, brew tap to `/opt/homebrew/bin/`, manual `cp` to
/// `/usr/local/bin/`) and the `BINDINGS_DIRECTORY/tmux` derivation rarely
/// points at real `.conf` files. To make tmux setup install-method-
/// agnostic the contents of all 8 conf files are baked into the binary
/// via `include_str!` and extracted to `$XDG_CACHE_HOME/powerliners/tmux/`
/// (or `~/.cache/powerliners/tmux/`) on first call. The cached directory
/// is returned as the canonical `TMUX_CONFIG_DIRECTORY`.
#[allow(non_snake_case)]
pub fn TMUX_CONFIG_DIRECTORY() -> &'static PathBuf {
    TMUX_CONFIG_DIRECTORY_CELL.get_or_init(|| {
        // py:9
        let default = BINDINGS_DIRECTORY().join("tmux");
        if default.join("powerline-base.conf").exists() {
            return default;
        }
        // Bundled `.conf` contents baked into the binary at compile
        // time so the runtime path resolution is install-method-
        // agnostic (works for cargo install, brew, manual cp).
        const BUNDLED: &[(&str, &str)] = &[
            (
                "powerline-base.conf",
                include_str!("bindings/tmux/powerline-base.conf"),
            ),
            (
                "powerline.conf",
                include_str!("bindings/tmux/powerline.conf"),
            ),
            (
                "powerline_tmux_1.7_plus.conf",
                include_str!("bindings/tmux/powerline_tmux_1.7_plus.conf"),
            ),
            (
                "powerline_tmux_1.8.conf",
                include_str!("bindings/tmux/powerline_tmux_1.8.conf"),
            ),
            (
                "powerline_tmux_1.8_minus.conf",
                include_str!("bindings/tmux/powerline_tmux_1.8_minus.conf"),
            ),
            (
                "powerline_tmux_1.8_plus.conf",
                include_str!("bindings/tmux/powerline_tmux_1.8_plus.conf"),
            ),
            (
                "powerline_tmux_1.9_plus.conf",
                include_str!("bindings/tmux/powerline_tmux_1.9_plus.conf"),
            ),
            (
                "powerline_tmux_2.1_plus.conf",
                include_str!("bindings/tmux/powerline_tmux_2.1_plus.conf"),
            ),
        ];
        let cache = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .unwrap_or_else(std::env::temp_dir)
            .join("powerliners")
            .join("tmux");
        if std::fs::create_dir_all(&cache).is_ok() {
            for (name, content) in BUNDLED {
                let target = cache.join(name);
                // Overwrite on every fresh process so an upgraded binary
                // picks up new conf contents — cheap (these are tiny).
                let _ = std::fs::write(&target, content);
            }
        }
        if cache.join("powerline-base.conf").exists() {
            return cache;
        }
        default
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
        // TMUX_CONFIG_DIRECTORY is either the upstream derivation
        // (BINDINGS_DIRECTORY/tmux) when the conf files are present
        // there, or the extracted cache dir under
        // $XDG_CACHE_HOME/powerliners/tmux (~/.cache/powerliners/tmux)
        // when the default doesn't exist. Verify it ends in
        // either '/tmux' OR '/powerliners/tmux' depending on resolution.
        let s = tmux.to_string_lossy();
        assert!(
            s.ends_with("/tmux") || s.ends_with("\\tmux"),
            "TMUX_CONFIG_DIRECTORY should end with /tmux, got {}",
            s
        );
        // Python: `DEFAULT_SYSTEM_CONFIG_DIR = None` at py:10.
        assert!(system.is_none());
    }
}
