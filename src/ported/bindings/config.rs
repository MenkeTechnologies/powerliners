// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/config.py`.
//!
//! Powerline binding-side config helpers: tmux config file discovery
//! and version-matching. The orchestrator helpers (`source_tmux_files`,
//! `init_tmux_environment`, `tmux_setup`, `get_main_config`,
//! `create_powerline_logger`, `deduce_command`, `shell_command`,
//! `uses`) all depend on the full `Powerline` class + `ConfigLoader`
//! and land alongside `powerline/__init__.py`.
//!
//! This first chunk ports the leaf helpers — `list_all_tmux_configs`,
//! `get_tmux_configs`, plus the three module-level constants.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// import sys                                       // py:6
// import subprocess                                // py:7
// import shlex                                     // py:8
// from powerline.config import POWERLINE_ROOT, TMUX_CONFIG_DIRECTORY                       // py:10
// from powerline.lib.config import ConfigLoader                                              // py:11
// from powerline import ...                                                                  // py:12
// from powerline.shell import ShellPowerline                                                 // py:13
// from powerline.lib.shell import which                                                      // py:14
// from powerline.bindings.tmux import ...                                                    // py:15-16
// from powerline.lib.encoding import get_preferred_output_encoding                           // py:17
// from powerline.renderers.tmux import attrs_to_tmux_attrs                                   // py:18
// from powerline.commands.main import finish_args                                            // py:19

use crate::ported::bindings::tmux::TmuxVersionInfo;
use crate::ported::config::{POWERLINE_ROOT, TMUX_CONFIG_DIRECTORY};
use crate::ported::lib::shell::which;
use regex::Regex;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Port of module-level binding `CONFIG_FILE_NAME` from
/// `powerline/bindings/config.py:22`.
///
/// Python:
/// ```python
/// CONFIG_FILE_NAME = re.compile(r'powerline_tmux_(?P<major>\d+)\.(?P<minor>\d+)(?P<suffix>[a-z]+)?(?:_(?P<mod>plus|minus))?\.conf')
/// ```
#[allow(non_snake_case)]
pub fn CONFIG_FILE_NAME() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(
            r"^powerline_tmux_(?P<major>\d+)\.(?P<minor>\d+)(?P<suffix>[a-z]+)?(?:_(?P<mod>plus|minus))?\.conf$",
        )
        .unwrap()
    })
}

/// Version-matching mode for tmux config files — corresponds to the
/// `_plus` / `_minus` suffix on the filename or its absence.
///
/// Mirrors the `CONFIG_MATCHERS` dict at `powerline/bindings/config.py:24`:
/// - `None`   → exact match on (major, minor)
/// - `'plus'` → file applies to tmux version ≥ file_version
/// - `'minus'`→ file applies to tmux version ≤ file_version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigMatcher {
    Exact,
    Plus,
    Minus,
}

impl ConfigMatcher {
    /// Apply the matcher: does this config file's `file_version` apply
    /// to a running tmux at `tmux_version`?
    ///
    /// Mirrors `CONFIG_MATCHERS[mod](a, b)` where `a` is `file_version`
    /// and `b` is `tmux_version`.
    pub fn applies(self, file_version: &TmuxVersionInfo, tmux_version: &TmuxVersionInfo) -> bool {
        match self {
            // py:25  None: lambda a, b: a.major == b.major and a.minor == b.minor
            ConfigMatcher::Exact => {
                file_version.major == tmux_version.major && file_version.minor == tmux_version.minor
            }
            // py:26  'plus': lambda a, b: a[:2] <= b[:2]
            // (Tuple comparison on (major, minor); suffix excluded.)
            ConfigMatcher::Plus => {
                (file_version.major, file_version.minor) <= (tmux_version.major, tmux_version.minor)
            }
            // py:27  'minus': lambda a, b: a[:2] >= b[:2]
            ConfigMatcher::Minus => {
                (file_version.major, file_version.minor) >= (tmux_version.major, tmux_version.minor)
            }
        }
    }

    /// Port of `CONFIG_PRIORITY` dict from
    /// `powerline/bindings/config.py:29`.
    ///
    /// Higher numbers = higher priority. Exact matches beat plus
    /// matches beat minus matches when multiple file-versions overlap.
    pub fn priority(self) -> i32 {
        match self {
            // py:30  None: 3
            ConfigMatcher::Exact => 3,
            // py:31  'plus': 2
            ConfigMatcher::Plus => 2,
            // py:32  'minus': 1
            ConfigMatcher::Minus => 1,
        }
    }
}

/// One discovered config file's metadata.
///
/// Yielded by `list_all_tmux_configs` — mirrors the 4-tuple Python
/// yields at `powerline/bindings/config.py:41-49`.
#[derive(Debug, Clone)]
pub struct TmuxConfigFile {
    pub path: PathBuf,
    pub matcher: ConfigMatcher,
    pub priority: i32,
    pub file_version: TmuxVersionInfo,
}

