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
    // py:8  def get_argparser(ArgumentParser=argparse.ArgumentParser):
    // py:9  parser = ArgumentParser(
    // py:10  description='Powerline BAR bindings.'
    // py:11  )
    ArgParser {
        description: "Powerline BAR bindings.".to_string(),
        arguments: vec![
            // py:12  parser.add_argument(
            // py:13  '--i3', action='store_true',
            Argument {
                flags: vec!["--i3".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                // py:14  help='Subscribe for i3 events.'
                // py:15  )
                help: "Subscribe for i3 events.".to_string(),
            },
            // py:16  parser.add_argument(
            // py:17  '--height', default='',
            Argument {
                flags: vec!["--height".into()],
                action: ArgAction::Store,
                // py:18  metavar='PIXELS', help='Bar height.'
                metavar: Some("PIXELS".into()),
                // py:19  )
                help: "Bar height.".to_string(),
            },
            // py:20  parser.add_argument(
            // py:21  '--interval', '-i',
            Argument {
                flags: vec!["--interval".into(), "-i".into()],
                // py:22  type=float, default=0.5,
                action: ArgAction::Store,
                // py:23  metavar='SECONDS', help='Refresh interval.'
                metavar: Some("SECONDS".into()),
                // py:24  )
                help: "Refresh interval.".to_string(),
            },
            // py:25  parser.add_argument(
            // py:26  '--bar-command', '-C',
            Argument {
                flags: vec!["--bar-command".into(), "-C".into()],
                // py:27  default='lemonbar',
                action: ArgAction::Store,
                // py:28  metavar='CMD', help='Name of the lemonbar executable to use.'
                metavar: Some("CMD".into()),
                // py:29  )
                help: "Name of the lemonbar executable to use.".to_string(),
            },
            // py:30  parser.add_argument(
            // py:31  'args', nargs=argparse.REMAINDER,
            Argument {
                flags: vec!["args".into()],
                action: ArgAction::Store,
                metavar: None,
                // py:32  help='Extra arguments for lemonbar. Should be preceded with ``--`` '
                // py:33  'argument in order not to be confused with script own arguments.'
                // py:34  )
                help: "Extra arguments for lemonbar. Should be preceded with ``--`` \
                       argument in order not to be confused with script own arguments."
                    .to_string(),
            },
        ],
    }
    // py:35  return parser
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
