// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/powerline-lint`.
//!
//! 13-line driver that calls `check(config_path, debug)` from
//! `powerline.lint` and exits with its return value.

// #!/usr/bin/env python                              // sh:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // sh:3
// import sys                                         // sh:5
// from powerline.lint import check                   // sh:7
// from powerline.commands.lint import get_argparser  // sh:8

use crate::ported::commands::lint::get_argparser;

/// Port of the `if __name__ == '__main__':` block at
/// `vendor/powerline/scripts/powerline-lint:11-13`.
///
/// Returns the process exit code Python would pass to `sys.exit()`.
/// Python at sh:13 calls `sys.exit(check(args.config_path, args.debug))`
/// which exits with the integer return of `check()` (0 = clean,
/// nonzero = problems found).
///
/// The full `check(paths, debug, echoerr, require_ext)` from
/// `powerline/lint/__init__.py` is deferred (it weaves through every
/// Spec-builder DSL). Until that lands, this script returns 0 when
/// invoked with `--config-path` (mirroring "no problems found" since
/// nothing is yet checked) and 2 when invoked without one (mirroring
/// argparse's required-argument exit code).
pub fn main(args: &[String]) -> i32 {
    // sh:12  args = get_argparser().parse_args()
    let _parser = get_argparser();

    // Minimal arg parsing: look for --config-path / -p.
    let mut config_paths: Vec<String> = Vec::new();
    let mut debug = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--config-path" | "-p" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    config_paths.push(p.clone());
                }
            }
            "--debug" | "-d" => {
                debug = true;
            }
            _ => {}
        }
        i += 1;
    }
    let _ = debug;

    if config_paths.is_empty() {
        eprintln!("powerline-lint: --config-path is required");
        return 2;
    }

    // sh:13  sys.exit(check(args.config_path, args.debug))
    // TODO: powerline.lint.check() is deferred; returning 0 once a
    // config path is supplied mirrors "no problems found" until the
    // checker is wired up.
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_returns_2_when_no_config_path() {
        // py:12-13  argparse "the following arguments are required: -p" → exit 2
        assert_eq!(main(&[]), 2);
    }

    #[test]
    fn main_returns_0_when_config_path_supplied_long() {
        let r = main(&[
            "--config-path".to_string(),
            "/tmp/powerline-config".to_string(),
        ]);
        assert_eq!(r, 0);
    }

    #[test]
    fn main_returns_0_when_config_path_supplied_short() {
        let r = main(&["-p".to_string(), "/tmp/powerline-config".to_string()]);
        assert_eq!(r, 0);
    }

    #[test]
    fn main_accepts_debug_flag() {
        let r = main(&[
            "--config-path".to_string(),
            "/tmp/x".to_string(),
            "--debug".to_string(),
        ]);
        assert_eq!(r, 0);
    }
}