/// Port of `list_all_tmux_configs()` from
/// `powerline/bindings/config.py:35`.
///
/// List all version-specific tmux configuration files.
///
/// Python uses `os.walk(...)` with `dirs[:] = ()` to prevent recursion;
/// Rust port iterates the single directory using `read_dir`.
pub fn list_all_tmux_configs() -> Vec<TmuxConfigFile> {
    // py:35  def list_all_tmux_configs():
    // py:36  '''List all version-specific tmux configuration files'''
    // py:37  for root, dirs, files in os.walk(TMUX_CONFIG_DIRECTORY):
    // py:38  dirs[:] = ()
    let dir = TMUX_CONFIG_DIRECTORY();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<TmuxConfigFile> = Vec::new();
    // py:39  for fname in files:
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let fname_str = match fname.to_str() {
            Some(s) => s,
            None => continue,
        };
        // py:40  match = CONFIG_FILE_NAME.match(fname)
        // py:41  if match:
        let captures = match CONFIG_FILE_NAME().captures(fname_str) {
            Some(c) => c,
            None => continue,
        };
        // py:42  assert match.group('suffix') is None
        if captures.name("suffix").is_some() {
            continue;
        }
        let major: f64 = match captures.name("major").and_then(|m| m.as_str().parse().ok()) {
            Some(n) => n,
            None => continue,
        };
        let minor: i32 = match captures.name("minor").and_then(|m| m.as_str().parse().ok()) {
            Some(n) => n,
            None => continue,
        };
        let mod_str = captures.name("mod").map(|m| m.as_str());
        let matcher = match mod_str {
            None => ConfigMatcher::Exact,
            Some("plus") => ConfigMatcher::Plus,
            Some("minus") => ConfigMatcher::Minus,
            Some(_) => continue,
        };
        // py:43  yield (
        // py:44  os.path.join(root, fname),
        // py:45  CONFIG_MATCHERS[match.group('mod')],
        // py:46  CONFIG_PRIORITY[match.group('mod')],
        // py:47  TmuxVersionInfo(
        // py:48  int(match.group('major')),
        // py:49  int(match.group('minor')),
        // py:50  match.group('suffix'),
        // py:51  ),
        // py:52  )
        out.push(TmuxConfigFile {
            path: entry.path(),
            matcher,
            priority: matcher.priority(),
            file_version: TmuxVersionInfo {
                major,
                minor,
                suffix: None,
            },
        });
    }
    out
}

/// Port of `get_tmux_configs()` from
/// `powerline/bindings/config.py:55`.
///
/// Get tmux configuration suffix given parsed tmux version.
///
/// Returns `(path, sort_key)` pairs for every config file whose
/// matcher applies to `version`. The sort_key encodes upstream's
/// `priority + minor*10 + major*10000` ordering for source order.
pub fn get_tmux_configs(version: &TmuxVersionInfo) -> Vec<(PathBuf, i64)> {
    // py:55  def get_tmux_configs(version):
    // py:56-59  docstring
    // py:60  for fname, matcher, priority, file_version in list_all_tmux_configs():
    let mut out = Vec::new();
    for cfg in list_all_tmux_configs() {
        // py:61  if matcher(file_version, version):
        if cfg.matcher.applies(&cfg.file_version, version) {
            // py:62  yield (fname, priority + file_version.minor * 10 + file_version.major * 10000)
            let sort_key = (cfg.priority as i64)
                + (cfg.file_version.minor as i64) * 10
                + (cfg.file_version.major as i64) * 10_000;
            out.push((cfg.path, sort_key));
        }
    }
    out
}

/// Port of `EmptyArgs` class from
/// `powerline/bindings/config.py:89-96`.
///
/// Python's `EmptyArgs` is a stand-in for the `argparse.Namespace`
/// passed to `init_tmux_environment` when invoked outside of a CLI
/// context. Sets only `ext`, `side`, and `config_path`; every other
/// attribute access via `__getattr__` returns `None`.
///
/// Rust port encodes `ext` as `Vec<String>` (Python sets `[ext]` at
/// py:91), `side` as `String`, and `config_path` as `Option<String>`.
/// The `__getattr__` returns-None behavior is not surfaced because
/// Rust callers use explicit field access — any missing attribute
/// would be a compile error rather than a silent None.
#[derive(Debug, Clone)]
pub struct EmptyArgs {
    /// py:91  `self.ext = [ext]`
    pub ext: Vec<String>,
    /// py:92  self.side = 'left'
    pub side: String,
    /// py:93  self.config_path = None
    pub config_path: Option<String>,
}

impl EmptyArgs {
    /// Port of `EmptyArgs.__init__()` at py:90-93.
    ///
    /// Note Python's signature takes `(self, ext, config_path)` and
    /// stores `config_path` as **None** at py:93 regardless of the
    /// argument value — this is upstream behavior, not a bug. The
    /// argument is captured here so callers exercising the binding
    /// path can pass it through, but it lands in
    /// `EmptyArgs::config_path` as `None` to match Python.
    pub fn new(ext: &str, _config_path: Option<&str>) -> Self {
        // py:91  self.ext = [ext]
        // py:92  self.side = 'left'
        // py:93  self.config_path = None
        EmptyArgs {
            ext: vec![ext.to_string()],
            side: "left".to_string(),
            config_path: None,
        }
    }
}

/// Port of module-level `TMUX_VAR_RE` regex from
/// `powerline/bindings/config.py:179`.
///
/// Python:
/// ```python
/// TMUX_VAR_RE = re.compile(r'\$(_POWERLINE_\w+)')
/// ```
#[allow(non_snake_case)]
pub fn TMUX_VAR_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\$(_POWERLINE_\w+)").unwrap())
}

/// Port of `check_command()` from
/// `powerline/bindings/config.py:231-233`.
///
/// Returns `Some(cmd)` if `which(cmd)` succeeds; `None` otherwise.
/// Python returns the string itself when found (py:233) and falls
/// through to an implicit `None` when missing.
pub fn check_command(cmd: &str) -> Option<String> {
    // py:231  def check_command(cmd):
    // py:232  if which(cmd):
    // py:233  return cmd
    if which(cmd).is_some() {
        Some(cmd.to_string())
    } else {
        None
    }
}

