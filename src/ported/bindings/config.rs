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
use crate::ported::config::{POWERLINE_ROOT, TMUX_CONFIG_DIRECTORY};
use crate::ported::lib::shell::which;
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

/// Port of `EmptyArgs` class from
/// `powerline/bindings/config.py:89-96`.
///
/// Python's `EmptyArgs` is a stand-in for the `argparse.Namespace`
/// passed to `init_tmux_environment` when invoked outside of a CLI
/// context. Sets only `ext`, `side`, and `config_path`; every other
/// attribute access via `__getattr__` returns `None`.
///
/// Rust port encodes `ext` as `Vec<String>` (Python sets `[ext]` at
/// py:91), `side` as `String`, and `config_path` as `Option<String>`.
/// The `__getattr__` returns-None behavior is not surfaced because
/// Rust callers use explicit field access — any missing attribute
/// would be a compile error rather than a silent None.
#[derive(Debug, Clone)]
pub struct EmptyArgs {
    /// py:91  self.ext = [ext]
    pub ext: Vec<String>,
    /// py:92  self.side = 'left'
    pub side: String,
    /// py:93  self.config_path = None
    pub config_path: Option<String>,
}

impl EmptyArgs {
    /// Port of `EmptyArgs.__init__()` at py:90-93.
    ///
    /// Note Python's signature takes `(self, ext, config_path)` and
    /// stores `config_path` as **None** at py:93 regardless of the
    /// argument value — this is upstream behavior, not a bug. The
    /// argument is captured here so callers exercising the binding
    /// path can pass it through, but it lands in
    /// `EmptyArgs::config_path` as `None` to match Python.
    pub fn new(ext: &str, _config_path: Option<&str>) -> Self {
        // py:91  self.ext = [ext]
        // py:92  self.side = 'left'
        // py:93  self.config_path = None
        EmptyArgs {
            ext: vec![ext.to_string()],
            side: "left".to_string(),
            config_path: None,
        }
    }
}

/// Port of module-level `TMUX_VAR_RE` regex from
/// `powerline/bindings/config.py:179`.
///
/// Python:
/// ```python
/// TMUX_VAR_RE = re.compile(r'\$(_POWERLINE_\w+)')
/// ```
#[allow(non_snake_case)]
pub fn TMUX_VAR_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\$(_POWERLINE_\w+)").unwrap())
}

/// Port of `check_command()` from
/// `powerline/bindings/config.py:231-233`.
///
/// Returns `Some(cmd)` if `which(cmd)` succeeds; `None` otherwise.
/// Python returns the string itself when found (py:233) and falls
/// through to an implicit `None` when missing.
pub fn check_command(cmd: &str) -> Option<String> {
    // py:232  if which(cmd):
    if which(cmd).is_some() {
        // py:233  return cmd
        Some(cmd.to_string())
    } else {
        None
    }
}

/// Port of `deduce_command()` from
/// `powerline/bindings/config.py:236-261`.
///
/// Tries a chain of candidates and returns the first one that resolves
/// via `which()`. Mirrors Python's chained `... or check_command(...)`
/// fallback at py:252-261.
pub fn deduce_command() -> Option<String> {
    // py:254  check_command('powerline')
    if let Some(c) = check_command("powerline") {
        return Some(c);
    }
    // py:255  os.path.join(POWERLINE_ROOT, 'scripts', 'powerline')
    let p = POWERLINE_ROOT().join("scripts").join("powerline");
    if let Some(c) = check_command(&p.to_string_lossy()) {
        return Some(c);
    }
    // py:256-257  if which('sh') and which('sed') and which('socat')
    if which("sh").is_some() && which("sed").is_some() && which("socat").is_some() {
        let p = POWERLINE_ROOT().join("client").join("powerline.sh");
        if let Some(c) = check_command(&p.to_string_lossy()) {
            return Some(c);
        }
    }
    // py:258  os.path.join(POWERLINE_ROOT, 'client', 'powerline.py')
    let p = POWERLINE_ROOT().join("client").join("powerline.py");
    if let Some(c) = check_command(&p.to_string_lossy()) {
        return Some(c);
    }
    // py:259  check_command('powerline-render')
    if let Some(c) = check_command("powerline-render") {
        return Some(c);
    }
    // py:260  os.path.join(POWERLINE_ROOT, 'scripts', 'powerline-render')
    let p = POWERLINE_ROOT().join("scripts").join("powerline-render");
    check_command(&p.to_string_lossy())
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

    #[test]
    fn empty_args_init_stores_ext_as_single_element_list() {
        // py:91  self.ext = [ext]
        let a = EmptyArgs::new("tmux", None);
        assert_eq!(a.ext, vec!["tmux".to_string()]);
    }

    #[test]
    fn empty_args_init_defaults_side_to_left() {
        // py:92  self.side = 'left'
        let a = EmptyArgs::new("tmux", None);
        assert_eq!(a.side, "left");
    }

    #[test]
    fn empty_args_init_pins_config_path_to_none() {
        // py:93  self.config_path = None — even though Python
        // accepts config_path as an argument, the body discards
        // it. Mirror that behavior.
        let a = EmptyArgs::new("tmux", Some("/etc/powerline"));
        assert!(a.config_path.is_none());
    }

    #[test]
    fn tmux_var_re_matches_dollar_powerline_var() {
        // py:179  re.compile(r'\$(_POWERLINE_\w+)')
        let re = TMUX_VAR_RE();
        assert!(re.is_match("$_POWERLINE_FOO"));
        assert!(re.is_match("foo $_POWERLINE_BAR_X bar"));
        assert!(!re.is_match("_POWERLINE_NOPE"));
        assert!(!re.is_match("$POWERLINE_NO_UNDER"));
    }

    #[test]
    fn tmux_var_re_captures_group_after_dollar() {
        let re = TMUX_VAR_RE();
        let cap = re.captures("$_POWERLINE_FG").unwrap();
        assert_eq!(cap.get(1).unwrap().as_str(), "_POWERLINE_FG");
    }

    #[test]
    fn check_command_returns_some_for_real_binary() {
        // sh exists on every Unix; mirrors py:232-233
        let r = check_command("sh");
        assert_eq!(r, Some("sh".to_string()));
    }

    #[test]
    fn check_command_returns_none_for_missing_binary() {
        let r = check_command("definitely-not-on-this-system-xyz-abc");
        assert!(r.is_none());
    }

    #[test]
    fn deduce_command_returns_some_or_none() {
        // Deterministic without mocking which(): we can't pin a
        // specific candidate, but py:252-261 always returns either a
        // string or None — assert the return shape, not the value.
        let r = deduce_command();
        if let Some(s) = r {
            assert!(!s.is_empty());
        }
    }
}
