// vim:fileencoding=utf-8:noet
//! Port of `powerline/commands/main.py`.
//!
//! Main CLI entry point logic: argument finalisation, `int_or_sig`
//! arg-type parser, and (deferred) the full `get_argparser` /
//! `write_output` orchestrators.
//!
//! This first chunk ports `arg_to_unicode`, `int_or_sig`, and
//! `finish_args` — all three use already-ported primitives (overrides,
//! mergeargs). `get_argparser` and `write_output` land alongside
//! `Powerline.__init__` since they depend on the renderer + binding
//! infrastructure.

// from __future__ import (division, absolute_import, print_function)  // py:3
// import argparse                                  // py:5
// import sys                                       // py:6
// from itertools import chain                      // py:8
// from powerline.lib.overrides import parsedotval, parse_override_var  // py:10
// from powerline.lib.dict import mergeargs                              // py:11
// from powerline.lib.encoding import get_preferred_arguments_encoding   // py:12
// from powerline.lib.unicode import u, unicode                          // py:13
// from powerline.bindings.wm import wm_threads                          // py:14

use crate::ported::lib::dict::mergeargs;
use crate::ported::lib::overrides::{parse_override_var, parsedotval_str};
use crate::ported::lib::unicode::u;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Port of `arg_to_unicode()` from `powerline/commands/main.py:20` /
/// `:23`.
///
/// Python 2: decode bytes → unicode using preferred encoding.
/// Python 3: identity passthrough (every str is already unicode).
///
/// The Rust port matches the Python 3 branch — every `&str` is already
/// valid UTF-8 by construction.
pub fn arg_to_unicode(s: &str) -> String {
    // py:23
    // py:24  return s
    u(s)
}

/// Port of `int_or_sig()` from `powerline/commands/main.py:75`.
///
/// Python:
/// ```python
/// def int_or_sig(s):
///     if s.startswith('sig'):
///         return u(s)
///     else:
///         return int(s)
/// ```
///
/// Returns the value as either a signal name string ("sigINT", "sigTERM")
/// or an integer exit code. Used by `--last-exit-code` / `--last-pipe-status`.
///
/// Rust port returns `IntOrSig` enum carrying the same shape.
#[derive(Debug, Clone, PartialEq)]
pub enum IntOrSig {
    Sig(String),
    Int(i32),
}

pub fn int_or_sig(s: &str) -> Result<IntOrSig, String> {
    // py:75
    if s.starts_with("sig") {
        // py:76
        Ok(IntOrSig::Sig(u(s))) // py:77  return u(s)
    } else {
        // py:78
        // py:79  return int(s)
        s.parse::<i32>()
            .map(IntOrSig::Int)
            .map_err(|e| format!("int_or_sig: cannot parse {:?}: {}", s, e))
    }
}

/// Parsed-args representation used by `finish_args` — mirrors the
/// argparse Namespace populated by `get_argparser`.
///
/// Python passes a mutable `argparse.Namespace`. Rust uses a struct
/// owned by the caller. Same field names.
#[derive(Debug, Default, Clone)]
pub struct Args {
    /// Python: `args.config_override` — list of `KEY.KEY=VAL` strings.
    pub config_override: Option<Vec<String>>,
    /// Python: `args.theme_override`.
    pub theme_override: Option<Vec<String>>,
    /// Python: `args.renderer_arg`.
    pub renderer_arg: Option<Vec<String>>,
    /// Python: `args.config_path` — list of directory paths.
    pub config_path: Option<Vec<String>>,
    /// Python: `args.ext` — list of one entry like `["shell"]` or `["wm.dwm"]`.
    pub ext: Vec<String>,
    /// Python: `args.side` — "left", "right", "above", "aboveleft", or None.
    pub side: Option<String>,
    /// Python: `args.width`.
    pub width: Option<i32>,
    /// Python: `args.last_exit_code`.
    pub last_exit_code: Option<IntOrSig>,
    /// Python: `args.last_pipe_status` — list of IntOrSig.
    pub last_pipe_status: Vec<IntOrSig>,
    /// Python: `args.jobnum`.
    pub jobnum: Option<i32>,
    /// Python: `args.socket`.
    pub socket: Option<String>,
    /// After finish_args: merged config_override dict.
    pub config_override_merged: Option<Map<String, Value>>,
    /// After finish_args: merged theme_override dict.
    pub theme_override_merged: Option<Map<String, Value>>,
    /// After finish_args: merged renderer_arg dict.
    pub renderer_arg_merged: Option<Map<String, Value>>,
}