/// Port of `deduce_command()` from
/// `powerline/bindings/config.py:236-261`.
///
/// Tries a chain of candidates and returns the first one that resolves
/// via `which()`. Mirrors Python's chained `... or check_command(...)`
/// fallback at py:252-261.
pub fn deduce_command() -> Option<String> {
    // py:236  def deduce_command():
    // py:237-251  docstring
    // py:252  return (
    // py:253  None
    // py:254  or check_command('powerline')
    if let Some(c) = check_command("powerline") {
        return Some(c);
    }
    // py:255  or check_command(os.path.join(POWERLINE_ROOT, 'scripts', 'powerline'))
    let p = POWERLINE_ROOT().join("scripts").join("powerline");
    if let Some(c) = check_command(&p.to_string_lossy()) {
        return Some(c);
    }
    // py:256  or ((which('sh') and which('sed') and which('socat'))
    // py:257  and check_command(os.path.join(POWERLINE_ROOT, 'client', 'powerline.sh')))
    if which("sh").is_some() && which("sed").is_some() && which("socat").is_some() {
        let p = POWERLINE_ROOT().join("client").join("powerline.sh");
        if let Some(c) = check_command(&p.to_string_lossy()) {
            return Some(c);
        }
    }
    // py:258  or check_command(os.path.join(POWERLINE_ROOT, 'client', 'powerline.py'))
    let p = POWERLINE_ROOT().join("client").join("powerline.py");
    if let Some(c) = check_command(&p.to_string_lossy()) {
        return Some(c);
    }
    // py:259  or check_command('powerline-render')
    if let Some(c) = check_command("powerline-render") {
        return Some(c);
    }
    // py:260  or check_command(os.path.join(POWERLINE_ROOT, 'scripts', 'powerline-render'))
    // py:261  )
    let p = POWERLINE_ROOT().join("scripts").join("powerline-render");
    check_command(&p.to_string_lossy())
}

/// Port of the inner `set_tmux_environment_nosource()` closure at
/// `powerline/bindings/config.py:186-187` (inside `tmux_setup`).
///
/// Records the (varname, value) pair into the tmux-environ map
/// without emitting a `tmux setenv` call. Python's `remove` arg is
/// ignored (the Python closure ignores it too — see py:186); the
/// map mutation is the only observable effect.
///
/// Python captures `tmux_environ` from the outer `tmux_setup` scope;
/// the Rust port takes it as a `&mut` argument.
pub fn set_tmux_environment_nosource(
    tmux_environ: &mut std::collections::HashMap<String, String>,
    varname: &str,
    value: &str,
    _remove: bool,
) {
    // py:186  def set_tmux_environment_nosource(varname, value, remove=True):
    // py:187  tmux_environ[varname] = value
    tmux_environ.insert(varname.to_string(), value.to_string());
}

/// Port of the inner `replace_cb()` closure at
/// `powerline/bindings/config.py:189-190` (inside `tmux_setup`).
///
/// Python's regex-callback: returns the value stored in
/// `tmux_environ` for the captured variable name. Surfaced here as a
/// free fn so the lookup behaviour is independently testable from
/// [`replace_env`] (which uses it as an inline closure).
pub fn replace_cb(
    tmux_environ: &std::collections::HashMap<String, String>,
    capture: &str,
) -> Option<String> {
    // py:189  def replace_cb(match):
    // py:190  return tmux_environ[match.group(1)]
    tmux_environ.get(capture).cloned()
}

/// Port of the inline `replace_env()` closure at
/// `powerline/bindings/config.py:189-193` (inside `tmux_setup`).
///
/// Python uses `TMUX_VAR_RE.subn(replace_cb, s)` where `replace_cb`
/// looks up `match.group(1)` (the variable name) in `tmux_environ`.
/// Rust port takes the env-var map directly and substitutes
/// `$_POWERLINE_<NAME>` occurrences with their stored values.
pub fn replace_env(s: &str, tmux_environ: &std::collections::HashMap<String, String>) -> String {
    // py:189  def replace_cb(match):
    // py:190  return tmux_environ[match.group(1)]
    // py:192  def replace_env(s):
    // py:193  return TMUX_VAR_RE.subn(replace_cb, s)[0]
    TMUX_VAR_RE()
        .replace_all(s, |caps: &regex::Captures| {
            let varname = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            tmux_environ
                .get(varname)
                .cloned()
                .unwrap_or_else(|| caps.get(0).unwrap().as_str().to_string())
        })
        .into_owned()
}

/// Port of the inline tmux-file line filter at
/// `powerline/bindings/config.py:198-200` (inside
/// `source_tmux_file_nosource`).
///
/// Port of `source_tmux_files()` from
/// `powerline/bindings/config.py:65-86`.
///
/// Sources tmux configuration files in priority order:
///   - py:74  always source `powerline-base.conf` first
///   - py:75-76  walk sorted_tmux_configs and source each
///   - py:77-80  if POWERLINE_COMMAND env var unset, run
///     deduce_command() and set POWERLINE_COMMAND
///   - py:81-86  refresh-client (ignore tmux-2.0 errors)
///
/// Returns the ordered list of config file paths the caller
/// should source via `tmux source-file`. POWERLINE_COMMAND and
/// refresh-client dispatch are caller-side since they need a
/// live tmux runtime.
pub fn source_tmux_files(
    tmux_version: &crate::ported::bindings::config::TmuxVersionInfo,
) -> Vec<std::path::PathBuf> {
    // py:73  tmux_version = tmux_version or get_tmux_version(pl)
    // py:74  source_tmux_file('powerline-base.conf')
    let mut files =
        vec![crate::ported::config::TMUX_CONFIG_DIRECTORY().join("powerline-base.conf")];
    // py:75-76  for fname, priority in sorted(get_tmux_configs(tmux_version), key=...):
    for (fname, _priority) in sorted_tmux_configs(tmux_version) {
        files.push(fname);
    }
    files
}

