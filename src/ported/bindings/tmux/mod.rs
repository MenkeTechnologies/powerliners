// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/tmux/__init__.py`.
//!
//! Tmux client: thin wrappers over `tmux` subprocess invocations used
//! by the `segments/tmux.py` segments (`attached_clients`) and by the
//! tmux config-source helpers in `commands/`. All fns shell out via
//! `lib/shell.run_cmd`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// import os                                        // py:5
// import subprocess                                // py:6
// from collections import namedtuple               // py:8
// from powerline.lib.shell import run_cmd          // py:10

use crate::ported::lib::shell::run_cmd;
use regex::Regex;
use std::process::Command;
use std::sync::OnceLock;

/// Port of `TmuxVersionInfo` namedtuple from
/// `powerline/bindings/tmux/__init__.py:13`.
///
/// Python: `TmuxVersionInfo = namedtuple('TmuxVersionInfo', ('major', 'minor', 'suffix'))`
#[derive(Debug, Clone, PartialEq)]
pub struct TmuxVersionInfo {
    // py:13
    pub major: f64, // f64 to model `float('inf')` py:74 master path
    pub minor: i32,
    pub suffix: Option<String>,
}

/// Port of `get_tmux_executable_name()` from
/// `powerline/bindings/tmux/__init__.py:16`.
///
/// Returns tmux executable name.
///
/// It should be defined in POWERLINE_TMUX_EXE environment variable,
/// otherwise it is simply "tmux".
pub fn get_tmux_executable_name() -> String {
    // py:22  return os.environ.get('POWERLINE_TMUX_EXE', 'tmux')
    std::env::var("POWERLINE_TMUX_EXE").unwrap_or_else(|_| "tmux".to_string())
}

/// Port of `_run_tmux()` from `powerline/bindings/tmux/__init__.py:25`.
///
/// Internal helper. Python: `def _run_tmux(runner, args): return runner([get_tmux_executable_name()] + list(args))`.
///
/// Rust shape: instead of accepting a runner callable (closures over
/// `Command` plus pl-logger have awkward lifetimes), we accept the
/// args and return a `Result<std::process::Output>` so callers choose
/// how to consume. `run_tmux_command` and `get_tmux_output` (the only
/// two upstream callers) each derive their behaviour from the same
/// `Output`.
pub fn _run_tmux(args: &[&str]) -> std::io::Result<std::process::Output> {
    // py:26  return runner([get_tmux_executable_name()] + list(args))
    let exe = get_tmux_executable_name();
    Command::new(&exe).args(args).output()
}

/// Port of `run_tmux_command()` from
/// `powerline/bindings/tmux/__init__.py:29`.
///
/// Run tmux command, ignoring the output.
pub fn run_tmux_command(args: &[&str]) {
    // py:31  _run_tmux(subprocess.check_call, args)
    let _ = _run_tmux(args);
}

/// Port of `get_tmux_output()` from
/// `powerline/bindings/tmux/__init__.py:34`.
///
/// Run tmux command and return its output.
pub fn get_tmux_output(pl: &(), args: &[&str]) -> Option<String> {
    // py:36  return _run_tmux(lambda cmd: run_cmd(pl, cmd), args)
    let mut cmd_vec: Vec<String> = vec![get_tmux_executable_name()];
    for a in args {
        cmd_vec.push(a.to_string());
    }
    run_cmd(pl, &cmd_vec, None, true)
}

/// Port of `set_tmux_environment()` from
/// `powerline/bindings/tmux/__init__.py:39`.
///
/// Set tmux global environment variable.
///
/// :param str varname: Name of the variable to set.
/// :param str value: Variable value.
/// :param bool remove: True if variable should be removed from the
///     environment prior to attaching any client (runs
///     `tmux set-environment -r {varname}`).
pub fn set_tmux_environment(varname: &str, value: &str, remove: bool) {
    // py:40  def set_tmux_environment(varname, value, remove=True):
    // py:51  run_tmux_command('set-environment', '-g', varname, value)
    run_tmux_command(&["set-environment", "-g", varname, value]);
    // py:52  if remove:
    if remove {
        // py:53  try:
        // py:54  run_tmux_command('set-environment', '-r', varname)
        // py:55  except subprocess.CalledProcessError:
        // py:56  # On tmux-2.0 this command may fail for whatever reason. Since it is
        // py:57  # critical just ignore the failure.
        // py:58  pass
        let _ = _run_tmux(&["set-environment", "-r", varname]);
    }
}

/// Port of `source_tmux_file()` from
/// `powerline/bindings/tmux/__init__.py:58`.
///
/// Source tmux configuration file.
pub fn source_tmux_file(fname: &str) {
    // py:63  run_tmux_command('source', fname)
    run_tmux_command(&["source", fname]);
}