/// Port of `finish_args()` from `powerline/commands/main.py:27`.
///
/// Do some final transformations.
///
/// Transforms `*_override` arguments into dictionaries, adding
/// overrides from environment variables. Transforms `renderer_arg`
/// argument into dictionary as well, but only if it is true.
///
/// :param environ: Environment from which additional overrides should
///     be taken.
/// :param args: Arguments object returned by `get_argparser().parse_args()`.
///     Modified in-place.
///
/// :return: Object received as `args` argument.
pub fn finish_args(
    environ: &HashMap<String, String>,
    args: &mut Args,
    _is_daemon: bool,
) -> Result<(), String> {
    let empty = String::new();

    // py:43-46  config_override
    let config_env = environ.get("POWERLINE_CONFIG_OVERRIDES").unwrap_or(&empty);
    let mut config_chain: Vec<(String, Value)> = parse_override_var(config_env);
    if let Some(cfg) = args.config_override.as_ref() {
        for v in cfg {
            if let Ok(pair) = parsedotval_str(v) {
                config_chain.push(pair);
            }
        }
    }
    args.config_override_merged = mergeargs(config_chain, false); // py:43

    // py:47-50  theme_override
    let theme_env = environ.get("POWERLINE_THEME_OVERRIDES").unwrap_or(&empty);
    let mut theme_chain: Vec<(String, Value)> = parse_override_var(theme_env);
    if let Some(th) = args.theme_override.as_ref() {
        for v in th {
            if let Ok(pair) = parsedotval_str(v) {
                theme_chain.push(pair);
            }
        }
    }
    args.theme_override_merged = mergeargs(theme_chain, false);

    // py:51-60  renderer_arg
    if let Some(rargs) = args.renderer_arg.as_ref() {
        if !rargs.is_empty() {
            let renderer_chain: Vec<(String, Value)> = rargs
                .iter()
                .filter_map(|v| parsedotval_str(v).ok())
                .collect();
            let mut merged = mergeargs(renderer_chain, true).unwrap_or_default(); // py:52

            // py:53-58  pane_id parsing
            if let Some(pane_id) = merged.get("pane_id").cloned() {
                let parsed_pane = match &pane_id {
                    Value::String(s) => {
                        // py:55-58  int(s.lstrip(' %')) or pass on ValueError
                        let stripped = s.trim_start_matches(|c| c == ' ' || c == '%');
                        stripped.parse::<i64>().ok().map(Value::from)
                    }
                    _ => None,
                };
                if let Some(p) = parsed_pane {
                    merged.insert("pane_id".to_string(), p.clone());
                    // py:59-60  client_id default to pane_id
                    if !merged.contains_key("client_id") {
                        merged.insert("client_id".to_string(), p);
                    }
                } else if !merged.contains_key("client_id") {
                    merged.insert("client_id".to_string(), pane_id);
                }
            }
            args.renderer_arg_merged = Some(merged);
        }
    }

    // py:61-64  config_path: env var paths + args.config_path
    let cp_env = environ.get("POWERLINE_CONFIG_PATHS").unwrap_or(&empty);
    let mut paths: Vec<String> = cp_env
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if let Some(extra) = args.config_path.take() {
        paths.extend(extra);
    }
    args.config_path = if paths.is_empty() { None } else { Some(paths) };

    // py:65-71  ext / side validation
    let ext0 = args.ext.first().cloned().unwrap_or_default();
    if ext0.starts_with("wm.") { // py:65
         // py:66-69  WM bindings require daemon; wm_threads check deferred
         // until bindings/wm/ ports.
    } else if args.side.is_none() {
        // py:70
        return Err("expected one argument".to_string()); // py:71
    }

    Ok(()) // py:72
}

