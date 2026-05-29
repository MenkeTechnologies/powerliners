// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/powerline-config`.
//!
//! Tiny CLI shim: build the argparser, parse args, create a
//! powerline logger, then dispatch the parsed sub-action via
//! `args.function(pl, args)`.

// #!/usr/bin/env python                              // sh:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // sh:3
// try:
//     from powerline.commands.config import get_argparser
// except ImportError:
//     ...
//     from powerline.commands.config import get_argparser     // sh:5-11
// import powerline.bindings.config as config                  // sh:13

use crate::ported::bindings::config as binding_config;
use crate::ported::commands::config::{get_argparser, StrFunction};

/// Port of the `if __name__ == '__main__':` block at
/// `vendor/powerline/scripts/powerline-config:16-22`.
///
/// Returns the integer exit code:
/// - `0` on successful dispatch
/// - `2` when no `--function` was selected on the command line
///
/// `args` is the raw `Vec<String>` from `std::env::args().skip(1).collect()`.
/// Mirrors Python's `parser.parse_args()` indirectly via the data-only
/// `ArgParser` in `ported::commands::config` — sub-action picked by
/// matching the first positional against `TMUX_ACTIONS` /
/// `SHELL_ACTIONS`.
///
/// `create_powerline_logger(args)` at sh:20 is not yet ported (depends
/// on `Powerline` class + `ConfigLoader`); the dispatch path skips the
/// logger arg since the leaf actions don't read it.
pub fn main(args: &[String]) -> i32 {
    // sh:17  parser = get_argparser()
    let _parser = get_argparser();

    // sh:18  args = parser.parse_args()
    // Minimal parser: look at args[0] for the sub-action.
    let action_name = match args.first() {
        Some(s) => s.as_str(),
        None => {
            eprintln!("powerline-config: missing function argument");
            return 2;
        }
    };

    // sh:20  pl = config.create_powerline_logger(args)
    // TODO: create_powerline_logger requires full Powerline class.
    // Surface the call shape so the script tree is structurally
    // complete; the binding helper used by sh:13 is wired in.
    let _binding = binding_config::deduce_command();

    // sh:22  args.function(pl, args)
    let resolved: Option<StrFunction> =
        crate::ported::commands::config::tmux_action_from_name(action_name)
            .or_else(|| crate::ported::commands::config::shell_action_from_name(action_name));
    match resolved {
        Some(StrFunction::Command) => {
            // bindings/config.py:264-269
            //   def shell_command(pl, args):
            //       cmd = deduce_command()
            //       if cmd: print(cmd) else: sys.exit(1)
            match binding_config::deduce_command() {
                Some(cmd) => {
                    println!("{}", cmd);
                    0
                }
                None => 1,
            }
        }
        Some(StrFunction::Uses) => {
            // bindings/config.py:272-286
            //   def uses(pl, args):
            //       component = args.component
            //       if not component: raise ValueError(...)
            //       template = 'POWERLINE_NO_{shell}_{component}'
            //       for sh in (shell, 'shell') if shell else ('shell',):
            //           varname = template.format(...)
            //           if os.environ.get(varname): sys.exit(1)
            //       (config check via Powerline class — deferred)
            // Args layout per ConfigArgParser: `uses COMPONENT [-s SHELL]`.
            let component = match args.get(1) {
                Some(c) => c.clone(),
                None => {
                    eprintln!("powerline-config uses: component required");
                    return 1;
                }
            };
            let mut shell: Option<String> = None;
            let mut i = 2;
            while i < args.len() {
                if args[i] == "-s" || args[i] == "--shell" {
                    if let Some(s) = args.get(i + 1) {
                        shell = Some(s.clone());
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let sh_list: Vec<String> = match shell.as_deref() {
                Some(s) => vec![s.to_string(), "shell".to_string()],
                None => vec!["shell".to_string()],
            };
            for sh in &sh_list {
                let varname = format!(
                    "POWERLINE_NO_{}_{}",
                    sh.to_uppercase(),
                    component.to_uppercase()
                );
                if std::env::var(&varname)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
                {
                    return 1;
                }
            }
            // py:282-285
            //   config = get_main_config(args)
            //   if component in config.get('ext', {}).get('shell', {})
            //                       .get('components', ('tmux', 'prompt')):
            //       sys.exit(0)
            //   else:
            //       sys.exit(1)
            // Load main config via the already-ported _find_config_files
            // + load_json_config + mergedicts cascade. Search paths come
            // from `--config-path` flags (per ConfigArgParser) or
            // POWERLINE_CONFIG_PATHS env, falling back to
            // get_config_paths() defaults.
            use crate::ported::lib::config::load_json_config;
            use crate::ported::lib::dict::mergedicts;
            use crate::ported::{_find_config_files, get_config_paths};
            let mut search_paths: Vec<std::path::PathBuf> = Vec::new();
            if let Ok(pcp) = std::env::var("POWERLINE_CONFIG_PATHS") {
                for p in pcp.split(':').filter(|s| !s.is_empty()) {
                    search_paths.push(std::path::PathBuf::from(p));
                }
            }
            // Read --config-path / -p from argv too.
            let mut j = 0;
            while j < args.len() {
                if args[j] == "--config-path" || args[j] == "-p" {
                    if let Some(p) = args.get(j + 1) {
                        search_paths.push(std::path::PathBuf::from(p));
                    }
                    j += 2;
                } else {
                    j += 1;
                }
            }
            search_paths.extend(get_config_paths());
            let mut config = serde_json::Map::new();
            if let Ok(matches) = _find_config_files(&search_paths, "config") {
                for path in matches {
                    if let Ok(v) = load_json_config(&path) {
                        if let Some(o) = v.as_object().cloned() {
                            mergedicts(&mut config, o, true);
                        }
                    }
                }
            }
            let components: Vec<String> = config
                .get("ext")
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("shell"))
                .and_then(|v| v.as_object())
                .and_then(|o| o.get("components"))
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_else(|| vec!["tmux".to_string(), "prompt".to_string()]);
            if components.iter().any(|c| c == &component) {
                0
            } else {
                1
            }
        }
        Some(StrFunction::Source) | Some(StrFunction::Setenv) | Some(StrFunction::Setup) => {
            // bindings/config.py:65, :99, :182
            // tmux actions need `ShellPowerline.update_renderer()` to
            // pull `theme_kwargs['colorscheme']`. That dispatches through
            // the not-yet-ported Powerline.__init__ chain.
            eprintln!(
                "powerline-config: action '{}' requires the Powerline orchestrator (deferred port)",
                action_name
            );
            1
        }
        None => {
            eprintln!("powerline-config: unknown function '{}'", action_name);
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_returns_2_when_no_args() {
        // sh:18  parser.parse_args() — Python argparse exits 2 on missing required
        assert_eq!(main(&[]), 2);
    }

    #[test]
    fn main_returns_2_for_unknown_function() {
        let r = main(&["definitely-not-a-real-function".to_string()]);
        assert_eq!(r, 2);
    }

    #[test]
    fn main_returns_1_for_tmux_function_requiring_powerline_orchestrator() {
        // 'source' / 'setenv' / 'setup' are TMUX_ACTIONS per
        // commands/config.py:21. The body needs `ShellPowerline` to
        // pull `theme_kwargs['colorscheme']`; until that ports we exit
        // 1 with a diagnostic rather than silently returning 0.
        let r = main(&["source".to_string()]);
        assert_eq!(r, 1);
    }

    #[test]
    fn main_handles_known_shell_function() {
        // 'command' is bindings/config.py:264 shell_command. It prints
        // `deduce_command()` and exits 0 on success, 1 on miss. We can
        // assert only that the dispatch path runs without panicking;
        // the exit code depends on `which powerline`.
        let r = main(&["command".to_string()]);
        assert!(r == 0 || r == 1, "unexpected exit code {}", r);
    }
}
