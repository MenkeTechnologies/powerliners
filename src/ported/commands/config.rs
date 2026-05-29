// vim:fileencoding=utf-8:noet
//! Port of `powerline/commands/config.py`.
//!
//! `powerline-config` binary arg-spec + action dispatcher. Used by:
//!   - `powerline-config tmux source`   — source version-specific tmux confs
//!   - `powerline-config tmux setenv`   — set `_POWERLINE_*` tmux env vars
//!   - `powerline-config tmux setup`    — setenv then source
//!   - `powerline-config shell command` — print preferred shell binding cmd
//!   - `powerline-config shell uses tmux|prompt` — exit 0 if component enabled

// from __future__ import (division, absolute_import, print_function)  // py:2
// import argparse                                  // py:4
// import powerline.bindings.config as config       // py:6

use crate::ported::commands::lint::{ArgAction, ArgParser, Argument};

/// Port of `class StrFunction` from `powerline/commands/config.py:9`.
///
/// Python: a callable wrapper that also has a `__str__` returning the
/// name. Used so argparse `choices=(StrFunction, ...)` shows the
/// action name in help / error messages.
///
/// Rust port: an enum tagging the action by its upstream name.
/// Callers dispatch via `match`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrFunction {
    // py:22  'source': StrFunction(config.source_tmux_files, 'source')
    Source,
    // py:23  'setenv': StrFunction(config.init_tmux_environment, 'setenv')
    Setenv,
    // py:24  'setup': StrFunction(config.tmux_setup, 'setup')
    Setup,
    // py:29  'command': StrFunction(config.shell_command, 'command')
    Command,
    // py:30  'uses': StrFunction(config.uses) — default name from fn __name__
    Uses,
}

impl StrFunction {
    /// Python `__str__` returns `self.name`. Mirrors the dispatch
    /// label shown by argparse.
    pub fn name(&self) -> &'static str {
        match self {
            StrFunction::Source => "source",
            StrFunction::Setenv => "setenv",
            StrFunction::Setup => "setup",
            StrFunction::Command => "command",
            StrFunction::Uses => "uses",
        }
    }
}

impl std::fmt::Display for StrFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // py:17-18  def __str__(self): return self.name
        write!(f, "{}", self.name())
    }
}

/// Port of module-level binding `TMUX_ACTIONS` from
/// `powerline/commands/config.py:21`.
///
/// Returns the available tmux subcommand names (source / setenv / setup).
#[allow(non_snake_case)]
pub fn TMUX_ACTIONS() -> &'static [StrFunction] {
    &[StrFunction::Source, StrFunction::Setenv, StrFunction::Setup]
}

/// Port of module-level binding `SHELL_ACTIONS` from
/// `powerline/commands/config.py:28`.
///
/// Returns the available shell subcommand names (command / uses).
#[allow(non_snake_case)]
pub fn SHELL_ACTIONS() -> &'static [StrFunction] {
    &[StrFunction::Command, StrFunction::Uses]
}

/// Look up a `StrFunction` by name (returns the `'tmux'` action).
///
/// Mirrors Python's `TMUX_ACTIONS.get(v)` at py:59.
pub fn tmux_action_from_name(name: &str) -> Option<StrFunction> {
    match name {
        "source" => Some(StrFunction::Source),
        "setenv" => Some(StrFunction::Setenv),
        "setup" => Some(StrFunction::Setup),
        _ => None,
    }
}

/// Look up a `StrFunction` by name (returns the `'shell'` action).
///
/// Mirrors Python's `SHELL_ACTIONS.get(v)` at py:87.
pub fn shell_action_from_name(name: &str) -> Option<StrFunction> {
    match name {
        "command" => Some(StrFunction::Command),
        "uses" => Some(StrFunction::Uses),
        _ => None,
    }
}

// py:34-42  class ConfigArgParser(argparse.ArgumentParser):
//   override parse_args to require a sub-command (raise "too few arguments")
// The Rust port handles this at the caller — when the parsed Args has
// no function set, the CLI prints "too few arguments" and exits.
// No separate type needed.