/// Port of module-level binding `NON_DIGITS` from
/// `powerline/bindings/tmux/__init__.py:66`.
///
/// Python: `NON_DIGITS = re.compile('[^0-9]+')`. Compiled once at
/// import time; Rust lazy-inits via `OnceLock`.
#[allow(non_snake_case)]
pub fn NON_DIGITS() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[^0-9]+").unwrap())
}

/// Port of module-level binding `DIGITS` from
/// `powerline/bindings/tmux/__init__.py:67`.
#[allow(non_snake_case)]
pub fn DIGITS() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[0-9]+").unwrap())
}

/// Port of module-level binding `NON_LETTERS` from
/// `powerline/bindings/tmux/__init__.py:68`.
///
/// Defined upstream but not currently used by `get_tmux_version`;
/// retained for parity.
#[allow(non_snake_case, dead_code)]
pub fn NON_LETTERS() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[^a-z]+").unwrap())
}

/// Port of `get_tmux_version()` from
/// `powerline/bindings/tmux/__init__.py:71`.
///
/// Parse `tmux -V` output into a `TmuxVersionInfo`.
pub fn get_tmux_version(pl: &()) -> Option<TmuxVersionInfo> {
    // py:75  def get_tmux_version(pl):
    // py:76  version_string = get_tmux_output(pl, '-V')
    let version_string = get_tmux_output(pl, &["-V"])?;

    // py:77  _, version_string = version_string.split(' ')
    let mut parts = version_string.splitn(2, ' ');
    let _ = parts.next()?;
    // py:78  version_string = version_string.strip()
    let version_string = parts.next()?.trim();

    // py:79  if version_string == 'master':
    if version_string == "master" {
        // py:80  return TmuxVersionInfo(float('inf'), 0, version_string)
        return Some(TmuxVersionInfo {
            major: f64::INFINITY,
            minor: 0,
            suffix: Some(version_string.to_string()),
        });
    }

    // py:81  major, minor = version_string.split('.')
    let (major_raw, minor_raw) = version_string.split_once('.')?;

    // py:82  major = NON_DIGITS.subn('', major)[0]
    let major_str = NON_DIGITS().replace_all(major_raw, "").into_owned();
    // py:83  suffix = DIGITS.subn('', minor)[0] or None
    let suffix_str = DIGITS().replace_all(minor_raw, "").into_owned();
    let suffix = if suffix_str.is_empty() {
        None
    } else {
        Some(suffix_str)
    };
    // py:84  minor = NON_DIGITS.subn('', minor)[0]
    let minor_str = NON_DIGITS().replace_all(minor_raw, "").into_owned();

    // py:85  return TmuxVersionInfo(int(major), int(minor), suffix)
    Some(TmuxVersionInfo {
        major: major_str.parse().ok()?,
        minor: minor_str.parse().ok()?,
        suffix,
    })
}

/// Process-wide lock serializing every test that mutates the
/// `POWERLINE_TMUX_EXE` env var. Lives at the crate level so the
/// races between `bindings/tmux/mod.rs::tests` and
/// `segments/tmux.rs::tests` actually share the same Mutex.
/// Without sharing, two `Mutex::new(())` in different modules each
/// serialize their OWN tests but not against each other — that
/// produced the macOS CI flake (left="/usr/local/bin/tmux",
/// right="tmux").
#[cfg(test)]
pub(crate) static POWERLINE_TMUX_EXE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_tmux_executable_name_defaults_to_tmux() {
        let _guard = POWERLINE_TMUX_EXE_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        let prev = std::env::var("POWERLINE_TMUX_EXE").ok();
        std::env::remove_var("POWERLINE_TMUX_EXE");
        assert_eq!(get_tmux_executable_name(), "tmux");
        if let Some(p) = prev {
            std::env::set_var("POWERLINE_TMUX_EXE", p);
        }
    }

    #[test]
    fn get_tmux_executable_name_uses_env_var() {
        let _guard = POWERLINE_TMUX_EXE_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        std::env::set_var("POWERLINE_TMUX_EXE", "/usr/local/bin/tmux");
        assert_eq!(get_tmux_executable_name(), "/usr/local/bin/tmux");
        std::env::remove_var("POWERLINE_TMUX_EXE");
    }

    /// regex helpers work as expected on a sample version string.
    #[test]
    fn version_regexes_strip_correctly() {
        // major: "2" → "2" (no non-digits to strip)
        assert_eq!(NON_DIGITS().replace_all("2", "").into_owned(), "2");
        // minor: "9a" → suffix "a", minor "9"
        assert_eq!(DIGITS().replace_all("9a", "").into_owned(), "a");
        assert_eq!(NON_DIGITS().replace_all("9a", "").into_owned(), "9");
    }
}
