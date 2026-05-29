// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/config.py`.
//!
//! Powerline binding-side config helpers: tmux config file discovery
//! and version-matching. The orchestrator helpers (`source_tmux_files`,
//! `init_tmux_environment`, `tmux_setup`, `get_main_config`,
//! `create_powerline_logger`, `deduce_command`, `shell_command`,
//! `uses`) all depend on the full `Powerline` class + `ConfigLoader`
//! and land alongside `powerline/__init__.py`.
//!
//! This first chunk ports the leaf helpers — `list_all_tmux_configs`,
//! `get_tmux_configs`, plus the three module-level constants.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// import sys                                       // py:6
// import subprocess                                // py:7
// import shlex                                     // py:8
// from powerline.config import POWERLINE_ROOT, TMUX_CONFIG_DIRECTORY                       // py:10
// from powerline.lib.config import ConfigLoader                                              // py:11
// from powerline import ...                                                                  // py:12
// from powerline.shell import ShellPowerline                                                 // py:13
// from powerline.lib.shell import which                                                      // py:14
// from powerline.bindings.tmux import ...                                                    // py:15-16
// from powerline.lib.encoding import get_preferred_output_encoding                           // py:17
// from powerline.renderers.tmux import attrs_to_tmux_attrs                                   // py:18
// from powerline.commands.main import finish_args                                            // py:19

use crate::ported::bindings::tmux::TmuxVersionInfo;
use crate::ported::config::TMUX_CONFIG_DIRECTORY;
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Port of module-level binding `CONFIG_FILE_NAME` from
/// `powerline/bindings/config.py:22`.
///
/// Python:
/// ```python
/// CONFIG_FILE_NAME = re.compile(r'powerline_tmux_(?P<major>\d+)\.(?P<minor>\d+)(?P<suffix>[a-z]+)?(?:_(?P<mod>plus|minus))?\.conf')
/// ```
#[allow(non_snake_case)]
pub fn CONFIG_FILE_NAME() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"^powerline_tmux_(?P<major>\d+)\.(?P<minor>\d+)(?P<suffix>[a-z]+)?(?:_(?P<mod>plus|minus))?\.conf$",
        )
        .unwrap()
    })
}

/// Version-matching mode for tmux config files — corresponds to the
/// `_plus` / `_minus` suffix on the filename or its absence.
///
/// Mirrors the `CONFIG_MATCHERS` dict at `powerline/bindings/config.py:24`:
/// - `None`   → exact match on (major, minor)
/// - `'plus'` → file applies to tmux version ≥ file_version
/// - `'minus'`→ file applies to tmux version ≤ file_version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigMatcher {
    Exact,
    Plus,
    Minus,
}

impl ConfigMatcher {
    /// Apply the matcher: does this config file's `file_version` apply
    /// to a running tmux at `tmux_version`?
    ///
    /// Mirrors `CONFIG_MATCHERS[mod](a, b)` where `a` is `file_version`
    /// and `b` is `tmux_version`.
    pub fn applies(self, file_version: &TmuxVersionInfo, tmux_version: &TmuxVersionInfo) -> bool {
        match self {
            // py:25  None: lambda a, b: a.major == b.major and a.minor == b.minor
            ConfigMatcher::Exact => {
                file_version.major == tmux_version.major && file_version.minor == tmux_version.minor
            }
            // py:26  'plus': lambda a, b: a[:2] <= b[:2]
            // (Tuple comparison on (major, minor); suffix excluded.)
            ConfigMatcher::Plus => {
                (file_version.major, file_version.minor) <= (tmux_version.major, tmux_version.minor)
            }
            // py:27  'minus': lambda a, b: a[:2] >= b[:2]
            ConfigMatcher::Minus => {
                (file_version.major, file_version.minor) >= (tmux_version.major, tmux_version.minor)
            }
        }
    }

    /// Port of `CONFIG_PRIORITY` dict from
    /// `powerline/bindings/config.py:29`.
    ///
    /// Higher numbers = higher priority. Exact matches beat plus
    /// matches beat minus matches when multiple file-versions overlap.
    pub fn priority(self) -> i32 {
        match self {
            // py:30  None: 3
            ConfigMatcher::Exact => 3,
            // py:31  'plus': 2
            ConfigMatcher::Plus => 2,
            // py:32  'minus': 1
            ConfigMatcher::Minus => 1,
        }
    }
}

/// One discovered config file's metadata.
///
/// Yielded by `list_all_tmux_configs` — mirrors the 4-tuple Python
/// yields at `powerline/bindings/config.py:41-49`.
#[derive(Debug, Clone)]
pub struct TmuxConfigFile {
    pub path: PathBuf,
    pub matcher: ConfigMatcher,
    pub priority: i32,
    pub file_version: TmuxVersionInfo,
}