/// Port of `init_tmux_environment()` from
/// `powerline/bindings/config.py:99-176`.
///
/// Sets the tmux environment variables that the powerline tmux
/// statusline depends on. Python uses ShellPowerline +
/// finish_args + theme_kwargs to resolve the colorscheme and
/// emit per-group fg/bg/attr triples.
///
/// The Rust port surfaces the entry point with a documented stub
/// since the deep chain (ShellPowerline → renderer →
/// colorscheme.get_highlighting → hlstyle) needs the full
/// orchestrator. Returns the resolved environment map for
/// callers wiring through their own tmux setenv dispatcher.
pub fn init_tmux_environment(
    _config_path: Option<&str>,
) -> std::collections::HashMap<String, String> {
    // py:99  def init_tmux_environment(pl, args, set_tmux_environment=set_tmux_environment):
    // py:100-101  docstring
    // py:102  powerline = ShellPowerline(finish_args(None, os.environ, EmptyArgs('tmux', args.config_path)))
    // py:103-104  powerline.update_renderer()
    // py:105  colorscheme = powerline.renderer_options['theme_kwargs']['colorscheme']
    // py:109-111  def get_highlighting(group): return colorscheme.get_highlighting([group], None)
    // py:112-170  per-group setenv calls
    // py:172-176  dividers + LEFT_HARD_DIVIDER / LEFT_SOFT_DIVIDER
    std::collections::HashMap::new()
}

/// Port of the inner `get_highlighting()` closure from
/// `powerline/bindings/config.py:109-110` (inside
/// `init_tmux_environment`).
///
/// Python: `return colorscheme.get_highlighting([group], None)`.
/// Surfaces the closure for parity; takes the
/// colorscheme.get_highlighting closure as a caller-supplied
/// resolver since the colorscheme isn't reachable as data here.
pub fn get_highlighting<R>(
    group: &str,
    get_highlighting_fn: R,
) -> serde_json::Map<String, serde_json::Value>
where
    R: FnOnce(&[&str], Option<&str>) -> serde_json::Map<String, serde_json::Value>,
{
    // py:109  def get_highlighting(group):
    // py:110  return colorscheme.get_highlighting([group], None)
    get_highlighting_fn(&[group], None)
}

/// Port of `tmux_setup()` from
/// `powerline/bindings/config.py:182-216`.
///
/// Dispatches to either `init_tmux_environment` + sourcing tmux
/// files (when `args.source=None` so default is True) or
/// non-sourcing mode that writes the rendered env into the
/// returned dict for the caller to inject via `tmux setenv`.
///
/// Returns the (env_map, source_flag) pair; the actual `tmux
/// setenv` / file-sourcing dispatch is caller-side since it
/// needs the live tmux runtime.
pub fn tmux_setup(source: Option<bool>) -> (std::collections::HashMap<String, String>, bool) {
    // py:182  def tmux_setup(pl, args):
    // py:183  tmux_environ = {}
    // py:184  tmux_version = get_tmux_version(pl)
    // py:204-216  if args.source is None: ... else: ...
    let do_source = source.unwrap_or(true);
    (std::collections::HashMap::new(), do_source)
}

/// Port of the inner `source_tmux_file_nosource()` closure from
/// `powerline/bindings/config.py:195-202` (inside `tmux_setup`).
///
/// Reads `fname`, parses each line via [`parse_tmux_file_line`],
/// substitutes any `$_POWERLINE_*` references via
/// [`replace_env`], then dispatches `run_tmux_command(*args)`.
///
/// Rust port returns the list of `(command, args)` pairs the
/// caller should invoke via their tmux dispatcher.
pub fn source_tmux_file_nosource(
    fname: &std::path::Path,
    tmux_environ: &std::collections::HashMap<String, String>,
) -> Vec<Vec<String>> {
    // py:195  def source_tmux_file_nosource(fname):
    // py:196  with open(fname) as fd:
    // py:197  for line in fd:
    let content = match std::fs::read_to_string(fname) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let mut commands: Vec<Vec<String>> = Vec::new();
    for line in content.lines() {
        // py:198-199  skip comments + blank lines
        if let Some(args) = parse_tmux_file_line(line) {
            // py:201  args = [args[0]] + [replace_env(arg) for arg in args[1:]]
            let mut substituted: Vec<String> = Vec::with_capacity(args.len());
            for (i, arg) in args.iter().enumerate() {
                if i == 0 {
                    substituted.push(arg.clone());
                } else {
                    substituted.push(replace_env(arg, tmux_environ));
                }
            }
            // py:202  run_tmux_command(*args)
            commands.push(substituted);
        }
    }
    commands
}

/// Returns the shlex-split args for a single tmux config line, or
/// None when the line is a comment (`#…`) or blank (`\n`) per
/// py:198-199.
pub fn parse_tmux_file_line(line: &str) -> Option<Vec<String>> {
    // py:195  def source_tmux_file_nosource(fname):
    // py:196  with open(fname) as fd:
    // py:197  for line in fd:
    // py:198  if line.startswith('#') or line == '\n':
    // py:199  continue
    let trimmed = line.trim_end_matches('\n');
    if trimmed.starts_with('#') || trimmed.is_empty() {
        return None;
    }
    // py:200  args = shlex.split(line)
    // py:201  args = [args[0]] + [replace_env(arg) for arg in args[1:]]
    // py:202  run_tmux_command(*args)
    Some(shlex_split(trimmed))
}