/// Port of `get_argparser()` from `powerline/commands/main.py:82`.
///
/// Returns the argument parser for the main `powerline` binary.
///
/// Python builds an argparse.ArgumentParser with ~10 arguments:
/// positional `ext` + `side`, plus flags for `--renderer-module`,
/// `--width`, `--last-exit-code`, `--last-pipe-status`, `--jobnum`,
/// `--config-override`, `--theme-override`, `--renderer-arg`,
/// `--config-path`, `--socket`.
pub fn get_argparser() -> crate::ported::commands::lint::ArgParser {
    use crate::ported::commands::lint::{ArgAction, ArgParser, Argument};
    use crate::ported::bindings::wm::wm_threads;

    // py:88  ', '.join(('`wm.' + key + '`' for key in wm_threads.keys()))
    let wm_keys: Vec<String> = wm_threads()
        .keys()
        .map(|k| format!("`wm.{}'", k))
        .collect();
    let wm_help = wm_keys.join(", ");

    ArgParser {
        description: "Powerline prompt and statusline script.".to_string(), // py:83
        arguments: vec![
            // py:84-89  ext (positional, nargs=1)
            Argument {
                flags: vec!["ext".into()],
                action: ArgAction::Store,
                metavar: None,
                help: format!(
                    "Extension: application for which powerline command is launched \
                     (usually `shell' or `tmux'). Also supports `wm.' extensions: {}.",
                    wm_help
                ),
            },
            // py:90-97  side (positional, nargs='?', choices=...)
            Argument {
                flags: vec!["side".into()],
                action: ArgAction::Store,
                metavar: None,
                help: "Side: `left' and `right' represent left and right side \
                       respectively, `above' emits lines that are supposed to be printed \
                       just above the prompt and `aboveleft' is like concatenating \
                       `above' with `left' with the exception that only one Python \
                       instance is used in this case. May be omitted for `wm.*' extensions.".into(),
            },
            // py:98-105  -r / --renderer-module
            Argument {
                flags: vec!["-r".into(), "--renderer-module".into()],
                action: ArgAction::Store,
                metavar: Some("MODULE".into()),
                help: "Renderer module. Usually something like `.bash' or `.zsh' \
                       (with leading dot) which is `powerline.renderers.{ext}{MODULE}', \
                       may also be full module name (must contain at least one dot or \
                       end with a dot in case it is top-level module) or \
                       `powerline.renderers' submodule (in case there are no dots).".into(),
            },
            // py:106-109  -w / --width
            Argument {
                flags: vec!["-w".into(), "--width".into()],
                action: ArgAction::Store,
                metavar: None,
                help: "Maximum prompt with. Triggers truncation of some segments.".into(),
            },
            // py:110-113  --last-exit-code
            Argument {
                flags: vec!["--last-exit-code".into()],
                action: ArgAction::Store,
                metavar: Some("INT".into()),
                help: "Last exit code.".into(),
            },
            // py:114-119  --last-pipe-status
            Argument {
                flags: vec!["--last-pipe-status".into()],
                action: ArgAction::Store,
                metavar: Some("LIST".into()),
                help: "Like above, but is supposed to contain space-separated array \
                       of statuses, representing exit statuses of commands in one pipe.".into(),
            },
            // py:120-123  --jobnum
            Argument {
                flags: vec!["--jobnum".into()],
                action: ArgAction::Store,
                metavar: Some("INT".into()),
                help: "Number of jobs.".into(),
            },
            // py:124-135  -c / --config-override
            Argument {
                flags: vec!["-c".into(), "--config-override".into()],
                action: ArgAction::Append,
                metavar: Some("KEY.KEY=VALUE".into()),
                help: "Configuration overrides for `config.json'. Is translated to a \
                       dictionary and merged with the dictionary obtained from actual \
                       JSON configuration: KEY.KEY=VALUE is translated to \
                       `{\"KEY\": {\"KEY\": VALUE}}' and then merged recursively. \
                       VALUE may be any JSON value, values that are not \
                       `null', `true', `false', start with digit, `{', `[' \
                       are treated like strings. If VALUE is omitted \
                       then corresponding key is removed.".into(),
            },
            // py:136-142  -t / --theme-override
            Argument {
                flags: vec!["-t".into(), "--theme-override".into()],
                action: ArgAction::Append,
                metavar: Some("THEME.KEY.KEY=VALUE".into()),
                help: "Like above, but theme-specific. THEME should point to \
                       an existing and used theme to have any effect, but it is fine \
                       to use any theme here.".into(),
            },
            // py:143-151  -R / --renderer-arg
            Argument {
                flags: vec!["-R".into(), "--renderer-arg".into()],
                action: ArgAction::Append,
                metavar: Some("KEY=VAL".into()),
                help: "Like above, but provides argument for renderer. Is supposed \
                       to be used only by shell bindings to provide various data like \
                       last-exit-code or last-pipe-status (they are not using \
                       `--renderer-arg' for historical reasons: `--renderer-arg' \
                       was added later).".into(),
            },
            // py:152-157  -p / --config-path
            Argument {
                flags: vec!["-p".into(), "--config-path".into()],
                action: ArgAction::Append,
                metavar: Some("PATH".into()),
                help: "Path to configuration directory. If it is present then \
                       configuration files will only be sought in the provided path. \
                       May be provided multiple times to search in a list of directories.".into(),
            },
            // py:158-166  --socket
            Argument {
                flags: vec!["--socket".into()],
                action: ArgAction::Store,
                metavar: Some("ADDRESS".into()),
                help: "Socket address to use in daemon clients. Is always UNIX domain \
                       socket on linux and file socket on Mac OS X. Not used here, \
                       present only for compatibility with other powerline clients. \
                       This argument must always be the first one and be in a form \
                       `--socket ADDRESS': no `=' or short form allowed \
                       (in other powerline clients, not here).".into(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn arg_to_unicode_is_identity_in_py3() {
        assert_eq!(arg_to_unicode("hello"), "hello");
        assert_eq!(arg_to_unicode("héllo"), "héllo");
    }

    #[test]
    fn int_or_sig_parses_integer() {
        assert_eq!(int_or_sig("42").unwrap(), IntOrSig::Int(42));
        assert_eq!(int_or_sig("-1").unwrap(), IntOrSig::Int(-1));
        assert_eq!(int_or_sig("0").unwrap(), IntOrSig::Int(0));
    }

    #[test]
    fn int_or_sig_parses_signal_name() {
        assert_eq!(
            int_or_sig("sigINT").unwrap(),
            IntOrSig::Sig("sigINT".into())
        );
        assert_eq!(
            int_or_sig("sigTERM").unwrap(),
            IntOrSig::Sig("sigTERM".into())
        );
    }

    #[test]
    fn int_or_sig_rejects_invalid_int() {
        assert!(int_or_sig("not-a-number").is_err());
    }

    #[test]
    fn finish_args_validates_ext_side() {
        let env = HashMap::new();
        let mut args = Args {
            ext: vec!["shell".to_string()],
            side: None,
            ..Default::default()
        };
        let r = finish_args(&env, &mut args, false);
        assert!(
            r.is_err(),
            "expected validation error when ext=shell + no side"
        );
    }

    #[test]
    fn finish_args_accepts_wm_ext_without_side() {
        let env = HashMap::new();
        let mut args = Args {
            ext: vec!["wm.dwm".to_string()],
            side: None,
            ..Default::default()
        };
        let r = finish_args(&env, &mut args, true);
        assert!(r.is_ok());
    }

    #[test]
    fn finish_args_merges_config_override() {
        let env = HashMap::new();
        let mut args = Args {
            ext: vec!["shell".to_string()],
            side: Some("left".to_string()),
            config_override: Some(vec!["ext.shell.theme=default".to_string()]),
            ..Default::default()
        };
        finish_args(&env, &mut args, false).unwrap();
        let merged = args
            .config_override_merged
            .expect("config_override should be merged");
        assert_eq!(
            merged.get("ext"),
            Some(&json!({"shell": {"theme": "default"}}))
        );
    }

    #[test]
    fn finish_args_combines_env_and_arg_config_path() {
        let mut env = HashMap::new();
        env.insert(
            "POWERLINE_CONFIG_PATHS".into(),
            "/etc/powerline:/opt/powerline".into(),
        );
        let mut args = Args {
            ext: vec!["shell".to_string()],
            side: Some("left".to_string()),
            config_path: Some(vec!["/home/user/.config/powerline".to_string()]),
            ..Default::default()
        };
        finish_args(&env, &mut args, false).unwrap();
        let paths = args.config_path.expect("config_path should be populated");
        assert_eq!(
            paths,
            vec![
                "/etc/powerline".to_string(),
                "/opt/powerline".to_string(),
                "/home/user/.config/powerline".to_string(),
            ]
        );
    }

    #[test]
    fn finish_args_merges_renderer_arg_with_pane_id_int_parsing() {
        let env = HashMap::new();
        let mut args = Args {
            ext: vec!["shell".to_string()],
            side: Some("left".to_string()),
            renderer_arg: Some(vec!["pane_id=%42".to_string()]),
            ..Default::default()
        };
        finish_args(&env, &mut args, false).unwrap();
        let merged = args
            .renderer_arg_merged
            .expect("renderer_arg should be merged");
        // Python: int("42") = 42; lstrip(' %') strips both.
        assert_eq!(merged.get("pane_id"), Some(&json!(42)));
        assert_eq!(merged.get("client_id"), Some(&json!(42)));
    }

    #[test]
    fn get_argparser_has_expected_argument_count() {
        let p = get_argparser();
        // py:84-166 — ext + side + 10 flag arguments = 12 total
        assert_eq!(p.arguments.len(), 12);
    }

    #[test]
    fn get_argparser_description_matches_upstream() {
        let p = get_argparser();
        assert_eq!(p.description, "Powerline prompt and statusline script.");
    }

    #[test]
    fn get_argparser_has_required_flags() {
        let p = get_argparser();
        let all_flags: Vec<&str> = p.arguments.iter()
            .flat_map(|a| a.flags.iter().map(|s| s.as_str()))
            .collect();
        for required in [
            "ext", "side", "-r", "--renderer-module", "-w", "--width",
            "--last-exit-code", "--last-pipe-status", "--jobnum",
            "-c", "--config-override", "-t", "--theme-override",
            "-R", "--renderer-arg", "-p", "--config-path", "--socket",
        ] {
            assert!(all_flags.contains(&required),
                "missing required arg {:?} from get_argparser output", required);
        }
    }
}
