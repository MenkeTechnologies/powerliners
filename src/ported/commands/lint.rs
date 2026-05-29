// vim:fileencoding=utf-8:noet
//! Port of `powerline/commands/lint.py`.
//!
//! Argument-spec for the `powerline-lint` binary. Upstream uses
//! argparse; the Rust port returns the same spec as a structured
//! `ArgParser` value that downstream CLI code consumes.

// from __future__ import (division, absolute_import, print_function)  // py:2
// import argparse                                  // py:4

/// Minimal argparse-like spec used by the `commands/*.py` ports.
///
/// Mirrors the subset of `argparse.ArgumentParser` that powerline
/// uses: short+long flags, action types (`store_true` / `store_const` /
/// `append`), help text, and a description.
///
/// Downstream callers convert this into a real CLI parser (clap, or
/// hand-rolled). Keeping it as data preserves the upstream surface
/// without dragging clap in as a dependency for the library crate.
#[derive(Debug, Clone)]
pub struct ArgParser {
    pub description: String,
    pub arguments: Vec<Argument>,
}

/// One argument specification (mirrors a single argparse `add_argument` call).
#[derive(Debug, Clone)]
pub struct Argument {
    pub flags: Vec<String>, // e.g. ["-p", "--config-path"]
    pub action: ArgAction,
    pub metavar: Option<String>,
    pub help: String,
}

/// Mirrors the subset of argparse `action=` values powerline uses.
#[derive(Debug, Clone, PartialEq)]
pub enum ArgAction {
    /// Default — value-bearing argument.
    Store,
    /// `action='store_true'` — flag, true if present.
    StoreTrue,
    /// `action='store_const', const=True` — same as StoreTrue.
    StoreConstTrue,
    /// `action='append'` — collect multiple occurrences into a list.
    Append,
}

/// Port of `get_argparser()` from `powerline/commands/lint.py:7`.
///
/// Returns the argument parser for `powerline-lint`.
pub fn get_argparser() -> ArgParser {
    // py:7  def get_argparser(ArgumentParser=argparse.ArgumentParser)
    ArgParser {
        // py:8  parser = ArgumentParser(description='Powerline configuration checker.')
        description: "Powerline configuration checker.".to_string(),
        arguments: vec![
            // py:9   parser.add_argument(
            // py:10  '-p', '--config-path', action='append', metavar='PATH',
            Argument {
                flags: vec!["-p".into(), "--config-path".into()],
                action: ArgAction::Append,
                metavar: Some("PATH".into()),
                // py:11  help='Paths where configuration should be checked, in order. You must '
                // py:12  'supply all paths necessary for powerline to work, '
                // py:13  'checking partial (e.g. only user overrides) configuration '
                // py:14  'is not supported.'
                // py:15  )
                help: "Paths where configuration should be checked, in order. You must \
                       supply all paths necessary for powerline to work, \
                       checking partial (e.g. only user overrides) configuration \
                       is not supported."
                    .to_string(),
            },
            // py:16  parser.add_argument(
            // py:17  '-d', '--debug', action='store_const', const=True,
            Argument {
                flags: vec!["-d".into(), "--debug".into()],
                action: ArgAction::StoreConstTrue,
                metavar: None,
                // py:18  help='Display additional information. Used for debugging '
                // py:19  '`powerline-lint\' itself, not for debugging configuration.'
                // py:20  )
                help: "Display additional information. Used for debugging \
                       `powerline-lint' itself, not for debugging configuration."
                    .to_string(),
            },
        ],
    }
    // py:21  return parser
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_parser_description_matches_upstream() {
        let p = get_argparser();
        assert_eq!(p.description, "Powerline configuration checker.");
    }

    #[test]
    fn lint_parser_has_two_arguments() {
        let p = get_argparser();
        assert_eq!(p.arguments.len(), 2);
        assert!(p.arguments[0].flags.contains(&"--config-path".to_string()));
        assert!(p.arguments[1].flags.contains(&"--debug".to_string()));
    }

    #[test]
    fn config_path_uses_append_action() {
        let p = get_argparser();
        assert_eq!(p.arguments[0].action, ArgAction::Append);
    }
}
