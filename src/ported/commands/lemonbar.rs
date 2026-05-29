// vim:fileencoding=utf-8:noet
//! Port of `powerline/commands/lemonbar.py`.
//!
//! Argument-spec for the `powerline-lemonbar` binary.

// from __future__ import (division, absolute_import, print_function)  // py:3
// import argparse                                  // py:5

use crate::ported::commands::lint::{ArgAction, ArgParser, Argument};

/// Port of `get_argparser()` from `powerline/commands/lemonbar.py:8`.
///
/// Returns the argument parser for `powerline-lemonbar`.
pub fn get_argparser() -> ArgParser {
    // py:9-11  description
    ArgParser {
        description: "Powerline BAR bindings.".to_string(),
        arguments: vec![
            // py:12-15  --i3
            Argument {
                flags: vec!["--i3".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "Subscribe for i3 events.".to_string(),
            },
            // py:16-19  --height
            Argument {
                flags: vec!["--height".into()],
                action: ArgAction::Store,
                metavar: Some("PIXELS".into()),
                help: "Bar height.".to_string(),
            },
            // py:20-24  --interval / -i
            Argument {
                flags: vec!["--interval".into(), "-i".into()],
                action: ArgAction::Store,
                metavar: Some("SECONDS".into()),
                help: "Refresh interval.".to_string(),
            },
            // py:25-29  --bar-command / -C
            Argument {
                flags: vec!["--bar-command".into(), "-C".into()],
                action: ArgAction::Store,
                metavar: Some("CMD".into()),
                help: "Name of the lemonbar executable to use.".to_string(),
            },
            // py:30-34  args (REMAINDER)
            Argument {
                flags: vec!["args".into()],
                action: ArgAction::Store,
                metavar: None,
                help: "Extra arguments for lemonbar. Should be preceded with ``--`` \
                       argument in order not to be confused with script own arguments."
                    .to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lemonbar_parser_description_matches_upstream() {
        let p = get_argparser();
        assert_eq!(p.description, "Powerline BAR bindings.");
    }

    #[test]
    fn lemonbar_parser_has_five_arguments() {
        let p = get_argparser();
        assert_eq!(p.arguments.len(), 5);
    }

    #[test]
    fn lemonbar_parser_has_i3_flag() {
        let p = get_argparser();
        assert!(p.arguments[0].flags.contains(&"--i3".to_string()));
        assert_eq!(p.arguments[0].action, ArgAction::StoreTrue);
    }

    #[test]
    fn lemonbar_parser_bar_command_default_meta() {
        let p = get_argparser();
        let bar = p
            .arguments
            .iter()
            .find(|a| a.flags.contains(&"--bar-command".to_string()))
            .unwrap();
        assert_eq!(bar.metavar, Some("CMD".to_string()));
    }
}
