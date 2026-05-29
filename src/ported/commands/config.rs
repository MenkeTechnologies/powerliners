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

    /// Returns the fully-qualified binding function name that this
    /// StrFunction wraps, per py:22-30. Used by the dispatch site to
    /// invoke the right `powerline.bindings.config` fn.
    pub fn binding_function_name(&self) -> &'static str {
        match self {
            // py:22  config.source_tmux_files
            StrFunction::Source => "source_tmux_files",
            // py:23  config.init_tmux_environment
            StrFunction::Setenv => "init_tmux_environment",
            // py:24  config.tmux_setup
            StrFunction::Setup => "tmux_setup",
            // py:29  config.shell_command
            StrFunction::Command => "shell_command",
            // py:30  config.uses
            StrFunction::Uses => "uses",
        }
    }

    /// Port of `StrFunction.__call__()` from
    /// `powerline/commands/config.py:14-15`.
    ///
    /// Python: `self.function(*args, **kwargs)`. The Rust port returns
    /// the bound fn name + the (pl, args) tuple it would dispatch to;
    /// actual execution happens at the caller since the underlying
    /// binding functions (source_tmux_files / init_tmux_environment /
    /// tmux_setup / shell_command / uses) are platform-glue with
    /// runtime tmux/socket dependencies and are deferred.
    pub fn call(&self) -> &'static str {
        // py:14-15  self.function(*args, **kwargs)
        self.binding_function_name()
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

/// Port of `class ConfigArgParser(argparse.ArgumentParser)` from
/// `powerline/commands/config.py:34-42`.
///
/// Python: subclass of `ArgumentParser` whose `parse_args` override
/// raises "too few arguments" when the parsed result has no
/// `function` attribute. Rust port wraps the data-only `ArgParser`
/// from `commands/lint.rs` and provides the same validation.
pub struct ConfigArgParser {
    /// The underlying argparse spec — same data shape as
    /// `commands::lint::ArgParser`.
    pub argparser: ArgParser,
}

impl ConfigArgParser {
    /// Construct from a base `ArgParser` (typically the value returned
    /// by `get_argparser`).
    pub fn new(argparser: ArgParser) -> Self {
        Self { argparser }
    }

    /// Port of `ConfigArgParser.parse_args()` from
    /// `powerline/commands/config.py:35-42`.
    ///
    /// Validates that the parsed args carry a `function` field. Python
    /// at py:37 checks `hasattr(ret, 'function')`; the Rust port
    /// receives the resolved function name (or None for absent) and
    /// returns an error matching Python's `self.error('too few
    /// arguments')` at py:41.
    pub fn parse_args(&self, function: Option<&str>) -> Result<String, String> {
        // py:35-42
        match function {
            // py:37-41  if not hasattr(ret, 'function'): self.error(...)
            None => Err("too few arguments".to_string()),
            Some(f) => Ok(f.to_string()),
        }
    }
}