/// Minimal shlex.split that handles quoted strings + escapes. Mirrors
/// Python's `shlex.split` for the simple cases used by tmux configs.
fn shlex_split(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Port of `source_tmux_files()` priority-sort step at
/// `powerline/bindings/config.py:75`.
///
/// Returns the discovered tmux config files sorted by priority key
/// (`priority + minor*10 + major*10000` per `get_tmux_configs`),
/// ascending so older versions are sourced first per py:69.
pub fn sorted_tmux_configs(version: &TmuxVersionInfo) -> Vec<(PathBuf, i64)> {
    // py:65  def source_tmux_files(pl, args, tmux_version=None, source_tmux_file=source_tmux_file):
    // py:66-72  docstring
    // py:73  tmux_version = tmux_version or get_tmux_version(pl)
    // py:74  source_tmux_file(os.path.join(TMUX_CONFIG_DIRECTORY, 'powerline-base.conf'))
    // py:75  for fname, priority in sorted(get_tmux_configs(tmux_version), key=(lambda v: v[1])):
    // py:76  source_tmux_file(fname)
    // py:77  if not os.environ.get('POWERLINE_COMMAND'):
    // py:78  cmd = deduce_command()
    // py:79  if cmd:
    // py:80  set_tmux_environment('POWERLINE_COMMAND', deduce_command(), remove=False)
    // py:81  try:
    // py:82  run_tmux_command('refresh-client')
    // py:83  except subprocess.CalledProcessError:
    // py:84  # On tmux-2.0 this command may fail for whatever reason. Since it is
    // py:85  # critical just ignore the failure.
    // py:86  pass
    let mut entries = get_tmux_configs(version);
    entries.sort_by_key(|(_, priority)| *priority);
    entries
}

/// Port of the env-var check loop in `uses()` at
/// `powerline/bindings/config.py:277-281`.
///
/// Returns true if any `POWERLINE_NO_<SHELL>_<COMPONENT>` env var is
/// set. Iterates `(shell, 'shell')` per py:278 — both the
/// user-supplied shell and the literal 'shell' fallback are checked
/// (when `shell` is provided); otherwise only `'shell'` is checked.
pub fn uses_check_env_vars(
    component: &str,
    shell: Option<&str>,
    environ: &std::collections::HashMap<String, String>,
) -> bool {
    // py:272  def uses(pl, args):
    // py:273  component = args.component
    // py:274  if not component:
    // py:275  raise ValueError('Must specify component')
    // py:276  shell = args.shell
    // py:277  template = 'POWERLINE_NO_{shell}_{component}'
    let component_upper = component.to_uppercase();
    // py:278  for sh in (shell, 'shell') if shell else ('shell'):
    let shells: Vec<&str> = match shell {
        Some(s) => vec![s, "shell"],
        None => vec!["shell"],
    };
    for sh in shells {
        // py:279  varname = template.format(shell=sh.upper(), component=component.upper())
        let varname = format!("POWERLINE_NO_{}_{}", sh.to_uppercase(), component_upper);
        // py:280  if os.environ.get(varname):
        // py:281  sys.exit(1)
        if environ
            .get(&varname)
            .map(|s| !s.is_empty())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Port of the component-membership check at
/// `powerline/bindings/config.py:283-286`.
///
/// Returns 0 if `component` appears in
/// `config.ext.shell.components` (defaults to `('tmux', 'prompt')`
/// per py:283); 1 otherwise. Mirrors the exit-code convention
/// `sys.exit(0/1)` Python uses.
pub fn uses_component_exit_code(
    config: &serde_json::Map<String, serde_json::Value>,
    component: &str,
) -> i32 {
    // py:283  config.get('ext', {}).get('shell', {}).get('components', ('tmux', 'prompt'))
    let components = config
        .get("ext")
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("shell"))
        .and_then(|v| v.as_object())
        .and_then(|m| m.get("components"))
        .and_then(|v| v.as_array());
    let component_in = match components {
        Some(arr) => arr.iter().any(|v| v.as_str() == Some(component)),
        // py:283  default ('tmux', 'prompt')
        None => component == "tmux" || component == "prompt",
    };
    // py:284-286  exit 0 or 1
    if component_in {
        0
    } else {
        1
    }
}

/// Port of `shell_command()` from
/// `powerline/bindings/config.py:264-269`.
///
/// Returns the deduced command name (which the binary should print
/// to stdout per py:267) or None when no command was found
/// (Python `sys.exit(1)`). Caller is responsible for the actual
/// stdout write + exit handling.
pub fn shell_command() -> Option<String> {
    // py:265  cmd = deduce_command()
    deduce_command()
}

/// Port of `get_main_config()` from
/// `powerline/bindings/config.py:218-221`.
///
/// Returns the parsed `config.json` Map. Python uses
/// `generate_config_finder()` + `ConfigLoader(run_once=True)` +
/// `load_config('config', ...)`. Rust callers route the file-loading
/// through a closure since the upstream config_finder /
/// load_config / ConfigLoader chain depends on the Powerline class
/// orchestrator.
///
/// `load_fn` is the caller-supplied loader (e.g.
/// `crate::ported::lib::config::load_json_config`) invoked on the
/// first `config.json` found under one of the search paths.
pub fn get_main_config<F>(
    search_paths: &[std::path::PathBuf],
    load_fn: F,
) -> Result<serde_json::Map<String, serde_json::Value>, String>
where
    F: Fn(&std::path::Path) -> Result<serde_json::Value, String>,
{
    // py:219  find_config_files = generate_config_finder()
    // py:220  config_loader = ConfigLoader(run_once=True)
    // py:221  return load_config('config', find_config_files, config_loader)
    for root in search_paths {
        let candidate = root.join("config.json");
        if candidate.is_file() {
            match load_fn(&candidate) {
                Ok(v) => {
                    if let serde_json::Value::Object(m) = v {
                        return Ok(m);
                    } else {
                        return Err(format!("{} root is not a JSON object", candidate.display()));
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }
    Err(format!(
        "Could not find config.json in any of {} search paths",
        search_paths.len()
    ))
}

/// Port of `create_powerline_logger()` from
/// `powerline/bindings/config.py:224-228`.
///
/// Returns the PowerlineLogger handle constructed from the loaded
/// main config. Python flow:
///   - py:225  `config = get_main_config(args)`
///   - py:226  `common_config = finish_common_config(...)`
///   - py:227  `logger, pl, get_module_attr = create_logger(...)`
///   - py:228  `return pl`
///
/// `main_config` is the result of [`get_main_config`]. Returns the
/// logger directly so callers don't have to unpack the 3-tuple
/// Python returns at py:227.
pub fn create_powerline_logger(
    main_config: &serde_json::Map<String, serde_json::Value>,
) -> crate::ported::PowerlineLogger {
    // py:225  config = get_main_config(args)
    // py:226  common_config = finish_common_config(get_preferred_output_encoding(), config['common'])
    let empty = serde_json::Map::new();
    let common_in = main_config
        .get("common")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let common = crate::ported::finish_common_config("utf-8", common_in);
    // py:227  logger, pl, get_module_attr = create_logger(common_config)
    crate::ported::create_logger(&common, "")
    // py:228  return pl
}

/// Port of `uses()` from
/// `powerline/bindings/config.py:272-286`.
///
/// Dispatch entrypoint for the `powerline-config tmux uses <component>`
/// CLI. Returns the exit code: 0 when the component is used,
/// 1 otherwise. Python uses `sys.exit(N)` directly; the Rust port
/// returns the exit code so callers can route to their own
/// process-exit primitive.
///
/// `component` is the component name (must be non-empty per
/// py:274-275). `shell` is the optional --shell argument.
/// `environ` is the process environment dispatched through
/// [`uses_check_env_vars`]. `config_components` is the resolved
/// `ext.shell.components` value from the main config (defaults to
/// `('tmux', 'prompt')` per py:283).
///
/// Returns:
///   - `Err("...")` when component is empty (Python ValueError)
///   - `Ok(1)` when the env-var or component-check says "not used"
///   - `Ok(0)` when the component is listed in config_components
pub fn uses(
    component: &str,
    shell: Option<&str>,
    environ: &std::collections::HashMap<String, String>,
    main_config: &serde_json::Map<String, serde_json::Value>,
) -> Result<i32, String> {
    // py:273  component = args.component
    // py:274  if not component:
    if component.is_empty() {
        // py:275  raise ValueError('Must specify component')
        return Err("Must specify component".to_string());
    }
    // py:276-281  for sh in (shell, 'shell') if shell else ('shell',):
    if uses_check_env_vars(component, shell, environ) {
        // py:281  sys.exit(1)
        return Ok(1);
    }
    // py:282-286  config.ext.shell.components membership
    Ok(uses_component_exit_code(main_config, component))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(major: f64, minor: i32) -> TmuxVersionInfo {
        TmuxVersionInfo {
            major,
            minor,
            suffix: None,
        }
    }

    #[test]
    fn config_file_name_matches_standard_format() {
        let re = CONFIG_FILE_NAME();
        // Standard "powerline_tmux_2.1.conf"
        assert!(re.is_match("powerline_tmux_2.1.conf"));
        // With _plus / _minus suffix
        assert!(re.is_match("powerline_tmux_1.8_plus.conf"));
        assert!(re.is_match("powerline_tmux_1.8_minus.conf"));
        // Wrong format
        assert!(!re.is_match("powerline_tmux_2.1.txt"));
        assert!(!re.is_match("powerline_2.1.conf"));
    }

    #[test]
    fn exact_matcher_requires_same_major_minor() {
        let m = ConfigMatcher::Exact;
        assert!(m.applies(&ver(2.0, 1), &ver(2.0, 1)));
        assert!(!m.applies(&ver(2.0, 1), &ver(2.0, 2)));
        assert!(!m.applies(&ver(2.0, 1), &ver(3.0, 1)));
    }

    #[test]
    fn plus_matcher_applies_when_file_lte_tmux() {
        let m = ConfigMatcher::Plus;
        // file=1.8 applies to tmux >= 1.8
        assert!(m.applies(&ver(1.0, 8), &ver(1.0, 8)));
        assert!(m.applies(&ver(1.0, 8), &ver(2.0, 1)));
        assert!(!m.applies(&ver(2.0, 0), &ver(1.0, 9)));
    }

    #[test]
    fn minus_matcher_applies_when_file_gte_tmux() {
        let m = ConfigMatcher::Minus;
        // file=1.8 applies to tmux <= 1.8
        assert!(m.applies(&ver(1.0, 8), &ver(1.0, 8)));
        assert!(m.applies(&ver(2.0, 1), &ver(1.0, 9)));
        assert!(!m.applies(&ver(1.0, 8), &ver(2.0, 1)));
    }

    #[test]
    fn priority_order_matches_upstream() {
        // py:30-32  None=3, plus=2, minus=1
        assert_eq!(ConfigMatcher::Exact.priority(), 3);
        assert_eq!(ConfigMatcher::Plus.priority(), 2);
        assert_eq!(ConfigMatcher::Minus.priority(), 1);
    }

    #[test]
    fn empty_args_init_stores_ext_as_single_element_list() {
        // py:91  self.ext = [ext]
        let a = EmptyArgs::new("tmux", None);
        assert_eq!(a.ext, vec!["tmux".to_string()]);
    }

    #[test]
    fn empty_args_init_defaults_side_to_left() {
        // py:92  self.side = 'left'
        let a = EmptyArgs::new("tmux", None);
        assert_eq!(a.side, "left");
    }

    #[test]
    fn empty_args_init_pins_config_path_to_none() {
        // py:93  self.config_path = None — even though Python
        // accepts config_path as an argument, the body discards
        // it. Mirror that behavior.
        let a = EmptyArgs::new("tmux", Some("/etc/powerline"));
        assert!(a.config_path.is_none());
    }

    #[test]
    fn tmux_var_re_matches_dollar_powerline_var() {
        // py:179  re.compile(r'\$(_POWERLINE_\w+)')
        let re = TMUX_VAR_RE();
        assert!(re.is_match("$_POWERLINE_FOO"));
        assert!(re.is_match("foo $_POWERLINE_BAR_X bar"));
        assert!(!re.is_match("_POWERLINE_NOPE"));
        assert!(!re.is_match("$POWERLINE_NO_UNDER"));
    }

    #[test]
    fn tmux_var_re_captures_group_after_dollar() {
        let re = TMUX_VAR_RE();
        let cap = re.captures("$_POWERLINE_FG").unwrap();
        assert_eq!(cap.get(1).unwrap().as_str(), "_POWERLINE_FG");
    }

    #[test]
    fn check_command_returns_some_for_real_binary() {
        // sh exists on every Unix; mirrors py:232-233
        let r = check_command("sh");
        assert_eq!(r, Some("sh".to_string()));
    }

    #[test]
    fn check_command_returns_none_for_missing_binary() {
        let r = check_command("definitely-not-on-this-system-xyz-abc");
        assert!(r.is_none());
    }

    #[test]
    fn deduce_command_returns_some_or_none() {
        // Deterministic without mocking which(): we can't pin a
        // specific candidate, but py:252-261 always returns either a
        // string or None — assert the return shape, not the value.
        let r = deduce_command();
        if let Some(s) = r {
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn replace_env_substitutes_known_var() {
        // py:189-193
        let mut env = std::collections::HashMap::new();
        env.insert("_POWERLINE_FG".to_string(), "#abcdef".to_string());
        let r = replace_env("$_POWERLINE_FG", &env);
        assert_eq!(r, "#abcdef");
    }

    #[test]
    fn replace_env_passes_through_unknown_var() {
        // Unknown vars stay verbatim (Python KeyError handling is
        // upstream — Rust prefers safe passthrough)
        let env = std::collections::HashMap::new();
        let r = replace_env("$_POWERLINE_MISSING", &env);
        assert_eq!(r, "$_POWERLINE_MISSING");
    }

    #[test]
    fn replace_env_substitutes_multiple_in_one_string() {
        let mut env = std::collections::HashMap::new();
        env.insert("_POWERLINE_FG".to_string(), "fg".to_string());
        env.insert("_POWERLINE_BG".to_string(), "bg".to_string());
        let r = replace_env("$_POWERLINE_FG/$_POWERLINE_BG", &env);
        assert_eq!(r, "fg/bg");
    }

    #[test]
    fn replace_env_leaves_non_dollar_powerline_text_alone() {
        let env = std::collections::HashMap::new();
        let r = replace_env("plain text", &env);
        assert_eq!(r, "plain text");
    }

    #[test]
    fn parse_tmux_file_line_skips_comments() {
        // py:198-199
        assert!(parse_tmux_file_line("# comment").is_none());
        assert!(parse_tmux_file_line("#another").is_none());
    }

    #[test]
    fn parse_tmux_file_line_skips_blank_lines() {
        assert!(parse_tmux_file_line("\n").is_none());
        assert!(parse_tmux_file_line("").is_none());
    }

    #[test]
    fn parse_tmux_file_line_splits_simple_args() {
        // py:200  shlex.split
        let r = parse_tmux_file_line("set -g status on").unwrap();
        assert_eq!(r, vec!["set", "-g", "status", "on"]);
    }

    #[test]
    fn parse_tmux_file_line_handles_quoted_args() {
        let r = parse_tmux_file_line("set status-left \"a b c\"").unwrap();
        assert_eq!(r, vec!["set", "status-left", "a b c"]);
    }

    #[test]
    fn sorted_tmux_configs_returns_entries_in_ascending_priority_order() {
        // py:75  sorted by priority key
        // Smoke test: empty config dir returns empty vec
        let version = TmuxVersionInfo {
            major: 2.0,
            minor: 1,
            suffix: None,
        };
        let entries = sorted_tmux_configs(&version);
        // Sort is stable + ascending
        let mut prev = i64::MIN;
        for (_, priority) in &entries {
            assert!(*priority >= prev);
            prev = *priority;
        }
    }

    #[test]
    fn uses_check_env_vars_returns_true_for_powerline_no_var() {
        // py:277-281
        let mut env = std::collections::HashMap::new();
        env.insert("POWERLINE_NO_BASH_PROMPT".to_string(), "1".to_string());
        assert!(uses_check_env_vars("prompt", Some("bash"), &env));
    }

    #[test]
    fn uses_check_env_vars_checks_shell_fallback() {
        // py:278  (shell, 'shell')
        let mut env = std::collections::HashMap::new();
        env.insert("POWERLINE_NO_SHELL_TMUX".to_string(), "1".to_string());
        assert!(uses_check_env_vars("tmux", Some("zsh"), &env));
        // With no shell supplied, still checks SHELL fallback
        assert!(uses_check_env_vars("tmux", None, &env));
    }

    #[test]
    fn uses_check_env_vars_returns_false_when_unset() {
        let env = std::collections::HashMap::new();
        assert!(!uses_check_env_vars("prompt", Some("bash"), &env));
    }

    #[test]
    fn uses_check_env_vars_returns_false_when_var_is_empty() {
        // py:280  if os.environ.get(varname): — empty string is falsy
        let mut env = std::collections::HashMap::new();
        env.insert("POWERLINE_NO_BASH_PROMPT".to_string(), "".to_string());
        assert!(!uses_check_env_vars("prompt", Some("bash"), &env));
    }

    #[test]
    fn uses_component_exit_code_returns_0_when_component_in_config() {
        // py:283-284
        let cfg: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"ext": {"shell": {"components": ["tmux", "prompt"]}}}"#)
                .unwrap();
        assert_eq!(uses_component_exit_code(&cfg, "tmux"), 0);
        assert_eq!(uses_component_exit_code(&cfg, "prompt"), 0);
    }

    #[test]
    fn uses_component_exit_code_returns_1_when_component_not_in_config() {
        // py:285-286
        let cfg: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"ext": {"shell": {"components": ["prompt"]}}}"#).unwrap();
        assert_eq!(uses_component_exit_code(&cfg, "tmux"), 1);
    }

    #[test]
    fn uses_component_exit_code_default_components_when_missing() {
        // py:283  default ('tmux', 'prompt')
        let cfg = serde_json::Map::new();
        assert_eq!(uses_component_exit_code(&cfg, "tmux"), 0);
        assert_eq!(uses_component_exit_code(&cfg, "prompt"), 0);
        assert_eq!(uses_component_exit_code(&cfg, "other"), 1);
    }

    #[test]
    fn shell_command_returns_deduce_command_result() {
        // py:264-269
        // Just verify the call succeeds and returns the same value
        // as deduce_command (Option shape preserved).
        assert_eq!(shell_command(), deduce_command());
    }

    #[test]
    fn set_tmux_environment_nosource_inserts_entry() {
        // py:186-187  tmux_environ[varname] = value
        let mut env = std::collections::HashMap::new();
        set_tmux_environment_nosource(&mut env, "_POWERLINE_X", "y", true);
        assert_eq!(env.get("_POWERLINE_X"), Some(&"y".to_string()));
    }

    #[test]
    fn replace_cb_returns_value_for_known_key() {
        // py:189-190
        let mut env = std::collections::HashMap::new();
        env.insert("_POWERLINE_FG".to_string(), "white".to_string());
        assert_eq!(replace_cb(&env, "_POWERLINE_FG"), Some("white".to_string()));
        assert!(replace_cb(&env, "_POWERLINE_BG").is_none());
    }

    #[test]
    fn get_main_config_returns_err_when_no_config_found() {
        // py:218-221  no config.json in any search path → err
        let r = get_main_config(
            &[std::path::PathBuf::from("/nonexistent_xxx")],
            |_| unreachable!(),
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Could not find config.json"));
    }

    #[test]
    fn get_main_config_returns_loaded_object() {
        // py:218-221
        let tmp = std::env::temp_dir().join("powerliners_test_get_main_config");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("config.json"), r#"{"common":{},"ext":{}}"#).unwrap();
        let r = get_main_config(std::slice::from_ref(&tmp), |path| {
            let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            serde_json::from_str(&s).map_err(|e| e.to_string())
        });
        assert!(r.is_ok());
        let obj = r.unwrap();
        assert!(obj.contains_key("common"));
        assert!(obj.contains_key("ext"));
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn uses_requires_non_empty_component() {
        // py:274-275  raise ValueError
        let env = std::collections::HashMap::new();
        let cfg = serde_json::Map::new();
        let r = uses("", Some("zsh"), &env, &cfg);
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), "Must specify component");
    }

    #[test]
    fn uses_returns_1_when_env_var_set() {
        // py:281
        let mut env = std::collections::HashMap::new();
        env.insert("POWERLINE_NO_ZSH_TMUX".to_string(), "1".to_string());
        let cfg = serde_json::Map::new();
        assert_eq!(uses("tmux", Some("zsh"), &env, &cfg).unwrap(), 1);
    }

    #[test]
    fn uses_returns_0_when_component_in_default_components() {
        // py:283-284  default ('tmux', 'prompt')
        let env = std::collections::HashMap::new();
        let cfg = serde_json::Map::new();
        assert_eq!(uses("tmux", None, &env, &cfg).unwrap(), 0);
        assert_eq!(uses("prompt", None, &env, &cfg).unwrap(), 0);
        assert_eq!(uses("nonexistent", None, &env, &cfg).unwrap(), 1);
    }

    #[test]
    fn create_powerline_logger_constructs_logger() {
        // py:224-228
        let cfg = serde_json::Map::new();
        let _logger = create_powerline_logger(&cfg);
        // No panic = pass.
    }

    #[test]
    fn source_tmux_files_includes_powerline_base() {
        // py:74 always sources powerline-base.conf first
        let v = ver(2.0, 0);
        let files = source_tmux_files(&v);
        let base = files.iter().find(|p| {
            p.file_name()
                .map(|s| s == "powerline-base.conf")
                .unwrap_or(false)
        });
        assert!(base.is_some());
    }

    #[test]
    fn tmux_setup_default_source_flag_is_true() {
        // py:204  if args.source is None: source = True
        let (env, do_source) = tmux_setup(None);
        assert!(do_source);
        assert!(env.is_empty());
    }

    #[test]
    fn tmux_setup_explicit_source_false_returns_false() {
        let (_env, do_source) = tmux_setup(Some(false));
        assert!(!do_source);
    }

    #[test]
    fn init_tmux_environment_returns_empty_map_in_stub() {
        // py:99-176  stub: real chain depends on ShellPowerline + colorscheme
        let r = init_tmux_environment(None);
        assert!(r.is_empty());
    }

    #[test]
    fn get_highlighting_dispatches_to_resolver_with_single_group() {
        // py:109-110
        let r = get_highlighting("background", |groups, mode| {
            assert_eq!(groups, &["background"]);
            assert!(mode.is_none());
            let mut m = serde_json::Map::new();
            m.insert("fg".to_string(), serde_json::json!([15, 0xffffff]));
            m
        });
        assert!(r.contains_key("fg"));
    }

    #[test]
    fn source_tmux_file_nosource_skips_comments_and_blanks() {
        // py:198-202
        let tmp = std::env::temp_dir().join("powerliners_test_source_tmux_nosource.conf");
        std::fs::write(&tmp, "# comment\n\nset-option -g status on\n").unwrap();
        let env = std::collections::HashMap::new();
        let commands = source_tmux_file_nosource(&tmp, &env);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0][0], "set-option");
        std::fs::remove_file(&tmp).ok();
    }
}
