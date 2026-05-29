// vim:fileencoding=utf-8:noet
//! Port of `powerline/commands/daemon.py`.
//!
//! Argument-spec for the `powerline-daemon` binary.

// from __future__ import (division, absolute_import, print_function)  // py:2
// import argparse                                  // py:4

use crate::ported::commands::lint::{ArgAction, ArgParser, Argument};

/// Port of `get_argparser()` from `powerline/commands/daemon.py:7`.
///
/// Returns the argument parser for `powerline-daemon`.
pub fn get_argparser() -> ArgParser {
    // py:7
    ArgParser {
        description: "Daemon that improves powerline performance.".to_string(), // py:8
        arguments: vec![
            // py:9-17  --quiet / -q
            Argument {
                flags: vec!["--quiet".into(), "-q".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "Without other options: do not complain about already running \
                       powerline-daemon instance. \
                       Will still exit with 1. \
                       With `--kill' and `--replace': do not show any messages. \
                       With `--foreground': ignored. \
                       Does not silence exceptions in any case."
                    .to_string(),
            },
            // py:18  --socket / -s
            Argument {
                flags: vec!["--socket".into(), "-s".into()],
                action: ArgAction::Store,
                metavar: None,
                help: "Specify socket which will be used for connecting to daemon.".to_string(),
            },
            // py:19  --kill / -k
            Argument {
                flags: vec!["--kill".into(), "-k".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "Kill an already running instance.".to_string(),
            },
            // py:20  --foreground / -f
            Argument {
                flags: vec!["--foreground".into(), "-f".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                // Note: the upstream help text uses a typographic apostrophe
                // (U+2019) inside "don\u{2019}t" — preserved verbatim.
                help: "Run in the foreground (don\u{2019}t daemonize).".to_string(),
            },
            // py:21  --replace / -r
            Argument {
                flags: vec!["--replace".into(), "-r".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "Replace an already running instance.".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_parser_description_matches_upstream() {
        let p = get_argparser();
        assert_eq!(p.description, "Daemon that improves powerline performance.");
    }

    #[test]
    fn daemon_parser_has_five_arguments() {
        let p = get_argparser();
        assert_eq!(p.arguments.len(), 5);
    }

    #[test]
    fn daemon_parser_flag_set() {
        let p = get_argparser();
        let names: Vec<&str> = p
            .arguments
            .iter()
            .flat_map(|a| a.flags.iter().map(|s| s.as_str()))
            .collect();
        assert!(names.contains(&"--quiet"));
        assert!(names.contains(&"--socket"));
        assert!(names.contains(&"--kill"));
        assert!(names.contains(&"--foreground"));
        assert!(names.contains(&"--replace"));
    }
}
