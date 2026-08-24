// vim:fileencoding=utf-8:noet
//! Port of `scripts/powerline-config`.
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

/// Build Colorscheme + Theme + TmuxRenderer + term_truecolor the
/// same way `powerline-daemon`'s `build_configs` does, then dispatch
/// `init_tmux_environment` + `source_tmux_files` via
/// `bindings::tmux::set_tmux_environment` / `source_tmux_file`.
///
/// Mirrors `tmux_setup()` from
/// `powerline/bindings/config.py:182-216` for the `args.source=True`
/// branch (the default — `args.source is None` and `tmux_version >=
/// (1, 9)` per py:204-206).
fn tmux_setup(args: &[String]) -> Result<(), String> {
    use crate::ported::bindings::config::{init_tmux_environment, sorted_tmux_configs};
    use crate::ported::bindings::tmux::{
        get_tmux_output, get_tmux_version, run_tmux_command, set_tmux_environment, source_tmux_file,
    };
    use crate::ported::colorscheme::Colorscheme;
    use crate::ported::config::TMUX_CONFIG_DIRECTORY;
    use crate::ported::lib::config::load_json_config;
    use crate::ported::lib::dict::mergedicts;
    use crate::ported::lib::encoding::get_preferred_output_encoding;
    use crate::ported::renderers::tmux::TmuxRenderer;
    use crate::ported::theme::Theme;
    use crate::ported::{_find_config_files, get_config_paths, get_default_theme};
    use std::path::PathBuf;

    // Build the same search-paths cascade the daemon uses:
    //   1. `POWERLINE_CONFIG_PATHS` env (colon-split)
    //   2. `--config-path` / `-p` flags (argparser sh:25 / py:get_argparser)
    //   3. `get_config_paths()` defaults (XDG + ~/.config)
    //   4. bundled `src/ported/config_files` (the in-tree path that
    //      ships in the published crate alongside the binary)
    // Mirror upstream `ShellPowerline.get_config_paths`
    // (shell.py:25-26): `args.config_path` (== --config-path /
    // POWERLINE_CONFIG_PATHS combined) REPLACES the default
    // cascade entirely when set; else use bundled + XDG.
    let mut explicit: Vec<PathBuf> = Vec::new();
    if let Ok(pcp) = std::env::var("POWERLINE_CONFIG_PATHS") {
        for p in pcp.split(':').filter(|s| !s.is_empty()) {
            explicit.push(PathBuf::from(p));
        }
    }
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--config-path" || args[i] == "-p" {
            if let Some(p) = args.get(i + 1) {
                explicit.push(PathBuf::from(p));
            }
            i += 2;
        } else {
            i += 1;
        }
    }
    let search_paths: Vec<PathBuf> = if !explicit.is_empty() {
        explicit
    } else {
        let mut paths: Vec<PathBuf> = Vec::new();
        // py:152  bundled `plugin_path` FIRST so user overrides win via
        // mergedicts in the load_cascade closure below.
        // Live checkout takes priority — dev iteration sees JSON edits
        // without a rebuild-and-extract cycle.
        if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
            let ported = PathBuf::from(manifest).join("src/ported/config_files");
            if ported.is_dir() {
                paths.push(ported);
            }
        }
        // Release-tarball fallback: include_str! tree extracted to
        // `$XDG_CACHE_HOME/powerliners/config_files/`. Without this
        // any non-`cargo install --path .` install (release tarball,
        // brew, bundled GHA binary) silently drops the bundled base
        // and `powerline-config tmux setup` emits broken markup.
        if let Some(cache) = crate::extensions::bundled_config::bundled_config_dir() {
            if !paths.contains(&cache) {
                paths.push(cache);
            }
        }
        paths.extend(get_config_paths());
        paths
    };

    // load_one(name) → first hit's load_json_config object.
    let load_one = |name: &str| -> Option<serde_json::Map<String, serde_json::Value>> {
        let matches = _find_config_files(&search_paths, name).ok()?;
        let p = matches.first()?;
        let v = load_json_config(p).ok()?;
        v.as_object().cloned()
    };
    let load_cascade = |levels: &[String]| -> Option<serde_json::Map<String, serde_json::Value>> {
        // py:191-200  load_config: iterate ALL matches per level and
        // mergedicts in order; later matches (user) override earlier
        // (bundled) per the search_paths layout above.
        let mut out: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        let mut loaded = 0u32;
        for level in levels {
            if let Ok(matches) = _find_config_files(&search_paths, level) {
                for p in &matches {
                    if let Ok(v) = load_json_config(p) {
                        if let Some(o) = v.as_object().cloned() {
                            mergedicts(&mut out, o, true);
                            loaded += 1;
                        }
                    }
                }
            }
        }
        if loaded == 0 {
            None
        } else {
            Some(out)
        }
    };

    let main = load_one("config").ok_or_else(|| "config.json not found".to_string())?;
    let colors_json = load_one("colors").ok_or_else(|| "colors.json not found".to_string())?;
    let cs_name = main
        .get("ext")
        .and_then(|e| e.get("tmux"))
        .and_then(|t| t.get("colorscheme"))
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let theme_name = main
        .get("ext")
        .and_then(|e| e.get("tmux"))
        .and_then(|t| t.get("theme"))
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();

    // py:165  colorscheme cascade
    let cs_levels = vec![
        "colorschemes/__main__".to_string(),
        "colorschemes/tmux/__main__".to_string(),
        format!("colorschemes/tmux/{}", cs_name),
    ];
    let colorscheme_json =
        load_cascade(&cs_levels).ok_or_else(|| "no colorscheme for tmux".to_string())?;

    // Same default_top_theme cascade the daemon uses (py:324-326).
    let user_top_theme = main
        .get("common")
        .and_then(|c| c.get("default_top_theme"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let computed_top_theme = {
        let enc = get_preferred_output_encoding().to_lowercase();
        get_default_theme(enc.starts_with("utf") || enc.starts_with("ucs"))
    };
    let top_theme: String = user_top_theme.unwrap_or_else(|| computed_top_theme.to_string());

    let theme_levels = vec![
        format!("themes/{}", top_theme),
        "themes/tmux/__main__".to_string(),
        format!("themes/tmux/{}", theme_name),
    ];
    let theme_json = load_cascade(&theme_levels).ok_or_else(|| "no theme for tmux".to_string())?;

    let colorscheme = Colorscheme::new(&colorscheme_json, &colors_json);
    let mut theme = Theme::new();
    // py:60-65  self.dividers = theme_config['dividers'] (deep-copied)
    if let Some(d) = theme_json.get("dividers").and_then(|v| v.as_object()) {
        theme.dividers = d.clone();
    }

    // py:167  common_config['term_truecolor'] — defaults to false.
    let term_truecolor = main
        .get("common")
        .and_then(|c| c.get("term_truecolor"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let renderer = TmuxRenderer::new(term_truecolor);

    // py:214  init_tmux_environment(pl, args, set_tmux_environment=ste)
    let env_vars = init_tmux_environment(&colorscheme, &theme, &renderer, term_truecolor);
    for (varname, value) in &env_vars {
        set_tmux_environment(varname, value, true);
    }

    // py:77-80  POWERLINE_COMMAND — hoisted above the source step.
    //
    // Deviation from upstream ordering (py:74-80 sources first, then
    // sets POWERLINE_COMMAND). From tmux 2.1 on, the status-left in
    // `powerline_tmux_2.1_plus.conf` is a double-quoted string
    // containing a bare `$POWERLINE_COMMAND`, which tmux expands from
    // the *server* environment at source time — not at `#()` run time.
    // Sourcing before the variable exists in that environment bakes an
    // empty command into status-left, so the left side runs
    // `env   tmux left …`, which is a tmux usage error rather than a
    // render. status-right escapes the expansion off
    // (`powerline-base.conf:4` is single-quoted) and is unaffected,
    // which is why the breakage is left-only and easy to miss.
    //
    // Upstream also gates on the *process* environment: if
    // POWERLINE_COMMAND is exported in the shell that runs
    // `powerline-config`, the tmux setenv is skipped entirely and the
    // server may never learn the value. We propagate the exported value
    // instead of skipping, so the tmux environment always agrees with
    // the shell's before anything expands it.
    let exported = std::env::var("POWERLINE_COMMAND").ok();
    if let Some(cmd) = resolve_powerline_command(exported, binding_config::deduce_command) {
        set_tmux_environment("POWERLINE_COMMAND", &cmd, false);
    }

    // Deviation from upstream: snapshot the user-tuned status lengths
    // before the source step so they can be re-applied after it.
    //
    // `powerline-base.conf:3,5` hardcode `status-left-length 20` /
    // `status-right-length 150`, and `tmux source` applies them
    // unconditionally. `~/.tmux.conf` runs `powerline-config tmux setup`
    // via `run-shell` and then tunes the lengths, so the config-load
    // ordering is fine — but any *later* `tmux setup` (a manual re-run,
    // a re-install check, a second binding) re-sources the same file
    // against the live server and silently reverts the tuning. tmux
    // trims `status-right` to `status-right-length` in
    // `status-format[0]` (`#{T;=/#{status-right-length}:status-right}`),
    // keeping the head of the string, so a 350 → 150 revert cuts the
    // right statusline's tail and shifts the surviving segments toward
    // the right edge.
    let preserved_lengths = preserved_status_lengths(|name| {
        // `show-options -gv` predates the oldest tmux we source conf
        // files for; on a server that rejects it the read fails, the
        // option is skipped, and behaviour falls back to upstream's.
        get_tmux_output(&(), &["show-options", "-gv", name]).map(|v| v.trim().to_string())
    });

    // py:215  source_tmux_files — version-matched conf files.
    // py:74  source_tmux_file(TMUX_CONFIG_DIRECTORY/powerline-base.conf)
    let base_conf = TMUX_CONFIG_DIRECTORY().join("powerline-base.conf");
    if base_conf.exists() {
        source_tmux_file(base_conf.to_str().unwrap_or(""));
    }
    // py:75-76  for fname, _ in sorted(get_tmux_configs(tmux_version), key=…): source
    if let Some(version) = get_tmux_version(&()) {
        for (fname, _priority) in sorted_tmux_configs(&version) {
            source_tmux_file(fname.to_str().unwrap_or(""));
        }
    }

    // Re-apply the snapshot over the values the conf files just wrote.
    for (name, value) in &preserved_lengths {
        run_tmux_command(&["set-option", "-g", name, value]);
    }

    // py:81-86  try run_tmux_command('refresh-client') — ignore failures
    let _ = std::process::Command::new("tmux")
        .arg("refresh-client")
        .status();

    Ok(())
}

/// tmux's own defaults for the status-length options that
/// `powerline-base.conf` overwrites, as reported by a
/// `tmux -f /dev/null` server (3.7b): `status-left-length 10`,
/// `status-right-length 40`.
const TMUX_STATUS_LENGTH_DEFAULTS: [(&str, &str); 2] =
    [("status-left-length", "10"), ("status-right-length", "40")];

/// Status-length options that must be re-applied after the tmux conf
/// files are sourced, as `(option, value)` pairs.
///
/// A length still sitting at tmux's built-in default is left alone, so
/// a fresh server keeps the wider budget `powerline-base.conf` wants.
/// Anything else is a deliberate setting and outranks the bundled
/// value. tmux reports defaults and explicit settings identically —
/// `show-options -g` on a `-f /dev/null` server prints
/// `status-right-length 40` — so "differs from tmux's default" is the
/// only signal available; pinning a length to tmux's own default is
/// therefore indistinguishable from not setting it, and still loses to
/// `powerline-base.conf`.
///
/// Split out from `tmux_setup` so the precedence is testable without a
/// live tmux server.
fn preserved_status_lengths<F>(read_option: F) -> Vec<(&'static str, String)>
where
    F: Fn(&str) -> Option<String>,
{
    TMUX_STATUS_LENGTH_DEFAULTS
        .iter()
        .filter_map(|(name, default)| match read_option(name) {
            Some(current) if current != *default => Some((*name, current)),
            _ => None,
        })
        .collect()
}

/// Which command name `tmux setup` should publish as
/// `POWERLINE_COMMAND` in the tmux server environment.
///
/// An exported, non-empty value wins; anything else falls through to
/// `deduce`. Split out from `tmux_setup` so the precedence is testable
/// without a live tmux server — see the note there for why this runs
/// before the config files are sourced.
fn resolve_powerline_command<F>(exported: Option<String>, deduce: F) -> Option<String>
where
    F: FnOnce() -> Option<String>,
{
    match exported {
        Some(v) if !v.is_empty() => Some(v),
        _ => deduce(),
    }
}

/// Port of the `if __name__ == '__main__':` block at
/// `scripts/powerline-config:16-22`.
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
    // sh:18  args = parser.parse_args()
    // The upstream argparser at `commands/config.py:33-58` defines two
    // subparsers — `tmux` and `shell` — each with sub-actions
    // (source/setenv/setup, command/uses). CLI form is
    //   powerline-config tmux setup [-p PATH] [-s]
    //   powerline-config shell command|uses [...]
    // Strip the leading `tmux` / `shell` prefix and treat the next
    // positional as the action name.
    // Non-port subcommand: `powerline-config vim source-path` prints
    // the extracted vim plugin path so users can drop one line in
    // `.vimrc`:
    //   execute 'source' trim(system('powerline-config vim source-path'))
    // Handled before the standard tmux/shell dispatch because `vim`
    // isn't a port-recognized prefix.
    if args.first().map(String::as_str) == Some("vim") {
        match args.get(1).map(String::as_str) {
            Some("source-path") => {
                match crate::extensions::bundled_config::bundled_vim_plugin_path() {
                    Some(p) => {
                        println!("{}", p.display());
                        return 0;
                    }
                    None => {
                        eprintln!("powerline-config vim source-path: extraction failed");
                        return 1;
                    }
                }
            }
            Some(s) => {
                eprintln!("powerline-config vim: unknown action '{}'", s);
                return 2;
            }
            None => {
                eprintln!("powerline-config vim: missing action argument (expected `source-path`)");
                return 2;
            }
        }
    }

    let action_name = match args.first().map(String::as_str) {
        Some("tmux") | Some("shell") => match args.get(1) {
            Some(s) => s.as_str(),
            None => {
                eprintln!("powerline-config: missing action argument");
                return 2;
            }
        },
        Some(s) => s,
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
        Some(StrFunction::Setup) => {
            // bindings/config.py:182-216
            //   def tmux_setup(pl, args):
            //       init_tmux_environment(pl, args, set_tmux_environment=ste)
            //       source_tmux_files(pl, args, tmux_version=..., source_tmux_file=stf)
            //
            // Builds Colorscheme + Theme + TmuxRenderer the same way the
            // daemon does, then loops set_tmux_environment for each var
            // and source_tmux_file for each version-matched conf.
            match tmux_setup(args) {
                Ok(()) => 0,
                Err(e) => {
                    eprintln!("powerline-config: tmux setup failed: {}", e);
                    1
                }
            }
        }
        Some(StrFunction::Source) | Some(StrFunction::Setenv) => {
            // bindings/config.py:65, :99
            // source/setenv variants of the orchestrator: not yet wired
            // (only tmux_setup is needed for the daemon-swap path).
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

    /// Upstream skips the tmux setenv entirely when POWERLINE_COMMAND is
    /// exported in the calling shell, which leaves the tmux server
    /// without the value that `powerline_tmux_2.1_plus.conf` expands
    /// into status-left. We propagate the exported value instead.
    #[test]
    fn exported_command_is_propagated_not_skipped() {
        let got = resolve_powerline_command(Some("zpowerline".to_string()), || {
            panic!("deduce must not run when the value is already exported")
        });
        assert_eq!(got.as_deref(), Some("zpowerline"));
    }

    /// An exported-but-empty value is the shape that produced
    /// `env   tmux left …`; it must not be published as the command.
    #[test]
    fn empty_exported_command_falls_through_to_deduce() {
        let got = resolve_powerline_command(Some(String::new()), || Some("powerline".to_string()));
        assert_eq!(got.as_deref(), Some("powerline"));
    }

    /// The reported breakage: a live server carrying a tuned
    /// `status-right-length` must come back out of `tmux setup` with
    /// that value, not `powerline-base.conf`'s 150. tmux keeps only the
    /// head of an over-long `status-right`, so the reverted 150 clipped
    /// the tail segments and pushed the rest to the right edge.
    #[test]
    fn tuned_status_lengths_survive_the_source_step() {
        let got = preserved_status_lengths(|name| {
            Some(
                match name {
                    "status-right-length" => "350",
                    "status-left-length" => "10",
                    other => panic!("unexpected option read: {}", other),
                }
                .to_string(),
            )
        });
        assert_eq!(got, vec![("status-right-length", "350".to_string())]);
    }

    /// A fresh server reports tmux's defaults, and those must NOT be
    /// preserved — `powerline-base.conf`'s wider budget is the point of
    /// sourcing it.
    #[test]
    fn default_status_lengths_are_not_preserved() {
        let got = preserved_status_lengths(|name| {
            Some(
                match name {
                    "status-left-length" => "10",
                    "status-right-length" => "40",
                    other => panic!("unexpected option read: {}", other),
                }
                .to_string(),
            )
        });
        assert!(got.is_empty(), "fresh-server defaults leaked: {:?}", got);
    }

    /// A server too old for `show-options -gv` (or no server at all)
    /// reads as `None`; setup must fall back to upstream behaviour
    /// instead of writing an empty option value.
    #[test]
    fn unreadable_options_are_skipped() {
        assert!(preserved_status_lengths(|_| None).is_empty());
    }

    #[test]
    fn unset_command_falls_through_to_deduce() {
        let got = resolve_powerline_command(None, || Some("powerline".to_string()));
        assert_eq!(got.as_deref(), Some("powerline"));

        // Nothing exported and nothing deducible means nothing to
        // publish — the caller must not set an empty tmux variable.
        assert_eq!(resolve_powerline_command(None, || None), None);
    }
}
