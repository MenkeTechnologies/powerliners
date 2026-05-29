// vim:fileencoding=utf-8:noet
//! Port of `powerline/version.py`.
//!
//! Exposes the powerline-status version string, with an optional git-revision
//! suffix appended when invoked from a working copy.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// (No Rust analogue — Python compatibility shim for 2/3.)

use std::process::Command; // py:4  import subprocess
                           // py:5  from traceback import print_exc — Python's traceback printer.
                           // Rust does not have a 1:1 analogue; we route the equivalent diagnostic
                           // through eprintln! at the catch site (see get_version below).

/// Port of module constant `__version__` from `powerline/version.py:7`.
#[allow(non_upper_case_globals)]
pub const __version__: &str = "2.8.4"; // py:7

/// Port of `get_version()` from `powerline/version.py:9`.
///
/// Returns `__version__` (optionally with a `b<count>` suffix encoding the
/// number of commits since the tag) by shelling out to `git rev-list --count`.
/// Falls back to `__version__` and prints a traceback if git is unavailable.
pub fn get_version() -> String {
    // py:10  try:
    match Command::new("git") // py:11  subprocess.check_output(['git', ...
        .args([
            "rev-list",
            "--count",
            &format!("{}..HEAD", __version__), // py:11  __version__ + '..HEAD'
        ])
        .output()
    {
        Ok(out) if out.status.success() => {
            let count = String::from_utf8_lossy(&out.stdout).trim().to_string(); // py:11  .strip().decode()
            format!("{}b{}", __version__, count) // py:11  __version__ + 'b' + ...
        }
        Ok(out) => {
            // py:12-13  except Exception: print_exc()
            // git ran but returned non-zero — emit a traceback-equivalent
            // diagnostic on stderr and fall through.
            eprintln!(
                "powerline.version.get_version: git rev-list exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            );
            __version__.to_string() // py:14  return __version__
        }
        Err(e) => {
            // py:12-13  except Exception: print_exc()
            eprintln!("powerline.version.get_version: {}", e);
            __version__.to_string() // py:14  return __version__
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `__version__` matches upstream string literal at version.py:7.
    #[test]
    fn version_constant_matches_upstream() {
        assert_eq!(__version__, "2.8.4");
    }

    /// `get_version()` returns a non-empty string in any environment.
    /// In a git working tree it yields "<version>b<count>", outside one
    /// (or when git fails) it yields just "<version>".
    #[test]
    fn get_version_returns_nonempty() {
        let v = get_version();
        assert!(v.starts_with(__version__));
        assert!(!v.is_empty());
    }
}