/// Port of `get_argparser()` from `powerline/commands/config.py:45`.
///
/// Returns the argument parser for `powerline-config`.
///
/// Python uses sub-parsers (`tmux` / `shell`) with their own arguments.
/// The Rust port flattens them into one `ArgParser` returning the full
/// flag set; the caller (CLI driver) dispatches on the sub-command.
pub fn get_argparser() -> ArgParser {
    ArgParser {
        description: "Script used to obtain powerline configuration.".to_string(), // py:46
        arguments: vec![
            // py:47-52  -p / --config-path (top-level)
            Argument {
                flags: vec!["-p".into(), "--config-path".into()],
                action: ArgAction::Append,
                metavar: Some("PATH".into()),
                help: "Path to configuration directory. If it is present \
                       then configuration files will only be sought in the provided path. \
                       May be provided multiple times to search in a list of directories.".into(),
            },
            // py:54  tmux sub-parser dispatch (function positional)
            Argument {
                flags: vec!["tmux".into()],
                action: ArgAction::Store,
                metavar: Some("ACTION".into()),
                help: "Tmux-specific commands: source / setenv / setup. \
                       If action is `source' then version-specific tmux configuration \
                       files are sourced, if it is `setenv' then special \
                       (prefixed with `_POWERLINE') tmux global environment variables \
                       are filled with data from powerline configuration. \
                       Action `setup' is just doing `setenv' then `source'.".into(),
            },
            // py:67-72  -s / --source (tmux sub-parser)
            Argument {
                flags: vec!["-s".into(), "--source".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "When using `setup': always use configuration file sourcing. \
                       By default this is determined automatically based on tmux \
                       version: this is the default for tmux 1.8 and below.".into(),
            },
            // py:73-81  -n / --no-source (tmux sub-parser)
            Argument {
                flags: vec!["-n".into(), "--no-source".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "When using `setup': in place of sourcing directly execute \
                       configuration files. That is, read each needed \
                       powerline-specific configuration file, substitute \
                       `$_POWERLINE_...' variables with appropriate values and run \
                       `tmux config line'. This is the default behaviour for \
                       tmux 1.9 and above.".into(),
            },
            // py:83  shell sub-parser dispatch (function positional)
            Argument {
                flags: vec!["shell".into()],
                action: ArgAction::Store,
                metavar: Some("ACTION".into()),
                help: "Shell-specific commands: command / uses. \
                       If action is `command' then preferred powerline command is \
                       output, if it is `uses' then powerline-config script will exit \
                       with 1 if specified component is disabled and 0 otherwise.".into(),
            },
            // py:93-103  component (positional for shell uses)
            Argument {
                flags: vec!["component".into()],
                action: ArgAction::Store,
                metavar: Some("COMPONENT".into()),
                help: "Only applicable for `uses' subcommand: makes `powerline-config' \
                       exit with 0 if specific component is enabled and with 1 otherwise. \
                       `tmux' component stands for tmux bindings \
                       (e.g. those that notify tmux about current directory changes), \
                       `prompt' component stands for shell prompt.".into(),
            },
            // py:104-108  -s / --shell (shell sub-parser)
            Argument {
                flags: vec!["--shell".into()],
                action: ArgAction::Store,
                metavar: Some("SHELL".into()),
                help: "Shell for which query is run".into(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_function_names_match_upstream() {
        assert_eq!(StrFunction::Source.name(), "source");
        assert_eq!(StrFunction::Setenv.name(), "setenv");
        assert_eq!(StrFunction::Setup.name(), "setup");
        assert_eq!(StrFunction::Command.name(), "command");
        assert_eq!(StrFunction::Uses.name(), "uses");
    }

    #[test]
    fn str_function_display_round_trips_via_name() {
        let s = format!("{}", StrFunction::Source);
        assert_eq!(s, "source");
        assert_eq!(tmux_action_from_name(&s), Some(StrFunction::Source));
    }

    #[test]
    fn tmux_actions_has_three_entries() {
        assert_eq!(TMUX_ACTIONS().len(), 3);
        assert!(TMUX_ACTIONS().contains(&StrFunction::Source));
        assert!(TMUX_ACTIONS().contains(&StrFunction::Setenv));
        assert!(TMUX_ACTIONS().contains(&StrFunction::Setup));
    }

    #[test]
    fn shell_actions_has_two_entries() {
        assert_eq!(SHELL_ACTIONS().len(), 2);
        assert!(SHELL_ACTIONS().contains(&StrFunction::Command));
        assert!(SHELL_ACTIONS().contains(&StrFunction::Uses));
    }

    #[test]
    fn tmux_action_lookup_unknown_returns_none() {
        assert_eq!(tmux_action_from_name("xyz"), None);
    }

    #[test]
    fn get_argparser_description_matches_upstream() {
        let p = get_argparser();
        assert_eq!(p.description, "Script used to obtain powerline configuration.");
    }

    #[test]
    fn get_argparser_includes_subcommand_handles() {
        let p = get_argparser();
        let flag_names: Vec<&str> = p.arguments.iter()
            .flat_map(|a| a.flags.iter().map(|s| s.as_str()))
            .collect();
        // Top-level config-path
        assert!(flag_names.contains(&"--config-path"));
        // Sub-command positional names
        assert!(flag_names.contains(&"tmux"));
        assert!(flag_names.contains(&"shell"));
        // Component positional for `shell uses`
        assert!(flag_names.contains(&"component"));
    }
}
