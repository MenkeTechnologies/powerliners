// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/shell.py`.
//!
//! Subprocess helpers used by VCS drivers, the tmux binding, and the
//! AppleScript shim. Three exported fns: `run_cmd`, `asrun`, `readlines`,
//! plus a `which` polyfill (`std::shutil.which` fallback for old Pythons).
//!
//! In Rust, all of these collapse onto `std::process::Command` —
//! straight ports.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import os                                        // py:5
// from subprocess import Popen, PIPE               // py:7
// from functools import partial                    // py:8
// from powerline.lib.encoding import ...           // py:10

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Port of `run_cmd()` from `powerline/lib/shell.py:19`.
///
/// Run command and return its stdout, stripped.
///
/// If running command fails returns `None` and logs failure to `pl`
/// argument (Python). Rust port: failure surfaces as `None`; the `pl`
/// logger is currently the unit type (`&()`) until the logger trait
/// lands. Match upstream's behaviour: combine stdout decoding +
/// optional strip; `strip=true` is the default.
pub fn run_cmd(_pl: &(), cmd: &[String], stdin: Option<&str>, strip: bool) -> Option<String> {
    // py:19  def run_cmd(pl, cmd, stdin=None, strip=True):
    // py:20-32  docstring
    // py:33  try:
    // py:34  p = Popen(cmd, shell=False, stdout=PIPE, stdin=PIPE)
    let mut child = Command::new(cmd.first()?)
        .args(&cmd[1..])
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        // py:35  except OSError as e:
        // py:36  pl.exception('Could not execute command ({0}): {1}', e, cmd)
        // py:37  return None
        .ok()?;

    // py:38  else:
    // py:39  stdout, err = p.communicate(
    // py:40  stdin if stdin is None else stdin.encode(get_preferred_output_encoding()))
    if let Some(s) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            let _ = child_stdin.write_all(s.as_bytes());
        }
    } else {
        drop(child.stdin.take());
    }
    let output = child.wait_with_output().ok()?;

    // py:41  stdout = stdout.decode(get_preferred_input_encoding())
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

    // py:42  return stdout.strip() if strip else stdout
    if strip {
        Some(stdout.trim().to_string())
    } else {
        Some(stdout)
    }
}

/// Port of `asrun()` from `powerline/lib/shell.py:45`.
///
/// Run the given AppleScript and return the standard output and error.
pub fn asrun(pl: &(), ascript: &str) -> Option<String> {
    // py:45  def asrun(pl, ascript):
    // py:46  '''Run the given AppleScript and return the standard output and error.'''
    // py:47  return run_cmd(pl, ['osascript', '-'], ascript)
    run_cmd(
        pl,
        &["osascript".to_string(), "-".to_string()],
        Some(ascript),
        true,
    )
}

/// Port of `readlines()` from `powerline/lib/shell.py:50`.
///
/// Run command and read its output, line by line.
///
/// Python uses a generator (`yield`); Rust returns a `Vec<String>`.
/// The streaming pattern is the same — caller iterates results.
pub fn readlines(cmd: &[String], cwd: &std::path::Path) -> Vec<String> {
    // py:50  def readlines(cmd, cwd):
    // py:51-57  docstring
    // py:58  p = Popen(cmd, shell=False, stdout=PIPE, stderr=PIPE, cwd=cwd)
    let output = match Command::new(cmd.first().map(|s| s.as_str()).unwrap_or(""))
        .args(&cmd[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        // py:59  encoding = get_preferred_input_encoding()
        // py:60  p.stderr.close()
        .stderr(Stdio::null())
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    // py:61  with p.stdout:
    // py:62  for line in p.stdout:
    // py:63  yield line[:-1].decode(encoding)
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(String::from)
        .collect()
}

/// Port of `which()` from `powerline/lib/shell.py:67` (the `shutil.which`
/// import; for Python < 3.3 a polyfill body is supplied at py:71-133).
///
/// Given a command name (or path), return the absolute path on PATH
/// that conforms to the access mode, or None.
///
/// Rust uses `env::var("PATH")` walked as colon-separated entries on
/// Unix / semicolon on Windows. Mirrors the upstream polyfill body
/// directly without the Py2/3 compat fork.
pub fn which(cmd: &str) -> Option<PathBuf> {
    // py:71  def which(cmd, mode=os.F_OK | os.X_OK, path=None):
    // py:72-79  docstring
    // py:93  if os.path.dirname(cmd):
    if cmd.contains(std::path::MAIN_SEPARATOR) {
        // py:94  if _access_check(cmd, mode):
        let p = PathBuf::from(cmd);
        if _access_check(&p) {
            // py:95  return cmd
            return Some(p);
        }
        // py:96  return None
        return None;
    }

    // py:98  if path is None:
    // py:99  path = os.environ.get('PATH', os.defpath)
    let path = std::env::var_os("PATH")?;
    // py:100  if not path:
    // py:101  return None
    // py:102  path = path.split(os.pathsep)
    // py:124  seen = set()
    let mut seen = std::collections::HashSet::new();
    // py:125  for dir in path:
    for dir in std::env::split_paths(&path) {
        // py:126  normdir = os.path.normcase(dir)
        // py:127  if normdir not in seen:
        // py:128  seen.add(normdir)
        if !seen.insert(dir.clone()) {
            continue;
        }
        // py:129  for thefile in files:
        // py:130  name = os.path.join(dir, thefile)
        let candidate = dir.join(cmd);
        // py:131  if _access_check(name, mode):
        if _access_check(&candidate) {
            // py:132  return name
            return Some(candidate);
        }
    }
    // py:133  return None
    None
}

/// Port of `_access_check()` from `powerline/lib/shell.py:83`.
///
/// Check that a given file exists, is accessible (executable), and is
/// not a directory.
fn _access_check(fn_path: &std::path::Path) -> bool {
    // py:84-88  exists + access(mode) + not isdir
    if !fn_path.exists() {
        return false;
    }
    if fn_path.is_dir() {
        return false;
    }
    // Executable bit (Unix). Windows: any extension match is treated
    // as executable per py:115-122; we accept any file for portability.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match fn_path.metadata() {
            Ok(m) => m.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_echo_returns_stdout() {
        let out = run_cmd(&(), &["echo".to_string(), "hello".to_string()], None, true);
        assert_eq!(out.as_deref(), Some("hello"));
    }

    #[test]
    fn run_cmd_missing_command_returns_none() {
        let out = run_cmd(
            &(),
            &["powerliners-nonexistent-command".to_string()],
            None,
            true,
        );
        assert!(out.is_none());
    }

    #[test]
    fn run_cmd_no_strip_keeps_trailing_newline() {
        let out = run_cmd(&(), &["echo".to_string(), "hello".to_string()], None, false);
        assert_eq!(out.as_deref(), Some("hello\n"));
    }

    #[test]
    fn which_finds_echo() {
        let p = which("sh");
        assert!(p.is_some(), "which('sh') should find /bin/sh on Unix");
    }

    #[test]
    fn which_missing_returns_none() {
        let p = which("powerliners-nonexistent-binary");
        assert!(p.is_none());
    }

    #[test]
    fn readlines_returns_per_line() {
        let lines = readlines(
            &["printf".to_string(), "a\nb\nc\n".to_string()],
            &PathBuf::from("/"),
        );
        assert_eq!(
            lines,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }
}