/// Port of `get_argparser()` from `powerline/commands/config.py:45`.
///
/// Returns the argument parser for `powerline-config`.
///
/// Python uses sub-parsers (`tmux` / `shell`) with their own arguments.
/// The Rust port flattens them into one `ArgParser` returning the full
/// flag set; the caller (CLI driver) dispatches on the sub-command.
pub fn get_argparser() -> ArgParser {
    // py:45  def get_argparser(ArgumentParser=ConfigArgParser):
    ArgParser {
        // py:46  parser = ArgumentParser(description='Script used to obtain powerline configuration.')
        description: "Script used to obtain powerline configuration.".to_string(),
        arguments: vec![
            // py:47  parser.add_argument(
            // py:48  '-p', '--config-path', action='append', metavar='PATH',
            Argument {
                flags: vec!["-p".into(), "--config-path".into()],
                action: ArgAction::Append,
                metavar: Some("PATH".into()),
                // py:49  help='Path to configuration directory. If it is present '
                // py:50  'then configuration files will only be sought in the provided path. '
                // py:51  'May be provided multiple times to search in a list of directories.'
                // py:52  )
                help: "Path to configuration directory. If it is present \
                       then configuration files will only be sought in the provided path. \
                       May be provided multiple times to search in a list of directories."
                    .into(),
            },
            // py:53  subparsers = parser.add_subparsers()
            // py:54  tmux_parser = subparsers.add_parser('tmux', help='Tmux-specific commands')
            // py:55  tmux_parser.add_argument(
            // py:56  'function',
            // py:57  choices=tuple(TMUX_ACTIONS.values()),
            // py:58  metavar='ACTION',
            // py:59  type=(lambda v: TMUX_ACTIONS.get(v)),
            Argument {
                flags: vec!["tmux".into()],
                action: ArgAction::Store,
                metavar: Some("ACTION".into()),
                // py:60  help='If action is `source\' then version-specific tmux configuration '
                // py:61  'files are sourced, if it is `setenv\' then special '
                // py:62  '(prefixed with `_POWERLINE\') tmux global environment variables '
                // py:63  'are filled with data from powerline configuration. '
                // py:64  'Action `setup\' is just doing `setenv\' then `source\'.'
                // py:65  )
                help: "Tmux-specific commands: source / setenv / setup. \
                       If action is `source' then version-specific tmux configuration \
                       files are sourced, if it is `setenv' then special \
                       (prefixed with `_POWERLINE') tmux global environment variables \
                       are filled with data from powerline configuration. \
                       Action `setup' is just doing `setenv' then `source'."
                    .into(),
            },
            // py:66  tpg = tmux_parser.add_mutually_exclusive_group()
            // py:67  tpg.add_argument(
            // py:68  '-s', '--source', action='store_true', default=None,
            Argument {
                flags: vec!["-s".into(), "--source".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                // py:69  help='When using `setup\': always use configuration file sourcing. '
                // py:70  'By default this is determined automatically based on tmux '
                // py:71  'version: this is the default for tmux 1.8 and below.',
                // py:72  )
                help: "When using `setup': always use configuration file sourcing. \
                       By default this is determined automatically based on tmux \
                       version: this is the default for tmux 1.8 and below."
                    .into(),
            },
            // py:73  tpg.add_argument(
            // py:74  '-n', '--no-source', action='store_false', dest='source', default=None,
            Argument {
                flags: vec!["-n".into(), "--no-source".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                // py:75  help='When using `setup\': in place of sourcing directly execute '
                // py:76  'configuration files. That is, read each needed '
                // py:77  'powerline-specific configuration file, substitute '
                // py:78  '`$_POWERLINE_…\' variables with appropriate values and run '
                // py:79  '`tmux config line\'. This is the default behaviour for '
                // py:80  'tmux 1.9 and above.'
                // py:81  )
                help: "When using `setup': in place of sourcing directly execute \
                       configuration files. That is, read each needed \
                       powerline-specific configuration file, substitute \
                       `$_POWERLINE_...' variables with appropriate values and run \
                       `tmux config line'. This is the default behaviour for \
                       tmux 1.9 and above."
                    .into(),
            },
            // py:83  shell_parser = subparsers.add_parser('shell', help='Shell-specific commands')
            // py:84  shell_parser.add_argument(
            // py:85  'function',
            // py:86  choices=tuple(SHELL_ACTIONS.values()),
            // py:87  type=(lambda v: SHELL_ACTIONS.get(v)),
            // py:88  metavar='ACTION',
            Argument {
                flags: vec!["shell".into()],
                action: ArgAction::Store,
                metavar: Some("ACTION".into()),
                // py:89  help='If action is `command\' then preferred powerline command is '
                // py:90  'output, if it is `uses\' then powerline-config script will exit '
                // py:91  'with 1 if specified component is disabled and 0 otherwise.',
                // py:92  )
                help: "Shell-specific commands: command / uses. \
                       If action is `command' then preferred powerline command is \
                       output, if it is `uses' then powerline-config script will exit \
                       with 1 if specified component is disabled and 0 otherwise."
                    .into(),
            },
            // py:93  shell_parser.add_argument(
            // py:94  'component',
            // py:95  nargs='?',
            // py:96  choices=('tmux', 'prompt'),
            // py:97  metavar='COMPONENT',
            Argument {
                flags: vec!["component".into()],
                action: ArgAction::Store,
                metavar: Some("COMPONENT".into()),
                // py:98  help='Only applicable for `uses\' subcommand: makes `powerline-config\' '
                // py:99  'exit with 0 if specific component is enabled and with 1 otherwise. '
                // py:100  '`tmux\' component stands for tmux bindings '
                // py:101  '(e.g. those that notify tmux about current directory changes), '
                // py:102  '`prompt\' component stands for shell prompt.'
                // py:103  )
                help: "Only applicable for `uses' subcommand: makes `powerline-config' \
                       exit with 0 if specific component is enabled and with 1 otherwise. \
                       `tmux' component stands for tmux bindings \
                       (e.g. those that notify tmux about current directory changes), \
                       `prompt' component stands for shell prompt."
                    .into(),
            },
            // py:104  shell_parser.add_argument(
            // py:105  '-s', '--shell',
            // py:106  metavar='SHELL',
            Argument {
                flags: vec!["--shell".into()],
                action: ArgAction::Store,
                metavar: Some("SHELL".into()),
                // py:107  help='Shell for which query is run',
                // py:108  )
                help: "Shell for which query is run".into(),
            },
        ],
    }
    // py:109  return parser
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
        assert_eq!(
            p.description,
            "Script used to obtain powerline configuration."
        );
    }

    #[test]
    fn get_argparser_includes_subcommand_handles() {
        let p = get_argparser();
        let flag_names: Vec<&str> = p
            .arguments
            .iter()
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

    #[test]
    fn shell_action_lookup_command_and_uses() {
        // py:87  SHELL_ACTIONS.get(v)
        assert_eq!(
            shell_action_from_name("command"),
            Some(StrFunction::Command)
        );
        assert_eq!(shell_action_from_name("uses"), Some(StrFunction::Uses));
        assert_eq!(shell_action_from_name("xyz"), None);
    }

    #[test]
    fn str_function_binding_names_match_python_module_attrs() {
        // py:22-30  function names in the powerline.bindings.config module
        assert_eq!(
            StrFunction::Source.binding_function_name(),
            "source_tmux_files"
        );
        assert_eq!(
            StrFunction::Setenv.binding_function_name(),
            "init_tmux_environment"
        );
        assert_eq!(StrFunction::Setup.binding_function_name(), "tmux_setup");
        assert_eq!(
            StrFunction::Command.binding_function_name(),
            "shell_command"
        );
        assert_eq!(StrFunction::Uses.binding_function_name(), "uses");
    }

    #[test]
    fn str_function_call_returns_binding_function_name() {
        // py:14-15  self.function(*args, **kwargs) — Rust returns the
        // name of the fn it would invoke at the call site.
        assert_eq!(StrFunction::Source.call(), "source_tmux_files");
        assert_eq!(StrFunction::Uses.call(), "uses");
    }

    #[test]
    fn config_arg_parser_parse_args_requires_function() {
        // py:37-41  if not hasattr(ret, 'function'): self.error('too few arguments')
        let parser = ConfigArgParser::new(get_argparser());
        let err = parser.parse_args(None).unwrap_err();
        assert_eq!(err, "too few arguments");
    }

    #[test]
    fn config_arg_parser_parse_args_returns_function_when_set() {
        let parser = ConfigArgParser::new(get_argparser());
        let r = parser.parse_args(Some("tmux")).unwrap();
        assert_eq!(r, "tmux");
    }

    #[test]
    fn config_arg_parser_round_trips_underlying_argparser() {
        // The ConfigArgParser wrapper preserves the underlying
        // argparser state for subsequent introspection.
        let underlying = get_argparser();
        let saved_desc = underlying.description.clone();
        let saved_args_count = underlying.arguments.len();
        let parser = ConfigArgParser::new(underlying);
        assert_eq!(parser.argparser.description, saved_desc);
        assert_eq!(parser.argparser.arguments.len(), saved_args_count);
    }
}