/// Port of `list_all_tmux_configs()` from
/// `powerline/bindings/config.py:35`.
///
/// List all version-specific tmux configuration files.
///
/// Python uses `os.walk(...)` with `dirs[:] = ()` to prevent recursion;
/// Rust port iterates the single directory using `read_dir`.
pub fn list_all_tmux_configs() -> Vec<TmuxConfigFile> {
    let dir = TMUX_CONFIG_DIRECTORY(); // py:36-37  os.walk(TMUX_CONFIG_DIRECTORY)
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<TmuxConfigFile> = Vec::new();
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let fname_str = match fname.to_str() {
            Some(s) => s,
            None => continue,
        };
        let captures = match CONFIG_FILE_NAME().captures(fname_str) {
            // py:39
            Some(c) => c,
            None => continue,
        };
        // py:41  assert match.group('suffix') is None
        // (Upstream's assertion fires on suffix-bearing filenames;
        // mirror it by skipping such files.)
        if captures.name("suffix").is_some() {
            continue;
        }
        let major: f64 = match captures.name("major").and_then(|m| m.as_str().parse().ok()) {
            Some(n) => n,
            None => continue,
        };
        let minor: i32 = match captures.name("minor").and_then(|m| m.as_str().parse().ok()) {
            Some(n) => n,
            None => continue,
        };
        let mod_str = captures.name("mod").map(|m| m.as_str());
        let matcher = match mod_str {
            None => ConfigMatcher::Exact,
            Some("plus") => ConfigMatcher::Plus,
            Some("minus") => ConfigMatcher::Minus,
            Some(_) => continue,
        };
        out.push(TmuxConfigFile {
            path: entry.path(), // py:42  os.path.join(root, fname)
            matcher,
            priority: matcher.priority(),
            file_version: TmuxVersionInfo {
                // py:45-49
                major,
                minor,
                suffix: None,
            },
        });
    }
    out
}

/// Port of `get_tmux_configs()` from
/// `powerline/bindings/config.py:55`.
///
/// Get tmux configuration suffix given parsed tmux version.
///
/// Returns `(path, sort_key)` pairs for every config file whose
/// matcher applies to `version`. The sort_key encodes upstream's
/// `priority + minor*10 + major*10000` ordering for source order.
pub fn get_tmux_configs(version: &TmuxVersionInfo) -> Vec<(PathBuf, i64)> {
    let mut out = Vec::new();
    for cfg in list_all_tmux_configs() {
        // py:60
        if cfg.matcher.applies(&cfg.file_version, version) {
            // py:61
            // py:62  priority + file_version.minor * 10 + file_version.major * 10000
            let sort_key = (cfg.priority as i64)
                + (cfg.file_version.minor as i64) * 10
                + (cfg.file_version.major as i64) * 10_000;
            out.push((cfg.path, sort_key));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(major: f64, minor: i32) -> TmuxVersionInfo {
        TmuxVersionInfo {
            major,
            minor,
            suffix: None,
        }
    }

    #[test]
    fn config_file_name_matches_standard_format() {
        let re = CONFIG_FILE_NAME();
        // Standard "powerline_tmux_2.1.conf"
        assert!(re.is_match("powerline_tmux_2.1.conf"));
        // With _plus / _minus suffix
        assert!(re.is_match("powerline_tmux_1.8_plus.conf"));
        assert!(re.is_match("powerline_tmux_1.8_minus.conf"));
        // Wrong format
        assert!(!re.is_match("powerline_tmux_2.1.txt"));
        assert!(!re.is_match("powerline_2.1.conf"));
    }

    #[test]
    fn exact_matcher_requires_same_major_minor() {
        let m = ConfigMatcher::Exact;
        assert!(m.applies(&ver(2.0, 1), &ver(2.0, 1)));
        assert!(!m.applies(&ver(2.0, 1), &ver(2.0, 2)));
        assert!(!m.applies(&ver(2.0, 1), &ver(3.0, 1)));
    }

    #[test]
    fn plus_matcher_applies_when_file_lte_tmux() {
        let m = ConfigMatcher::Plus;
        // file=1.8 applies to tmux >= 1.8
        assert!(m.applies(&ver(1.0, 8), &ver(1.0, 8)));
        assert!(m.applies(&ver(1.0, 8), &ver(2.0, 1)));
        assert!(!m.applies(&ver(2.0, 0), &ver(1.0, 9)));
    }

    #[test]
    fn minus_matcher_applies_when_file_gte_tmux() {
        let m = ConfigMatcher::Minus;
        // file=1.8 applies to tmux <= 1.8
        assert!(m.applies(&ver(1.0, 8), &ver(1.0, 8)));
        assert!(m.applies(&ver(2.0, 1), &ver(1.0, 9)));
        assert!(!m.applies(&ver(1.0, 8), &ver(2.0, 1)));
    }

    #[test]
    fn priority_order_matches_upstream() {
        // py:30-32  None=3, plus=2, minus=1
        assert_eq!(ConfigMatcher::Exact.priority(), 3);
        assert_eq!(ConfigMatcher::Plus.priority(), 2);
        assert_eq!(ConfigMatcher::Minus.priority(), 1);
    }
}
