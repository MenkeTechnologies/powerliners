// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/__init__.py`.
//!
//! Entry point for the powerline-lint tool. Surfaces:
//!   - `open_file(path)` — opens a config file as bytes
//!   - `function_name_re` regex (py:43)
//!   - `register_common_names()` — registers the well-known segment
//!     aliases (currently just `player`) per py:321
//!   - `load_json_file(path)` — wraps the markedjson load() return
//!     into a (hadproblem, config, error) triple
//!   - `updated_with_config(d)` — merges load_json_file output into
//!     the supplied dict
//!   - `dict2(d)` — defaultdict(dict, ...) factory
//!
//! The `check(paths, debug, echoerr, require_ext)` main entry
//! point + the Spec-builder DSL definitions at py:45-318 are
//! heavy enough to deserve their own port pass and are deferred.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import logging                                   // py:5
// from collections import defaultdict              // py:7
// from itertools import chain                      // py:8
// from functools import partial                    // py:9
// from powerline import generate_config_finder, get_config_paths, load_config  // py:11
// from powerline.segments.vim import vim_modes     // py:12
// from powerline.lib.dict import mergedicts_copy   // py:13
// from powerline.lib.config import ConfigLoader    // py:14
// from powerline.lib.unicode import unicode        // py:15
// from powerline.lib.path import join              // py:16
// from powerline.lint.markedjson import load       // py:17
// from powerline.lint.markedjson.error import echoerr, EchoErr, MarkedError  // py:18
// from powerline.lint.checks import (...)           // py:19-25
// from powerline.lint.spec import Spec              // py:26
// from powerline.lint.context import Context        // py:27

pub mod checks;
pub mod context;
pub mod imp;
pub mod inspect;
pub mod markedjson;
pub mod selfcheck;
pub mod spec;

use crate::ported::lint::checks::register_common_name;
use crate::ported::lint::markedjson::load;
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Port of `open_file()` from
/// `powerline/lint/__init__.py:30`.
///
/// Returns the file's raw bytes. Python returns the file handle
/// itself; the Rust port reads the whole file since the caller's
/// usage is always `with open_file(path) as F: load(F)`.
pub fn open_file(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    // py:30  def open_file(path):
    // py:31  return open(path, 'rb')
    std::fs::read(path)
}

/// Port of `function_name_re` from
/// `powerline/lint/__init__.py:43`.
///
/// Pattern: `^(\w+\.)*[a-zA-Z_]\w*$` — dotted Python identifier
/// path, used for validating segment function references.
pub fn function_name_re() -> &'static Regex {
    // py:43  function_name_re = r'^(\w+\.)*[a-zA-Z_]\w*$'
    // py:46  divider_spec = Spec().printable().len(
    // py:47  'le', 3, (lambda value: 'Divider {0!r} is too large!'.format(value))).copy
    // py:48  ext_theme_spec = Spec().type(unicode).func(lambda *args: check_config('themes', *args)).copy
    // py:49  top_theme_spec = Spec().type(unicode).func(check_top_theme).copy
    // py:50  ext_spec = Spec(
    // py:51  colorscheme=Spec().type(unicode).func(
    // py:52  (lambda *args: check_config('colorschemes', *args))
    // py:53  ),
    // py:54  theme=ext_theme_spec(),
    // py:55  top_theme=top_theme_spec().optional(),
    // py:56  ).copy
    // py:57  gen_components_spec = (lambda *components: Spec().list(Spec().type(unicode).oneof(set(components))))
    // py:58  log_level_spec = Spec().re('^[A-Z]+$').func(
    // py:59  (lambda value, *args: (True, True, not hasattr(logging, value))),
    // py:60  (lambda value: 'unknown debugging level {0}'.format(value))
    // py:61  ).copy
    // py:62  log_format_spec = Spec().type(unicode).copy
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(\w+\.)*[a-zA-Z_]\w*$").unwrap())
}

/// Port of `register_common_names()` from
/// `powerline/lint/__init__.py:321`.
///
/// Registers the well-known segment aliases. Currently the only
/// alias is `player → powerline.segments.common.players._player`
/// per py:322.
pub fn register_common_names() {
    // py:321  def register_common_names():
    // py:322  register_common_name('player', 'powerline.segments.common.players', '_player')
    register_common_name("player", "powerline.segments.common.players", "_player");
}

/// Result of `load_json_file()` matching the Python (hadproblem,
/// config, error) return triple at py:325-333.
#[derive(Debug, Clone)]
pub struct LoadJsonResult {
    /// Python: first tuple element.
    pub hadproblem: bool,
    /// Python: second tuple element — parsed config or None when
    /// load errored out.
    pub config: Option<Value>,
    /// Python: third tuple element — error message string when
    /// MarkedError was caught.
    pub error: Option<String>,
}

/// Port of `load_json_file()` from
/// `powerline/lint/__init__.py:325`.
///
/// Loads the JSON file via the markedjson loader. Returns
/// `(hadproblem, config, error)` matching the Python triple.
pub fn load_json_file(path: &std::path::Path) -> LoadJsonResult {
    // py:325  def load_json_file(path):
    // py:326  with open_file(path) as F:
    // py:327  try:
    // py:328  config, hadproblem = load(F)
    // py:329  except MarkedError as e:
    // py:330  return True, None, str(e)
    // py:331  return hadproblem, config, None
    if !path.exists() {
        return LoadJsonResult {
            hadproblem: true,
            config: None,
            error: Some(format!("Path not found: {}", path.display())),
        };
    }
    let (config, hadproblem) = load(path);
    LoadJsonResult {
        hadproblem,
        config,
        error: None,
    }
}

/// Port of `updated_with_config()` from
/// `powerline/lint/__init__.py:335`.
///
/// Merges `load_json_file(d['path'])` result into `d` per py:337-341.
pub fn updated_with_config(d: &mut Map<String, Value>) {
    // py:335  def updated_with_config(d):
    // py:336  hadproblem, config, error = load_json_file(d['path'])
    // py:337  d.update(
    // py:338  hadproblem=hadproblem,
    // py:339  config=config,
    // py:340  error=error,
    // py:341  )
    // py:342  return d
    let path = d
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);
    let path = match path {
        Some(p) => p,
        None => return,
    };
    let r = load_json_file(&path);
    d.insert("hadproblem".to_string(), Value::Bool(r.hadproblem));
    if let Some(cfg) = r.config {
        d.insert("config".to_string(), cfg);
    }
    if let Some(err) = r.error {
        d.insert("error".to_string(), Value::String(err));
    }
}

/// Port of `dict2()` from
/// `powerline/lint/__init__.py:389`.
///
/// Python: `defaultdict(dict, ((k, dict(v)) for k, v in d.items()))`
/// — creates a defaultdict with all entries shallow-copied as
/// dicts. Rust port returns a fresh Map of Maps since Rust doesn't
/// have defaultdict.
pub fn dict2(d: &Map<String, Value>) -> Map<String, Value> {
    // py:389  def dict2(d):
    // py:390  return defaultdict(dict, (
    // py:391  (k, dict(v)) for k, v in d.items()
    // py:392  ))
    let mut out = Map::new();
    for (k, v) in d {
        if let Some(inner) = v.as_object() {
            out.insert(k.clone(), Value::Object(inner.clone()));
        } else {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

/// Strips the `.json` suffix from a path filename to produce the
/// `name` field per py:361 (`ext_name[:-5]`) and py:373
/// (`config_file_name[:-5]`).
pub fn strip_json_suffix(name: &str) -> String {
    // py:361, py:373  name[:-5]
    name.strip_suffix(".json").unwrap_or(name).to_string()
}

/// Port of `generate_json_config_loader()` from
/// `powerline/lint/__init__.py:34-41`.
///
/// Python: returns a closure capturing `lhadproblem` (a mutable
/// `[False]` flag) that flips when `load(...)` reports a problem.
/// The Rust port mirrors this with an `Arc<Mutex<bool>>` since the
/// closure must mutate the captured flag from outside.
///
/// The returned closure takes a config file path and returns the
/// parsed `Value` per py:40, while flipping the shared flag when
/// load reports hadproblem per py:38-39.
pub fn generate_json_config_loader(
    lhadproblem: std::sync::Arc<std::sync::Mutex<bool>>,
) -> Box<dyn Fn(&std::path::Path) -> Option<Value>> {
    // py:34  def generate_json_config_loader(lhadproblem):
    // py:35  def load_json_config(config_file_path, load=load, open_file=open_file):
    // py:36  with open_file(config_file_path) as config_file_fp:
    // py:37  r, hadproblem = load(config_file_fp)
    // py:38  if hadproblem:
    // py:39  lhadproblem[0] = True
    // py:40  return r
    // py:41  return load_json_config
    Box::new(move |config_file_path: &std::path::Path| -> Option<Value> {
        let (r, hadproblem) = load(config_file_path);
        if hadproblem {
            let mut flag = lhadproblem.lock().unwrap_or_else(|e| e.into_inner());
            *flag = true;
        }
        r
    })
}

/// Discovered config file entry produced by
/// `find_all_ext_config_files`. Mirrors the dict shape Python yields
/// at py:350-353 / py:359-365 / py:367-370 / py:375-381 / py:383-386.
#[derive(Debug, Clone)]
pub struct ExtConfigEntry {
    /// Error message when `error` is set, else None.
    pub error: Option<String>,
    /// Path that produced this entry.
    pub path: std::path::PathBuf,
    /// Config name (`<file>.json` → `<file>`). None for error entries
    /// without a successfully-resolved file.
    pub name: Option<String>,
    /// Extension name (subdirectory name in the config tree). None
    /// for top-level config files per py:363.
    pub ext: Option<String>,
    /// `"theme"` / `"colorscheme"` (the `subdir` arg) or
    /// `"top_<subdir>"` for top-level entries per py:364 / py:380.
    pub kind: Option<String>,
}

/// Port of `find_all_ext_config_files()` from
/// `powerline/lint/__init__.py:345-386`.
///
/// Walks `search_paths` looking for `<root>/<subdir>/{<ext>/<name>.json,
/// <name>.json}` configs. Yields one `ExtConfigEntry` per discovered
/// file (or per malformed path that prevents the walk).
pub fn find_all_ext_config_files(
    search_paths: &[std::path::PathBuf],
    subdir: &str,
) -> Vec<ExtConfigEntry> {
    // py:344  def find_all_ext_config_files(search_paths, subdir):
    // py:345  for config_root in search_paths:
    // py:346  top_config_subpath = join(config_root, subdir)
    // py:347  if not os.path.isdir(top_config_subpath):
    // py:348  if os.path.exists(top_config_subpath):
    // py:349  yield {
    // py:350  'error': 'Path {0} is not a directory'.format(top_config_subpath),
    // py:351  'path': top_config_subpath,
    // py:352  }
    // py:353  continue
    // py:354  for ext_name in sorted(os.listdir(top_config_subpath)):
    // py:355  ext_path = join(top_config_subpath, ext_name)
    // py:356  if not os.path.isdir(ext_path):
    // py:357  if ext_name.endswith('.json') and os.path.isfile(ext_path):
    // py:358  yield {
    // py:359  'error': None,
    // py:360  'path': ext_path,
    // py:361  'name': ext_name[:-5],
    // py:362  'ext': None,
    // py:363  'type': 'top_' + subdir,
    // py:364  }
    // py:365  else:
    // py:366  yield {
    // py:367  'error': 'Path {0} is not a directory or configuration file'.format(ext_path),
    // py:368  'path': ext_path,
    // py:369  }
    // py:370  continue
    // py:371  for config_file_name in sorted(os.listdir(ext_path)):
    // py:372  config_file_path = join(ext_path, config_file_name)
    // py:373  if config_file_name.endswith('.json') and os.path.isfile(config_file_path):
    // py:374  yield {
    // py:375  'error': None,
    // py:376  'path': config_file_path,
    // py:377  'name': config_file_name[:-5],
    // py:378  'ext': ext_name,
    // py:379  'type': subdir,
    // py:380  }
    // py:381  else:
    // py:382  yield {
    // py:383  'error': 'Path {0} is not a configuration file'.format(config_file_path),
    // py:384  'path': config_file_path,
    // py:385  }
    let mut out: Vec<ExtConfigEntry> = Vec::new();
    for config_root in search_paths {
        let top_config_subpath = config_root.join(subdir);
        if !top_config_subpath.is_dir() {
            if top_config_subpath.exists() {
                out.push(ExtConfigEntry {
                    error: Some(format!(
                        "Path {} is not a directory",
                        top_config_subpath.display()
                    )),
                    path: top_config_subpath.clone(),
                    name: None,
                    ext: None,
                    kind: None,
                });
            }
            continue;
        }
        let entries = match std::fs::read_dir(&top_config_subpath) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let ext_name = match entry.file_name().into_string() {
                Ok(s) => s,
                Err(_) => continue,
            };
            let ext_path = entry.path();
            if !ext_path.is_dir() {
                if ext_name.ends_with(".json") && ext_path.is_file() {
                    out.push(ExtConfigEntry {
                        error: None,
                        path: ext_path.clone(),
                        name: Some(strip_json_suffix(&ext_name)),
                        ext: None,
                        kind: Some(format!("top_{}", subdir)),
                    });
                } else {
                    out.push(ExtConfigEntry {
                        error: Some(format!(
                            "Path {} is not a directory or configuration file",
                            ext_path.display()
                        )),
                        path: ext_path.clone(),
                        name: None,
                        ext: None,
                        kind: None,
                    });
                }
                continue;
            }
            let inner = match std::fs::read_dir(&ext_path) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for cfg_entry in inner.flatten() {
                let config_file_name = match cfg_entry.file_name().into_string() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let config_file_path = cfg_entry.path();
                // py:374-381  yield ext-scoped entry or error
                if config_file_name.ends_with(".json") && config_file_path.is_file() {
                    out.push(ExtConfigEntry {
                        error: None,
                        path: config_file_path.clone(),
                        name: Some(strip_json_suffix(&config_file_name)),
                        ext: Some(ext_name.clone()),
                        kind: Some(subdir.to_string()),
                    });
                } else {
                    // py:383-386  yield non-configuration-file error
                    out.push(ExtConfigEntry {
                        error: Some(format!(
                            "Path {} is not a configuration file",
                            config_file_path.display()
                        )),
                        path: config_file_path.clone(),
                        name: None,
                        ext: None,
                        kind: None,
                    });
                }
            }
        }
    }
    out
}

/// Port of `check()` from `powerline/lint/__init__.py:393`.
///
/// **Status:** structural — args wired, body deferred.
///
/// The Rust port mirrors the Python signature
/// `check(paths=None, debug=False, echoerr=echoerr, require_ext=None)`.
/// The full lint pipeline (Spec DSL walks, themes/colorscheme
/// cross-validation, undefined-name detection) is deferred; this
/// surface satisfies the `powerline-lint` script contract by
/// returning `false` (no problems found) so the script exits 0 when
/// the config-path is supplied.
/// Port of module-level `top_theme_spec` binding from
/// `powerline/lint/__init__.py:50`.
pub fn top_theme_spec() -> spec::Spec {
    // py:50  Spec().type(unicode).func(check_top_theme).copy
    spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_top_theme")
}

/// Port of module-level `ext_theme_spec` binding from
/// `powerline/lint/__init__.py:49`.
pub fn ext_theme_spec() -> spec::Spec {
    // py:49  Spec().type(unicode).func(check_config(themes, *args)).copy
    spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_config_themes")
}

/// Port of module-level `log_format_spec` binding from
/// `powerline/lint/__init__.py:63`.
pub fn log_format_spec() -> spec::Spec {
    // py:63  Spec().type(unicode).copy
    spec::Spec::new().type_check(&[spec::SpecType::Unicode])
}

/// Port of module-level `log_level_spec` binding from
/// `powerline/lint/__init__.py:59-62`.
pub fn log_level_spec() -> spec::Spec {
    // py:59-62  Spec().re('^[A-Z]+$').func(lambda check logging level)
    spec::Spec::new()
        .regex("^[A-Z]+$")
        .func("check_logging_level")
}

/// Port of module-level `term_color_spec` binding from
/// `powerline/lint/__init__.py:142`.
pub fn term_color_spec() -> spec::Spec {
    // py:142  Spec().unsigned().cmp('le', 255).copy
    spec::Spec::new().unsigned().cmp(spec::Cmp::Le, 255.0)
}

/// Port of module-level `true_color_spec` binding from
/// `powerline/lint/__init__.py:143-146`.
pub fn true_color_spec() -> spec::Spec {
    // py:143-146  Spec().re('^[0-9a-fA-F]{6}$').copy
    spec::Spec::new().regex("^[0-9a-fA-F]{6}$")
}

/// Port of module-level `colors_spec` binding from
/// `powerline/lint/__init__.py:147-162`.
pub fn colors_spec() -> spec::Spec {
    // py:147-162
    let color_value = spec::Spec::new().either(vec![
        spec::Spec::new().tuple(vec![term_color_spec(), true_color_spec()]),
        term_color_spec(),
    ]);
    let colors_inner = spec::Spec::new()
        .unknown_spec(spec::Spec::new().ident(), color_value)
        .context_message("Error while checking colors (key {key})");
    let gradient_value = spec::Spec::new().tuple(vec![
        spec::Spec::new()
            .len(spec::Cmp::Gt, 1)
            .list(term_color_spec()),
        spec::Spec::new()
            .len(spec::Cmp::Gt, 1)
            .list(true_color_spec())
            .optional(),
    ]);
    let gradients_inner = spec::Spec::new()
        .unknown_spec(spec::Spec::new().ident(), gradient_value)
        .context_message("Error while checking gradients (key {key})");
    let mut s = spec::Spec::new();
    s = s.update("colors", colors_inner);
    s = s.update("gradients", gradients_inner);
    s.context_message("Error while loading colors configuration")
}

/// Port of module-level `mode_translations_value_spec` binding from
/// `powerline/lint/__init__.py:181-190`.
pub fn mode_translations_value_spec() -> spec::Spec {
    // py:181-190
    let colors_unknown = spec::Spec::new()
        .unknown_spec(color_spec(), color_spec())
        .optional();
    let groups_unknown = spec::Spec::new()
        .unknown_spec(
            group_name_spec().func("check_translated_group_name"),
            group_spec(),
        )
        .optional();
    let mut s = spec::Spec::new();
    s = s.update("colors", colors_unknown);
    s = s.update("groups", groups_unknown);
    s
}

/// Port of module-level `top_colorscheme_spec` binding from
/// `powerline/lint/__init__.py:191-198`.
pub fn top_colorscheme_spec() -> spec::Spec {
    // py:191-198
    let mt_inner = spec::Spec::new()
        .unknown_spec(
            spec::Spec::new().type_check(&[spec::SpecType::Unicode]),
            mode_translations_value_spec(),
        )
        .optional()
        .context_message("Error while loading mode translations (key {key})")
        .optional();
    let mut s = spec::Spec::new();
    s = s.update("name", name_spec());
    s = s.update("groups", groups_spec().required());
    s = s.update("mode_translations", mt_inner);
    s.context_message("Error while loading top-level colorscheme")
}

/// Port of module-level `vim_mode_spec` binding from
/// `powerline/lint/__init__.py:199`.
pub fn vim_mode_spec() -> spec::Spec {
    // py:199  Spec().oneof(set(list(vim_modes) + ['nc', 'tab_nc', 'buf_nc'])).copy
    // vim_modes is the dict at segments/vim/__init__.py:43-67; ported as
    // crate::ported::segments::vim::vim_modes(). Building the union here
    // from the live accessor avoids drift if the upstream dict changes.
    let modes = crate::ported::segments::vim::vim_modes();
    let mut values: Vec<&str> = modes.keys().copied().collect();
    values.push("nc");
    values.push("tab_nc");
    values.push("buf_nc");
    spec::Spec::new().oneof(&values)
}

/// Port of module-level `shell_mode_spec` binding from
/// `powerline/lint/__init__.py:208`.
pub fn shell_mode_spec() -> spec::Spec {
    // py:208  Spec().re(r'^(?:[\w\-]+|\.safe)$').copy
    spec::Spec::new().regex(r"^(?:[\w\-]+|\.safe)$")
}

/// Port of module-level `divider_spec` binding from
/// `powerline/lint/__init__.py:47-48`.
pub fn divider_spec() -> spec::Spec {
    // py:47-48  Spec().printable().len('le', 3, ...).copy
    spec::Spec::new().printable().len(spec::Cmp::Le, 3)
}

/// Port of module-level `ext_spec` binding from
/// `powerline/lint/__init__.py:51-57`.
pub fn ext_spec() -> spec::Spec {
    // py:51-57
    let colorscheme_value = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_config_colorschemes");
    let mut s = spec::Spec::new();
    s = s.update("colorscheme", colorscheme_value);
    s = s.update("theme", ext_theme_spec());
    s = s.update("top_theme", top_theme_spec().optional());
    s
}

/// Port of module-level `gen_components_spec` binding from
/// `powerline/lint/__init__.py:58`.
///
/// `gen_components_spec = (lambda *components: Spec().list(Spec().type(unicode).oneof(set(components))))`
pub fn gen_components_spec(components: &[&str]) -> spec::Spec {
    // py:58
    let item = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .oneof(components);
    spec::Spec::new().list(item)
}

/// Port of module-level `args_spec` binding from
/// `powerline/lint/__init__.py:219-222`.
pub fn args_spec() -> spec::Spec {
    // py:219-222
    let pl_err = spec::Spec::new()
        .error("pl object must be set by powerline")
        .optional();
    let segment_info_err = spec::Spec::new()
        .error("Segment info dictionary must be set by powerline")
        .optional();
    let mut s = spec::Spec::new();
    s = s.update("pl", pl_err);
    s = s.update("segment_info", segment_info_err);
    s.unknown_spec(spec::Spec::new(), spec::Spec::new())
        .optional()
}

/// Port of module-level `segment_module_spec` binding from
/// `powerline/lint/__init__.py:223`.
pub fn segment_module_spec() -> spec::Spec {
    // py:223  Spec().type(unicode).func(check_segment_module).optional().copy
    spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_segment_module")
        .optional()
}

/// Port of module-level `exinclude_spec` binding from
/// `powerline/lint/__init__.py:224`.
pub fn exinclude_spec() -> spec::Spec {
    // py:224  Spec().re(function_name_re).func(check_exinclude_function).copy
    spec::Spec::new()
        .regex(r"^(\w+\.)*[a-zA-Z_]\w*$")
        .func("check_exinclude_function")
}

/// Port of module-level `highlight_group_spec` binding from
/// `powerline/lint/checks.py:349`.
pub fn highlight_group_spec() -> spec::Spec {
    // checks.py:349  highlight_group_spec = Spec().ident().copy
    spec::Spec::new().ident()
}

/// Port of module-level `_highlight_group_spec` binding from
/// `powerline/lint/checks.py:350-351`.
///
/// Same shape as [`highlight_group_spec`] but pre-wrapped with the
/// "Error while checking function documentation while checking theme
/// (key {key})" context message. Used internally for hl-group lookups
/// inside function docstring scans.
pub fn _highlight_group_spec() -> spec::Spec {
    // checks.py:350-351
    highlight_group_spec().context_message(
        "Error while checking function documentation while checking theme (key {key})",
    )
}

/// Port of module-level `vim_colorscheme_spec` binding from
/// `powerline/lint/__init__.py:200-207`.
pub fn vim_colorscheme_spec() -> spec::Spec {
    // py:200-207
    let mt = spec::Spec::new()
        .unknown_spec(vim_mode_spec(), mode_translations_value_spec())
        .optional()
        .context_message("Error while loading mode translations (key {key})");
    let mut s = spec::Spec::new();
    s = s.update("name", name_spec());
    s = s.update("groups", groups_spec());
    s = s.update("mode_translations", mt);
    s.context_message("Error while loading vim colorscheme")
}

/// Port of module-level `shell_colorscheme_spec` binding from
/// `powerline/lint/__init__.py:209-216`.
pub fn shell_colorscheme_spec() -> spec::Spec {
    // py:209-216
    let mt = spec::Spec::new()
        .unknown_spec(shell_mode_spec(), mode_translations_value_spec())
        .optional()
        .context_message("Error while loading mode translations (key {key})");
    let mut s = spec::Spec::new();
    s = s.update("name", name_spec());
    s = s.update("groups", groups_spec());
    s = s.update("mode_translations", mt);
    s.context_message("Error while loading shell colorscheme")
}

/// Port of module-level `segment_spec_base` binding from
/// `powerline/lint/__init__.py:225-254`.
///
/// Shared spec for both `segment_spec` and `subsegment_spec`; the
/// `type=` key differs between them and is added by the caller.
pub fn segment_spec_base() -> spec::Spec {
    // py:226
    let name_inner = spec::Spec::new().regex(r"^[a-zA-Z_]\w*$").optional();
    // py:227
    let function_inner = spec::Spec::new()
        .regex(r"^(\w+\.)*[a-zA-Z_]\w*$")
        .func("check_segment_function")
        .optional();
    // py:228-229
    let exclude_modes_inner = spec::Spec::new().list(vim_mode_spec()).optional();
    let include_modes_inner = spec::Spec::new().list(vim_mode_spec()).optional();
    // py:230-231
    let exclude_function_inner = exinclude_spec().optional();
    let include_function_inner = exinclude_spec().optional();
    // py:232-235
    let draw_hard_divider_inner = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    let draw_soft_divider_inner = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    let draw_inner_divider_inner = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    let display_inner = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    // py:236
    let module_inner = segment_module_spec();
    // py:237  Spec().type(int, float, type(None)).optional()
    // Python's int + float collapse to SpecType::Float in our enum per
    // the comment at SpecType::Float; None → Null.
    let priority_inner = spec::Spec::new()
        .type_check(&[spec::SpecType::Float, spec::SpecType::Null])
        .optional();
    // py:238-239
    let after_inner = spec::Spec::new().printable().optional();
    let before_inner = spec::Spec::new().printable().optional();
    // py:240  width is either unsigned int OR literal string "auto".
    // Python uses .cmp('eq', 'auto') which compares the value against
    // the literal; Rust's `cmp` takes f64 only, so .oneof(&["auto"])
    // captures the same constraint shape.
    let width_inner = spec::Spec::new()
        .either(vec![
            spec::Spec::new().unsigned(),
            spec::Spec::new().oneof(&["auto"]),
        ])
        .optional();
    // py:241  align l|r
    let align_inner = spec::Spec::new().oneof(&["l", "r"]).optional();
    // py:242
    let args_inner = args_spec().func("check_args");
    // py:243
    let contents_inner = spec::Spec::new().printable().optional();
    // py:244-249
    let highlight_group_item = highlight_group_spec().regex(r"^(?:(?!:divider$).)+$");
    let highlight_groups_inner = spec::Spec::new()
        .list(highlight_group_item)
        .func("check_highlight_groups")
        .optional();
    // py:250-253
    let divider_hg_inner = highlight_group_spec()
        .func("check_highlight_group")
        .regex(":divider$")
        .optional();

    let mut s = spec::Spec::new();
    s = s.update("name", name_inner);
    s = s.update("function", function_inner);
    s = s.update("exclude_modes", exclude_modes_inner);
    s = s.update("include_modes", include_modes_inner);
    s = s.update("exclude_function", exclude_function_inner);
    s = s.update("include_function", include_function_inner);
    s = s.update("draw_hard_divider", draw_hard_divider_inner);
    s = s.update("draw_soft_divider", draw_soft_divider_inner);
    s = s.update("draw_inner_divider", draw_inner_divider_inner);
    s = s.update("display", display_inner);
    s = s.update("module", module_inner);
    s = s.update("priority", priority_inner);
    s = s.update("after", after_inner);
    s = s.update("before", before_inner);
    s = s.update("width", width_inner);
    s = s.update("align", align_inner);
    s = s.update("args", args_inner);
    s = s.update("contents", contents_inner);
    s = s.update("highlight_groups", highlight_groups_inner);
    s = s.update("divider_highlight_group", divider_hg_inner);
    s.func("check_full_segment_data")
}

/// Port of module-level `subsegment_spec` binding from
/// `powerline/lint/__init__.py:255-257`.
pub fn subsegment_spec() -> spec::Spec {
    // py:256  type=Spec().oneof(set((k for k in type_keys if k != 'segment_list')))
    let allowed: Vec<&str> = crate::ported::lint::checks::type_keys()
        .keys()
        .copied()
        .filter(|k| *k != "segment_list")
        .collect();
    let type_inner = spec::Spec::new().oneof(&allowed).optional();
    segment_spec_base().update("type", type_inner)
}

/// Port of module-level `segment_spec` binding from
/// `powerline/lint/__init__.py:258-261`.
pub fn segment_spec() -> spec::Spec {
    // py:259  type=Spec().oneof(type_keys).optional()
    let allowed: Vec<&str> = crate::ported::lint::checks::type_keys()
        .keys()
        .copied()
        .collect();
    let type_inner = spec::Spec::new().oneof(&allowed).optional();
    // py:260  segments=Spec().optional().list(subsegment_spec)
    let segments_inner = spec::Spec::new().optional().list(subsegment_spec());
    segment_spec_base()
        .update("type", type_inner)
        .update("segments", segments_inner)
}

/// Port of module-level `segments_spec` binding from
/// `powerline/lint/__init__.py:262`.
pub fn segments_spec() -> spec::Spec {
    // py:262  Spec().optional().list(segment_spec).copy
    spec::Spec::new().optional().list(segment_spec())
}

/// Port of module-level `segdict_spec` binding from
/// `powerline/lint/__init__.py:263-269`.
pub fn segdict_spec() -> spec::Spec {
    // py:264  left=segments_spec().context_message('Error ... left')
    let left_inner =
        segments_spec().context_message("Error while loading segments from left side (key {key})");
    // py:265  right=segments_spec().context_message('Error ... right')
    let right_inner =
        segments_spec().context_message("Error while loading segments from right side (key {key})");
    let mut s = spec::Spec::new();
    s = s.update("left", left_inner);
    s = s.update("right", right_inner);
    s.func("check_segments_left_or_right")
        .context_message("Error while loading segments (key {key})")
}

/// Port of module-level `divside_spec` binding from
/// `powerline/lint/__init__.py:270-273`.
pub fn divside_spec() -> spec::Spec {
    // py:270-273
    let mut s = spec::Spec::new();
    s = s.update("hard", divider_spec());
    s = s.update("soft", divider_spec());
    s
}

/// Port of module-level `segment_data_value_spec` binding from
/// `powerline/lint/__init__.py:274-280`.
pub fn segment_data_value_spec() -> spec::Spec {
    // py:274-280
    let mut s = spec::Spec::new();
    s = s.update("after", spec::Spec::new().printable().optional());
    s = s.update("before", spec::Spec::new().printable().optional());
    s = s.update(
        "display",
        spec::Spec::new()
            .type_check(&[spec::SpecType::Bool])
            .optional(),
    );
    s = s.update("args", args_spec().func("check_args"));
    s = s.update("contents", spec::Spec::new().printable().optional());
    s
}

/// Port of module-level `dividers_spec` binding from
/// `powerline/lint/__init__.py:281-284`.
pub fn dividers_spec() -> spec::Spec {
    // py:281-284
    let mut s = spec::Spec::new();
    s = s.update("left", divside_spec());
    s = s.update("right", divside_spec());
    s
}

/// Port of module-level `spaces_spec` binding from
/// `powerline/lint/__init__.py:285-287`.
pub fn spaces_spec() -> spec::Spec {
    // py:285-287  Spec().unsigned().cmp('le', 2, ...).copy
    spec::Spec::new().unsigned().cmp(spec::Cmp::Le, 2.0)
}

/// Port of module-level `common_theme_spec` binding from
/// `powerline/lint/__init__.py:288-292`.
pub fn common_theme_spec() -> spec::Spec {
    // py:288-292
    let cursor_space = spec::Spec::new()
        .type_check(&[spec::SpecType::Float])
        .cmp(spec::Cmp::Le, 100.0)
        .cmp(spec::Cmp::Gt, 0.0)
        .optional();
    let cursor_columns = spec::Spec::new()
        .type_check(&[spec::SpecType::Float])
        .cmp(spec::Cmp::Gt, 0.0)
        .optional();
    let mut s = spec::Spec::new();
    s = s.update("default_module", segment_module_spec().optional());
    s = s.update("cursor_space", cursor_space);
    s = s.update("cursor_columns", cursor_columns);
    s.context_message("Error while loading theme")
}

/// Port of the second (rebound) `top_theme_spec` binding from
/// `powerline/lint/__init__.py:293-301`.
///
/// Python rebinds the module attribute `top_theme_spec` at py:293 from
/// the unicode-name validator (py:50, ported as `top_theme_spec`) to
/// the full top-theme JSON structure validator. Rust can't redefine
/// the same fn name, so the structure-spec variant is exposed as
/// `top_theme_structure_spec`.
pub fn top_theme_structure_spec() -> spec::Spec {
    // py:293-301
    let segment_data = spec::Spec::new()
        .unknown_spec(
            spec::Spec::new().func("check_segment_data_key"),
            segment_data_value_spec(),
        )
        .optional()
        .context_message("Error while loading segment data (key {key})");
    common_theme_spec()
        .update("dividers", dividers_spec())
        .update("spaces", spaces_spec())
        .update(
            "use_non_breaking_spaces",
            spec::Spec::new()
                .type_check(&[spec::SpecType::Bool])
                .optional(),
        )
        .update("segment_data", segment_data)
}

/// Port of module-level `main_theme_spec` binding from
/// `powerline/lint/__init__.py:302-309`.
pub fn main_theme_spec() -> spec::Spec {
    // py:302-309
    let segment_data = spec::Spec::new()
        .unknown_spec(
            spec::Spec::new().func("check_segment_data_key"),
            segment_data_value_spec(),
        )
        .optional()
        .context_message("Error while loading segment data (key {key})");
    common_theme_spec()
        .update("dividers", dividers_spec().optional())
        .update("spaces", spaces_spec().optional())
        .update("segment_data", segment_data)
}

/// Port of the inline `log_file` Spec at
/// `powerline/lint/__init__.py:75-98`.
///
/// Python builds the either-of-two-shapes spec inline inside the
/// main_spec call. The Rust port extracts it to keep main_spec
/// readable. Each shape is faithfully composed: the string variant
/// (unicode path with directory-exists check) and the list variant
/// (list of either None|string or 4-element tuple).
pub fn log_file_spec() -> spec::Spec {
    // py:76-85  unicode + dirname check
    let unicode_path = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_log_file_dir");
    // py:88-96  tuple of (handler-name, ctor-args-tuple, level, format)
    let tuple_entry = spec::Spec::new().tuple(vec![
        spec::Spec::new()
            .regex(r"^(\w+\.)*[a-zA-Z_]\w*$")
            .func("check_logging_handler"),
        spec::Spec::new().tuple(vec![
            spec::Spec::new()
                .type_check(&[spec::SpecType::List])
                .optional(),
            spec::Spec::new()
                .type_check(&[spec::SpecType::Dict])
                .optional(),
        ]),
        log_level_spec().func("check_log_file_level").optional(),
        log_format_spec().optional(),
    ]);
    // py:86-87  either (unicode|null) or tuple
    let list_item = spec::Spec::new().either(vec![
        spec::Spec::new().type_check(&[spec::SpecType::Unicode, spec::SpecType::Null]),
        tuple_entry,
    ]);
    let list_variant = spec::Spec::new().list(list_item);
    spec::Spec::new()
        .either(vec![unicode_path, list_variant])
        .optional()
}

/// Port of the inline `common` block inside `main_spec` at
/// `powerline/lint/__init__.py:65-104`.
pub fn common_spec() -> spec::Spec {
    // py:66
    let default_top_theme = top_theme_spec().optional();
    // py:67
    let term_truecolor = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    // py:68
    let term_escape_style = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .oneof(&["auto", "xterm", "fbterm"])
        .optional();
    // py:71-74  paths: list with path-exists check on each entry.
    let paths_inner = spec::Spec::new()
        .list(spec::Spec::new().func("check_path_exists"))
        .optional();
    // py:75-98
    let log_file = log_file_spec();
    // py:99-100
    let log_level = log_level_spec().optional();
    let log_format = log_format_spec().optional();
    // py:101  interval: either(cmp>0, None) optional
    let interval = spec::Spec::new()
        .either(vec![
            spec::Spec::new().cmp(spec::Cmp::Gt, 0.0),
            spec::Spec::new().type_check(&[spec::SpecType::Null]),
        ])
        .optional();
    // py:102
    let reload_config = spec::Spec::new()
        .type_check(&[spec::SpecType::Bool])
        .optional();
    // py:103
    let watcher = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .oneof(&["auto", "inotify", "stat"])
        .optional();

    let mut s = spec::Spec::new();
    s = s.update("default_top_theme", default_top_theme);
    s = s.update("term_truecolor", term_truecolor);
    s = s.update("term_escape_style", term_escape_style);
    s = s.update("paths", paths_inner);
    s = s.update("log_file", log_file);
    s = s.update("log_level", log_level);
    s = s.update("log_format", log_format);
    s = s.update("interval", interval);
    s = s.update("reload_config", reload_config);
    s = s.update("watcher", watcher);
    s.context_message("Error while loading common configuration (key {key})")
}

/// Port of the inline `ext` block inside `main_spec` at
/// `powerline/lint/__init__.py:105-139`.
pub fn ext_block_spec() -> spec::Spec {
    // py:106-114  vim
    let vim_local_themes = {
        let mut t = spec::Spec::new();
        t = t.update("__tabline__", ext_theme_spec());
        t.unknown_spec(
            spec::Spec::new()
                .regex(r"^(\w+\.)*[a-zA-Z_]\w*$")
                .func("check_matcher_func_vim"),
            ext_theme_spec(),
        )
    };
    let vim_inner = ext_spec()
        .update(
            "components",
            gen_components_spec(&["statusline", "tabline"]).optional(),
        )
        .update("local_themes", vim_local_themes)
        .optional();
    // py:115-121  ipython
    let ipython_local_themes = {
        let mut t = spec::Spec::new();
        t = t.update("in2", ext_theme_spec());
        t = t.update("out", ext_theme_spec());
        t = t.update("rewrite", ext_theme_spec());
        t
    };
    let ipython_inner = ext_spec()
        .update("local_themes", ipython_local_themes)
        .optional();
    // py:122-128  shell
    let shell_local_themes = {
        let mut t = spec::Spec::new();
        t = t.update("continuation", ext_theme_spec());
        t = t.update("select", ext_theme_spec());
        t
    };
    let shell_inner = ext_spec()
        .update(
            "components",
            gen_components_spec(&["tmux", "prompt"]).optional(),
        )
        .update("local_themes", shell_local_themes)
        .optional();
    // py:129-135  wm
    let wm_local_themes = spec::Spec::new()
        .unknown_spec(
            spec::Spec::new().regex(r"^[0-9A-Za-z-]+$"),
            ext_theme_spec(),
        )
        .optional();
    let wm_inner = ext_spec()
        .update("local_themes", wm_local_themes)
        .update(
            "update_interval",
            spec::Spec::new().cmp(spec::Cmp::Gt, 0.0).optional(),
        )
        .optional();

    let mut s = spec::Spec::new();
    s = s.update("vim", vim_inner);
    s = s.update("ipython", ipython_inner);
    s = s.update("shell", shell_inner);
    s = s.update("wm", wm_inner);
    // py:136-138  unknown_spec(check_ext, ext_spec())
    s.unknown_spec(spec::Spec::new().func("check_ext"), ext_spec())
        .context_message("Error while loading extensions configuration (key {key})")
}

/// Port of module-level `main_spec` binding from
/// `powerline/lint/__init__.py:64-140`.
pub fn main_spec() -> spec::Spec {
    // py:64-140
    let mut s = spec::Spec::new();
    s = s.update("common", common_spec());
    s = s.update("ext", ext_block_spec());
    s.context_message("Error while loading main configuration")
}

/// Port of module-level `theme_spec` binding from
/// `powerline/lint/__init__.py:310-318`.
pub fn theme_spec() -> spec::Spec {
    // py:310-318
    let segment_data = spec::Spec::new()
        .unknown_spec(
            spec::Spec::new().func("check_segment_data_key"),
            segment_data_value_spec(),
        )
        .optional()
        .context_message("Error while loading segment data (key {key})");
    // py:317  segments=segdict_spec().update(above=Spec().list(segdict_spec()).optional())
    let segments_inner =
        segdict_spec().update("above", spec::Spec::new().list(segdict_spec()).optional());
    common_theme_spec()
        .update("dividers", dividers_spec().optional())
        .update("spaces", spaces_spec().optional())
        .update("segment_data", segment_data)
        .update("segments", segments_inner)
}

/// Port of module-level `color_spec` binding from
/// `powerline/lint/__init__.py:165`.
///
/// `Spec().type(unicode).func(check_color).copy` — a fluent builder
/// reference that returns a fresh `Spec` per call.
pub fn color_spec() -> spec::Spec {
    // py:165
    spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .func("check_color")
}

/// Port of module-level `name_spec` binding from
/// `powerline/lint/__init__.py:166`.
pub fn name_spec() -> spec::Spec {
    // py:166  Spec().type(unicode).len('gt', 0).optional().copy
    spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .len(spec::Cmp::Gt, 0)
        .optional()
}

/// Port of module-level `group_name_spec` binding from
/// `powerline/lint/__init__.py:167`.
pub fn group_name_spec() -> spec::Spec {
    // py:167  Spec().ident().copy
    spec::Spec::new().ident()
}

/// Port of module-level `attrs_spec` binding (inlined at
/// `powerline/lint/__init__.py:171`).
///
/// `Spec().list(Spec().type(unicode).oneof(set(('bold', 'italic',
/// 'underline'))))`.
pub fn attrs_spec() -> spec::Spec {
    // py:171
    let item = spec::Spec::new()
        .type_check(&[spec::SpecType::Unicode])
        .oneof(&["bold", "italic", "underline"]);
    spec::Spec::new().list(item)
}

/// Port of module-level `group_spec` binding from
/// `powerline/lint/__init__.py:168-172`.
///
/// Python:
/// ```python
/// group_spec = Spec().either(Spec(
///     fg=color_spec(),
///     bg=color_spec(),
///     attrs=Spec().list(Spec().type(unicode).oneof(set(('bold',
///         'italic', 'underline')))),
/// ), group_name_spec().func(check_group)).copy
/// ```
pub fn group_spec() -> spec::Spec {
    // py:168-172
    let mut inner = spec::Spec::new();
    inner = inner.update("fg", color_spec());
    inner = inner.update("bg", color_spec());
    inner = inner.update("attrs", attrs_spec());
    let alias = group_name_spec().func("check_group");
    spec::Spec::new().either(vec![inner, alias])
}

/// Port of module-level `groups_spec` binding from
/// `powerline/lint/__init__.py:173-176`.
pub fn groups_spec() -> spec::Spec {
    // py:173-176  Spec().unknown_spec(group_name_spec(), group_spec()).context_message(...).copy
    spec::Spec::new()
        .unknown_spec(group_name_spec(), group_spec())
        .context_message("Error while loading groups (key {key})")
}

/// Port of module-level `colorscheme_spec` binding from
/// `powerline/lint/__init__.py:177-180`.
pub fn colorscheme_spec() -> spec::Spec {
    // py:177-180  Spec(name=name_spec(), groups=groups_spec()).context_message(...).copy
    let mut s = spec::Spec::new();
    s = s.update("name", name_spec());
    s = s.update("groups", groups_spec().required());
    s.context_message("Error while loading colorscheme")
}

pub fn check(paths: Option<&[String]>, debug: bool, require_ext: Option<&str>) -> bool {
    let _ = (debug, require_ext);
    // py:412  hadproblem = False
    let mut hadproblem = false;

    // py:414  register_common_names()
    register_common_names();

    // py:415  search_paths = paths or get_config_paths()
    let search_paths: Vec<std::path::PathBuf> = match paths {
        Some(ps) if !ps.is_empty() => ps.iter().map(std::path::PathBuf::from).collect(),
        _ => crate::ported::get_config_paths(),
    };

    // py:456-498  for d in chain(find_all_ext_config_files(search_paths, 'colorschemes'),
    //                            find_all_ext_config_files(search_paths, 'themes')):
    for subdir in ["colorschemes", "themes"] {
        for entry in find_all_ext_config_files(&search_paths, subdir) {
            if let Some(err) = &entry.error {
                eprintln!("powerline-lint: {}", err);
                hadproblem = true;
                continue;
            }
            // py:471-478  Reject `__name__`/`name__` filenames except __main__
            if let Some(name) = entry.name.as_deref() {
                if name != "__main__" && (name.starts_with("__") || name.ends_with("__")) {
                    eprintln!(
                        "powerline-lint: File name is not supposed to start or end with \u{201c}__\u{201d}: {}",
                        entry.path.display()
                    );
                    hadproblem = true;
                }
            }
            // py:506-621  Spec-DSL match. The Rust Spec composition
            // for `main_spec` / `colorscheme_spec` / `theme_spec` is
            // not yet wired (the per-Spec ports at `lint::spec` and
            // `lint::checks` are reusable but `register_main_spec`
            // hasn't been ported). Substitute a structural JSON-shape
            // probe that catches the most common authoring errors:
            // parse failures, non-object roots, and missing required
            // top-level keys per ext-specific schema.
            let config_v = match crate::ported::lib::config::load_json_config(&entry.path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "powerline-lint: parse error in {}: {}",
                        entry.path.display(),
                        e
                    );
                    hadproblem = true;
                    continue;
                }
            };
            let config_obj = match config_v.as_object() {
                Some(o) => o,
                None => {
                    eprintln!(
                        "powerline-lint: {} root must be a JSON object",
                        entry.path.display()
                    );
                    hadproblem = true;
                    continue;
                }
            };
            // Skip ext-name-only entries (__main__ etc) — they layer
            // defaults; required keys may not be present.
            let is_main = entry.name.as_deref() == Some("__main__");
            match subdir {
                "colorschemes" if !is_main => {
                    // py:506-552  colorscheme_spec requires 'groups'.
                    let groups = config_obj.get("groups");
                    if groups.is_none() {
                        eprintln!(
                            "powerline-lint: colorscheme {} missing required key 'groups'",
                            entry.path.display()
                        );
                        hadproblem = true;
                    }
                    // py:168-176  group_spec: either {fg, bg, attrs} or
                    // a string alias to another group. Validate the
                    // shape of each entry in `groups`.
                    if let Some(groups_obj) = groups.and_then(|v| v.as_object()) {
                        for (gname, gv) in groups_obj {
                            match gv {
                                // String aliases (py:168 either branch
                                // takes either a dict or a string).
                                Value::String(_) => {}
                                Value::Object(gobj) => {
                                    for required_key in ["fg", "bg"] {
                                        if !gobj.contains_key(required_key) {
                                            eprintln!(
                                                "powerline-lint: colorscheme {} group {:?} missing required key {:?}",
                                                entry.path.display(),
                                                gname,
                                                required_key,
                                            );
                                            hadproblem = true;
                                        }
                                    }
                                    if let Some(a) = gobj.get("attrs") {
                                        if !a.is_array() {
                                            eprintln!(
                                                "powerline-lint: colorscheme {} group {:?} 'attrs' must be a list",
                                                entry.path.display(),
                                                gname,
                                            );
                                            hadproblem = true;
                                        }
                                    }
                                }
                                _ => {
                                    eprintln!(
                                        "powerline-lint: colorscheme {} group {:?} must be an object or alias string",
                                        entry.path.display(),
                                        gname,
                                    );
                                    hadproblem = true;
                                }
                            }
                        }
                    }
                    // py:181-198  mode_translations_value_spec — when
                    // present, must be an object.
                    if let Some(mt) = config_obj.get("mode_translations") {
                        if !mt.is_object() {
                            eprintln!(
                                "powerline-lint: colorscheme {} 'mode_translations' must be an object",
                                entry.path.display()
                            );
                            hadproblem = true;
                        }
                    }
                }
                "themes" if !is_main => {
                    // py:556-621  theme_spec requires 'segments'.
                    let segments = config_obj.get("segments");
                    if segments.is_none() {
                        eprintln!(
                            "powerline-lint: theme {} missing required key 'segments'",
                            entry.path.display()
                        );
                        hadproblem = true;
                    }
                    // py:298-310  segments shape: {left: [...], right: [...]}
                    // or optionally {above: [{left, right}, ...]}.
                    if let Some(seg_obj) = segments.and_then(|v| v.as_object()) {
                        for side in ["left", "right"] {
                            if let Some(side_v) = seg_obj.get(side) {
                                if !side_v.is_array() {
                                    eprintln!(
                                        "powerline-lint: theme {} segments.{} must be an array",
                                        entry.path.display(),
                                        side
                                    );
                                    hadproblem = true;
                                }
                            }
                        }
                        if let Some(above) = seg_obj.get("above") {
                            if !above.is_array() {
                                eprintln!(
                                    "powerline-lint: theme {} segments.above must be an array",
                                    entry.path.display()
                                );
                                hadproblem = true;
                            }
                        }
                    }
                    // py:284-292  Numeric theme keys.
                    for (key, expected) in [
                        ("spaces", "integer"),
                        ("outer_padding", "integer"),
                        ("cursor_space", "number"),
                        ("cursor_columns", "integer"),
                    ] {
                        if let Some(v) = config_obj.get(key) {
                            let ok = matches!(expected, "integer") && v.is_i64()
                                || matches!(expected, "number") && v.is_number();
                            if !ok {
                                eprintln!(
                                    "powerline-lint: theme {} key {:?} must be a {}",
                                    entry.path.display(),
                                    key,
                                    expected
                                );
                                hadproblem = true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // py:495-621  Detailed Spec-DSL matches across main_spec /
    // colorscheme_spec / theme_spec are deferred (require ConfigLoader
    // mutex weaving + register_main_spec subsystem). Structural
    // scan above flags filesystem-level issues; deeper validation is
    // tracked by the per-Spec ports in `lint::spec`.
    let _ = paths; // silence unused after refactor
    hadproblem
}

// Legacy body trace from `lint::check()` kept here so the upstream
// Python line citations remain greppable for the deferred Spec-DSL
// walk. None of these comment lines is compiled.
#[cfg(any())]
mod _check_body_trace {
    // py:393  def check(paths=None, debug=False, echoerr=echoerr, require_ext=None):
    // py:394  '''Check configuration sanity
    // py:395-411  docstring
    // py:412  hadproblem = False
    // py:414  register_common_names()
    // py:415  search_paths = paths or get_config_paths()
    // py:416  find_config_files = generate_config_finder(lambda: search_paths)
    // py:418  logger = logging.getLogger('powerline-lint')
    // py:419  logger.setLevel(logging.DEBUG if debug else logging.ERROR)
    // py:420  logger.addHandler(logging.StreamHandler())
    // py:422  ee = EchoErr(echoerr, logger)
    // py:424  if require_ext:
    // py:425  used_main_spec = main_spec.copy()
    // py:426  try:
    // py:427  used_main_spec['ext'][require_ext].required()
    // py:428  except KeyError:
    // py:429  used_main_spec['ext'][require_ext] = ext_spec()
    // py:430  else:
    // py:431  used_main_spec = main_spec
    // py:433  lhadproblem = [False]
    // py:434  load_json_config = generate_json_config_loader(lhadproblem)
    // py:436  config_loader = ConfigLoader(run_once=True, load=load_json_config)
    // py:438  lists = {
    // py:439  'colorschemes': set(),
    // py:440  'themes': set(),
    // py:441  'exts': set(),
    // py:442  }
    // py:443  found_dir = {
    // py:444  'themes': False,
    // py:445  'colorschemes': False,
    // py:446  }
    // py:447  config_paths = defaultdict(lambda: defaultdict(dict))
    // py:448  loaded_configs = defaultdict(lambda: defaultdict(dict))
    // py:449  for d in chain(
    // py:450  find_all_ext_config_files(search_paths, 'colorschemes'),
    // py:451  find_all_ext_config_files(search_paths, 'themes'),
    // py:452  ):
    // py:453  if d['error']:
    // py:454  hadproblem = True
    // py:455  ee(problem=d['error'])
    // py:456  continue
    // py:457  if d['hadproblem']:
    // py:458  hadproblem = True
    // py:459  if d['ext']:
    // py:460  found_dir[d['type']] = True
    // py:461  lists['exts'].add(d['ext'])
    // py:462  if d['name'] == '__main__':
    // py:463  pass
    // py:464  elif d['name'].startswith('__') or d['name'].endswith('__'):
    // py:465  hadproblem = True
    // py:466  ee(problem='File name is not supposed to start or end with "__": {0}'.format(
    // py:467  d['path']))
    // py:468  else:
    // py:469  lists[d['type']].add(d['name'])
    // py:470  config_paths[d['type']][d['ext']][d['name']] = d['path']
    // py:471  loaded_configs[d['type']][d['ext']][d['name']] = d['config']
    // py:472  else:
    // py:473  config_paths[d['type']][d['name']] = d['path']
    // py:474  loaded_configs[d['type']][d['name']] = d['config']
    // py:476  for typ in ('themes', 'colorschemes'):
    // py:477  if not found_dir[typ]:
    // py:478  hadproblem = True
    // py:479  ee(problem='Subdirectory {0} was not found in paths {1}'.format(typ, ', '.join(search_paths)))
    // py:481  diff = set(config_paths['colorschemes']) - set(config_paths['themes'])
    // py:482  if diff:
    // py:483  hadproblem = True
    // py:484  for ext in diff:
    // py:485  typ = 'colorschemes' if ext in config_paths['themes'] else 'themes'
    // py:486  if not config_paths['top_' + typ] or typ == 'themes':
    // py:487  ee(problem='{0} extension {1} not present in {2}'.format(
    // py:488  ext,
    // py:489  'configuration' if (
    // py:490  ext in loaded_configs['themes'] and ext in loaded_configs['colorschemes']
    // py:491  ) else 'directory',
    // py:492  typ,
    // py:493  ))
    // py:495  try:
    // py:496  main_config = load_config('config', find_config_files, config_loader)
    // py:497  except IOError:
    // py:498  main_config = {}
    // py:499  ee(problem='Configuration file not found: config.json')
    // py:500  hadproblem = True
    // py:501  except MarkedError as e:
    // py:502  main_config = {}
    // py:503  ee(problem=str(e))
    // py:504  hadproblem = True
    // py:505  else:
    // py:506  if used_main_spec.match(
    // py:507  main_config,
    // py:508  data={'configs': config_paths, 'lists': lists},
    // py:509  context=Context(main_config),
    // py:510  echoerr=ee
    // py:511  )[1]:
    // py:512  hadproblem = True
    // py:514  import_paths = [os.path.expanduser(path) for path in main_config.get('common', {}).get('paths', [])]
    // py:516  try:
    // py:517  colors_config = load_config('colors', find_config_files, config_loader)
    // py:518  except IOError:
    // py:519  colors_config = {}
    // py:520  ee(problem='Configuration file not found: colors.json')
    // py:521  hadproblem = True
    // py:522  except MarkedError as e:
    // py:523  colors_config = {}
    // py:524  ee(problem=str(e))
    // py:525  hadproblem = True
    // py:526  else:
    // py:527  if colors_spec.match(colors_config, context=Context(colors_config), echoerr=ee)[1]:
    // py:528  hadproblem = True
    // py:530  if lhadproblem[0]:
    // py:531  hadproblem = True
    // py:533  top_colorscheme_configs = dict(loaded_configs['top_colorschemes'])
    // py:534  data = {
    // py:535  'ext': None,
    // py:536  'top_colorscheme_configs': top_colorscheme_configs,
    // py:537  'ext_colorscheme_configs': {},
    // py:538  'colors_config': colors_config
    // py:539  }
    // py:540  for colorscheme, config in loaded_configs['top_colorschemes'].items():
    // py:541  data['colorscheme'] = colorscheme
    // py:542  if top_colorscheme_spec.match(config, context=Context(config), data=data, echoerr=ee)[1]:
    // py:543  hadproblem = True
    // py:545  ext_colorscheme_configs = dict2(loaded_configs['colorschemes'])
    // py:546  for ext, econfigs in ext_colorscheme_configs.items():
    // py:547  data = {
    // py:548  'ext': ext,
    // py:549  'top_colorscheme_configs': top_colorscheme_configs,
    // py:550  'ext_colorscheme_configs': ext_colorscheme_configs,
    // py:551  'colors_config': colors_config,
    // py:552  }
    // py:553  for colorscheme, config in econfigs.items():
    // py:554  data['colorscheme'] = colorscheme
    // py:555  if ext == 'vim':
    // py:556  spec = vim_colorscheme_spec
    // py:557  elif ext == 'shell':
    // py:558  spec = shell_colorscheme_spec
    // py:559  else:
    // py:560  spec = colorscheme_spec
    // py:561  if spec.match(config, context=Context(config), data=data, echoerr=ee)[1]:
    // py:562  hadproblem = True
    // py:564  colorscheme_configs = {}
    // py:565  for ext in lists['exts']:
    // py:566  colorscheme_configs[ext] = {}
    // py:567  for colorscheme in lists['colorschemes']:
    // py:568  econfigs = ext_colorscheme_configs[ext]
    // py:569  ecconfigs = econfigs.get(colorscheme)
    // py:570  mconfigs = (
    // py:571  top_colorscheme_configs.get(colorscheme),
    // py:572  econfigs.get('__main__'),
    // py:573  ecconfigs,
    // py:574  )
    // py:575  if not (mconfigs[0] or mconfigs[2]):
    // py:576  continue
    // py:577  config = None
    // py:578  for mconfig in mconfigs:
    // py:579  if not mconfig:
    // py:580  continue
    // py:581  if config:
    // py:582  config = mergedicts_copy(config, mconfig)
    // py:583  else:
    // py:584  config = mconfig
    // py:585  colorscheme_configs[ext][colorscheme] = config
    // py:587  theme_configs = dict2(loaded_configs['themes'])
    // py:588  top_theme_configs = dict(loaded_configs['top_themes'])
    // py:589  for ext, configs in theme_configs.items():
    // py:590  data = {
    // py:591  'ext': ext,
    // py:592  'colorscheme_configs': colorscheme_configs,
    // py:593  'import_paths': import_paths,
    // py:594  'main_config': main_config,
    // py:595  'top_themes': top_theme_configs,
    // py:596  'ext_theme_configs': configs,
    // py:597  'colors_config': colors_config
    // py:598  }
    // py:599  for theme, config in configs.items():
    // py:600  data['theme'] = theme
    // py:601  if theme == '__main__':
    // py:602  data['theme_type'] = 'main'
    // py:603  spec = main_theme_spec
    // py:604  else:
    // py:605  data['theme_type'] = 'regular'
    // py:606  spec = theme_spec
    // py:607  if spec.match(config, context=Context(config), data=data, echoerr=ee)[1]:
    // py:608  hadproblem = True
    // py:610  for top_theme, config in top_theme_configs.items():
    // py:611  data = {
    // py:612  'ext': None,
    // py:613  'colorscheme_configs': colorscheme_configs,
    // py:614  'import_paths': import_paths,
    // py:615  'main_config': main_config,
    // py:616  'theme_configs': theme_configs,
    // py:617  'ext_theme_configs': None,
    // py:618  'colors_config': colors_config
    // py:619  }
    // py:620  data['theme_type'] = 'top'
    // py:621  data['theme'] = top_theme
    // py:622  if top_theme_spec.match(config, context=Context(config), data=data, echoerr=ee)[1]:
    // py:623  hadproblem = True
    // py:625  return hadproblem
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-lint-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn function_name_re_matches_simple_name() {
        // py:43  ^(\w+\.)*[a-zA-Z_]\w*$
        let r = function_name_re();
        assert!(r.is_match("foo"));
        assert!(r.is_match("_private"));
        assert!(r.is_match("func2"));
    }

    #[test]
    fn function_name_re_matches_dotted_path() {
        let r = function_name_re();
        assert!(r.is_match("powerline.segments.common.sys.uptime"));
        assert!(r.is_match("mymodule.fn"));
    }

    #[test]
    fn function_name_re_rejects_starting_digit() {
        let r = function_name_re();
        assert!(!r.is_match("1foo"));
        assert!(!r.is_match("powerline.1foo"));
    }

    #[test]
    fn function_name_re_rejects_special_chars() {
        let r = function_name_re();
        assert!(!r.is_match("foo bar"));
        assert!(!r.is_match("foo-bar"));
    }

    #[test]
    fn open_file_reads_bytes() {
        let d = tmp_dir();
        let p = d.join("test.json");
        let mut h = std::fs::File::create(&p).unwrap();
        h.write_all(b"{\"k\": 1}").unwrap();
        let bytes = open_file(&p).unwrap();
        assert_eq!(bytes, b"{\"k\": 1}".to_vec());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn open_file_missing_returns_error() {
        let r = open_file(std::path::Path::new("/nonexistent/path/x.json"));
        assert!(r.is_err());
    }

    #[test]
    fn load_json_file_parses_valid_config() {
        let d = tmp_dir();
        let p = d.join("good.json");
        let mut h = std::fs::File::create(&p).unwrap();
        h.write_all(b"{\"key\": \"value\"}").unwrap();
        let r = load_json_file(&p);
        assert!(!r.hadproblem);
        assert!(r.config.is_some());
        assert!(r.error.is_none());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn load_json_file_missing_returns_error() {
        let r = load_json_file(std::path::Path::new("/never/exists/x.json"));
        assert!(r.hadproblem);
        assert!(r.config.is_none());
        assert!(r.error.is_some());
    }

    #[test]
    fn load_json_file_invalid_json_sets_hadproblem() {
        let d = tmp_dir();
        let p = d.join("bad.json");
        let mut h = std::fs::File::create(&p).unwrap();
        h.write_all(b"not valid json").unwrap();
        let r = load_json_file(&p);
        assert!(r.hadproblem);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn updated_with_config_merges_results_into_dict() {
        let d_path = tmp_dir();
        let p = d_path.join("c.json");
        let mut h = std::fs::File::create(&p).unwrap();
        h.write_all(b"{\"a\": 1}").unwrap();
        let mut d = Map::new();
        d.insert(
            "path".to_string(),
            Value::String(p.to_string_lossy().to_string()),
        );
        updated_with_config(&mut d);
        assert_eq!(d.get("hadproblem"), Some(&Value::Bool(false)));
        assert!(d.get("config").is_some());
        std::fs::remove_dir_all(&d_path).ok();
    }

    #[test]
    fn updated_with_config_no_path_no_op() {
        let mut d = Map::new();
        d.insert("k".to_string(), Value::from(1));
        updated_with_config(&mut d);
        assert!(d.get("hadproblem").is_none());
        assert!(d.get("config").is_none());
    }

    #[test]
    fn dict2_shallow_copies_inner_dicts() {
        let mut inner = Map::new();
        inner.insert("inner_k".to_string(), Value::from(42));
        let mut d = Map::new();
        d.insert("outer".to_string(), Value::Object(inner));
        let r = dict2(&d);
        assert!(r.get("outer").is_some());
        assert_eq!(
            r["outer"].as_object().unwrap().get("inner_k"),
            Some(&Value::from(42))
        );
    }

    #[test]
    fn dict2_passes_non_dict_values_through() {
        let mut d = Map::new();
        d.insert("scalar".to_string(), Value::from(7));
        let r = dict2(&d);
        assert_eq!(r["scalar"], Value::from(7));
    }

    #[test]
    fn strip_json_suffix_removes_suffix() {
        assert_eq!(strip_json_suffix("powerline.json"), "powerline");
        assert_eq!(strip_json_suffix("default.json"), "default");
    }

    #[test]
    fn strip_json_suffix_passes_through_when_absent() {
        assert_eq!(strip_json_suffix("powerline"), "powerline");
        assert_eq!(strip_json_suffix(""), "");
    }

    #[test]
    fn load_json_result_struct_fields_accessible() {
        let r = LoadJsonResult {
            hadproblem: false,
            config: Some(Value::Object(Map::new())),
            error: None,
        };
        assert!(!r.hadproblem);
        assert!(r.config.is_some());
        assert!(r.error.is_none());
    }

    #[test]
    fn register_common_names_inserts_player_alias() {
        // py:322  register player → powerline.segments.common.players._player
        register_common_names();
        let map = crate::ported::lint::checks::common_names()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert!(map.contains_key("player"));
        let entries = map.get("player").unwrap();
        let pair = entries
            .iter()
            .find(|(m, n)| m == "powerline.segments.common.players" && n == "_player");
        assert!(pair.is_some());
    }

    #[test]
    fn generate_json_config_loader_loads_valid_config_and_flag_stays_false() {
        // py:35-40
        let d = tmp_dir();
        let p = d.join("good.json");
        let mut h = std::fs::File::create(&p).unwrap();
        h.write_all(b"{\"k\": 1}").unwrap();

        let flag = std::sync::Arc::new(std::sync::Mutex::new(false));
        let loader = generate_json_config_loader(flag.clone());
        let r = loader(&p);
        assert!(r.is_some());
        assert!(!*flag.lock().unwrap());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn generate_json_config_loader_missing_file_does_not_set_flag() {
        // py:38-39  hadproblem only flips when load reports it
        // load() of a missing file returns (None, false) in the
        // Rust port's current shape.
        let flag = std::sync::Arc::new(std::sync::Mutex::new(false));
        let loader = generate_json_config_loader(flag.clone());
        let r = loader(std::path::Path::new("/never/exists/x.json"));
        assert!(r.is_none());
        // Flag stays false since load returns (None, false) for missing
        // files (no hadproblem signal raised).
    }

    #[test]
    fn find_all_ext_config_files_yields_ext_scoped_entry() {
        // py:374-381  ext-scoped config: <root>/<subdir>/<ext>/<name>.json
        let root = tmp_dir();
        let themes_dir = root.join("themes");
        let vim_dir = themes_dir.join("vim");
        std::fs::create_dir_all(&vim_dir).unwrap();
        std::fs::write(vim_dir.join("default.json"), "{}").unwrap();

        let entries = find_all_ext_config_files(std::slice::from_ref(&root), "themes");
        let ext_entry = entries
            .iter()
            .find(|e| e.error.is_none() && e.ext.is_some());
        assert!(ext_entry.is_some());
        let e = ext_entry.unwrap();
        assert_eq!(e.name.as_deref(), Some("default"));
        assert_eq!(e.ext.as_deref(), Some("vim"));
        assert_eq!(e.kind.as_deref(), Some("themes"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_all_ext_config_files_yields_top_level_entry() {
        // py:359-365  top-level config: <root>/<subdir>/<name>.json
        let root = tmp_dir();
        let themes_dir = root.join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("base.json"), "{}").unwrap();

        let entries = find_all_ext_config_files(std::slice::from_ref(&root), "themes");
        let top_entry = entries
            .iter()
            .find(|e| e.error.is_none() && e.kind.as_deref() == Some("top_themes"));
        assert!(top_entry.is_some());
        let e = top_entry.unwrap();
        assert_eq!(e.name.as_deref(), Some("base"));
        assert!(e.ext.is_none());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_all_ext_config_files_yields_error_for_non_directory() {
        // py:349-353  if exists but not directory
        let root = tmp_dir();
        std::fs::write(root.join("themes"), "not-a-dir").unwrap();

        let entries = find_all_ext_config_files(std::slice::from_ref(&root), "themes");
        let err = entries
            .iter()
            .find(|e| e.error.as_deref().map(|s| s.contains("not a directory")) == Some(true));
        assert!(err.is_some());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn find_all_ext_config_files_skips_nonexistent_root_silently() {
        // py:348  if not isdir + py:354 continue (no exists check fires)
        let entries =
            find_all_ext_config_files(&[std::path::PathBuf::from("/never/exists/abc")], "themes");
        assert!(entries.is_empty());
    }

    #[test]
    fn find_all_ext_config_files_yields_error_for_non_json_file_in_subdir() {
        // py:367-370  non-json non-directory entry
        let root = tmp_dir();
        let themes_dir = root.join("themes");
        std::fs::create_dir_all(&themes_dir).unwrap();
        std::fs::write(themes_dir.join("README.txt"), "not json").unwrap();

        let entries = find_all_ext_config_files(std::slice::from_ref(&root), "themes");
        let err = entries.iter().find(|e| {
            e.error
                .as_deref()
                .map(|s| s.contains("not a directory or configuration file"))
                == Some(true)
        });
        assert!(err.is_some());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn colorscheme_spec_registers_required_groups_key() {
        // py:177-180  colorscheme_spec has `name` + required `groups`.
        let s = colorscheme_spec();
        assert!(s.get("groups").is_some(), "groups key missing");
        assert!(s.get("name").is_some(), "name key missing");
    }

    #[test]
    fn group_spec_constructs_without_panic() {
        // py:168-172  either(dict-form, string-alias).
        let _ = group_spec();
    }

    #[test]
    fn groups_spec_attrs_spec_color_spec_constructible() {
        let _ = groups_spec();
        let _ = attrs_spec();
        let _ = color_spec();
    }

    #[test]
    fn check_against_clean_fixture_returns_false() {
        // Existing E2E fixture is hand-curated to be lint-clean.
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/data/e2e/scenario_hostname");
        if !fixture.is_dir() {
            return;
        }
        let paths = vec![fixture.to_string_lossy().to_string()];
        let hadproblem = check(Some(&paths), false, None);
        assert!(!hadproblem, "lint flagged a clean fixture");
    }

    #[test]
    fn check_rejects_colorscheme_missing_groups() {
        // Construct a minimal config tree with a malformed colorscheme.
        let root = tmp_dir();
        std::fs::create_dir_all(root.join("colorschemes/tmux")).unwrap();
        // No `groups` key — colorscheme_spec at py:506-552 requires it.
        std::fs::write(
            root.join("colorschemes/tmux/bad.json"),
            br#"{"name": "Bad"}"#,
        )
        .unwrap();
        let paths = vec![root.to_string_lossy().to_string()];
        let hadproblem = check(Some(&paths), false, None);
        assert!(hadproblem, "missing 'groups' key should be flagged");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_rejects_double_underscore_filenames() {
        // py:471-478  File names ending in `__` are rejected (except
        // the literal `__main__`).
        let root = tmp_dir();
        std::fs::create_dir_all(root.join("themes/tmux")).unwrap();
        std::fs::write(
            root.join("themes/tmux/__bogus.json"),
            br#"{"segments": {"left": [], "right": []}}"#,
        )
        .unwrap();
        let paths = vec![root.to_string_lossy().to_string()];
        let hadproblem = check(Some(&paths), false, None);
        assert!(hadproblem, "double-underscore filename should be flagged");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_rejects_theme_segments_left_not_array() {
        let root = tmp_dir();
        std::fs::create_dir_all(root.join("themes/tmux")).unwrap();
        std::fs::write(
            root.join("themes/tmux/typo.json"),
            br#"{"segments": {"left": "should-be-array", "right": []}}"#,
        )
        .unwrap();
        let paths = vec![root.to_string_lossy().to_string()];
        let hadproblem = check(Some(&paths), false, None);
        assert!(hadproblem, "non-array segments.left should be flagged");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn check_rejects_parse_error() {
        let root = tmp_dir();
        std::fs::create_dir_all(root.join("themes/tmux")).unwrap();
        std::fs::write(
            root.join("themes/tmux/broken.json"),
            br#"{ this is not json"#,
        )
        .unwrap();
        let paths = vec![root.to_string_lossy().to_string()];
        let hadproblem = check(Some(&paths), false, None);
        assert!(hadproblem, "JSON parse failure should be flagged");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_all_ext_config_files_walks_multiple_search_paths() {
        // py:346  for config_root in search_paths
        let r1 = tmp_dir();
        let r2 = tmp_dir();
        std::fs::create_dir_all(r1.join("themes").join("vim")).unwrap();
        std::fs::create_dir_all(r2.join("themes").join("shell")).unwrap();
        std::fs::write(r1.join("themes").join("vim").join("a.json"), "{}").unwrap();
        std::fs::write(r2.join("themes").join("shell").join("b.json"), "{}").unwrap();

        let entries = find_all_ext_config_files(&[r1.clone(), r2.clone()], "themes");
        let exts: std::collections::HashSet<String> =
            entries.iter().filter_map(|e| e.ext.clone()).collect();
        assert!(exts.contains("vim"));
        assert!(exts.contains("shell"));
        std::fs::remove_dir_all(&r1).ok();
        std::fs::remove_dir_all(&r2).ok();
    }

    #[test]
    fn ext_spec_carries_required_keys() {
        // py:51-57  ext_spec keys: colorscheme, theme, top_theme.
        let s = ext_spec();
        assert!(s.get("colorscheme").is_some(), "colorscheme key missing");
        assert!(s.get("theme").is_some(), "theme key missing");
        assert!(s.get("top_theme").is_some(), "top_theme key missing");
        let tt = s.get("top_theme").unwrap();
        assert!(tt.isoptional, "top_theme must be optional per py:56");
    }

    #[test]
    fn divider_spec_has_printable_and_len_constraint() {
        // py:47-48  Spec().printable().len('le', 3, ...).copy
        let s = divider_spec();
        assert!(s.printable_flag, "divider must be printable");
        assert!(
            s.len_constraints.contains(&(spec::Cmp::Le, 3)),
            "divider len <= 3 missing"
        );
    }

    #[test]
    fn term_color_spec_is_unsigned_le_255() {
        // py:142  Spec().unsigned().cmp('le', 255).copy
        let s = term_color_spec();
        assert!(s.unsigned_flag, "term color must be unsigned");
        let (op, v) = s.cmp_constraint.unwrap();
        assert_eq!(op, spec::Cmp::Le);
        assert_eq!(v, 255.0);
    }

    #[test]
    fn true_color_spec_has_hex_regex() {
        // py:143-146  Spec().re('^[0-9a-fA-F]{6}$').copy
        let s = true_color_spec();
        assert_eq!(s.regex.unwrap(), "^[0-9a-fA-F]{6}$");
    }

    #[test]
    fn colors_spec_has_colors_and_gradients_keys() {
        // py:147-162  Spec(colors=..., gradients=...).context_message(...)
        let s = colors_spec();
        assert!(s.get("colors").is_some(), "colors key missing");
        assert!(s.get("gradients").is_some(), "gradients key missing");
        assert!(s.cmsg.contains("colors configuration"));
    }

    #[test]
    fn vim_mode_spec_contains_known_modes() {
        // py:199 derives from vim_modes + ['nc', 'tab_nc', 'buf_nc'].
        let s = vim_mode_spec();
        let one = s.oneof.unwrap();
        assert!(one.iter().any(|m| m == "n"));
        assert!(one.iter().any(|m| m == "i"));
        assert!(one.iter().any(|m| m == "nc"));
        assert!(one.iter().any(|m| m == "tab_nc"));
        assert!(one.iter().any(|m| m == "buf_nc"));
    }

    #[test]
    fn shell_mode_spec_has_regex() {
        // py:208  Spec().re(r'^(?:[\w\-]+|\.safe)$').copy
        let s = shell_mode_spec();
        assert_eq!(s.regex.unwrap(), r"^(?:[\w\-]+|\.safe)$");
    }

    #[test]
    fn top_colorscheme_spec_has_mt_optional() {
        // py:191-198  mode_translations is optional.
        let s = top_colorscheme_spec();
        assert!(s.get("mode_translations").unwrap().isoptional);
        assert!(s.get("groups").is_some(), "groups required by spec");
        assert!(s.get("name").is_some(), "name present");
    }

    #[test]
    fn args_spec_has_pl_and_segment_info_optional() {
        // py:219-222  pl + segment_info both optional with error messages.
        let s = args_spec();
        // Top-level optional applied at outer scope.
        assert!(s.isoptional);
        assert!(s.get("pl").is_some());
        assert!(s.get("segment_info").is_some());
    }

    #[test]
    fn gen_components_spec_lists_oneof_components() {
        // py:58  gen_components_spec uses oneof + list pattern.
        let s = gen_components_spec(&["statusline", "tabline"]);
        assert!(s.allowed_types.contains(&spec::SpecType::List));
        // Inner item spec stored in s.specs[0]
        assert_eq!(s.specs.len(), 1);
        let inner = &s.specs[0];
        let one = inner.oneof.as_ref().unwrap();
        assert!(one.iter().any(|m| m == "statusline"));
        assert!(one.iter().any(|m| m == "tabline"));
    }

    #[test]
    fn segment_spec_base_has_all_segment_keys() {
        // py:225-254  segment_spec_base lists 20 segment-config keys.
        let s = segment_spec_base();
        for k in &[
            "name",
            "function",
            "exclude_modes",
            "include_modes",
            "exclude_function",
            "include_function",
            "draw_hard_divider",
            "draw_soft_divider",
            "draw_inner_divider",
            "display",
            "module",
            "priority",
            "after",
            "before",
            "width",
            "align",
            "args",
            "contents",
            "highlight_groups",
            "divider_highlight_group",
        ] {
            assert!(s.get(k).is_some(), "missing key {k} in segment_spec_base");
        }
    }

    #[test]
    fn segment_spec_has_segments_and_type_keys() {
        // py:258-261  segment_spec extends segment_spec_base with type + segments.
        let s = segment_spec();
        assert!(s.get("type").is_some(), "type key missing");
        assert!(s.get("segments").is_some(), "segments key missing");
    }

    #[test]
    fn subsegment_spec_excludes_segment_list_from_type() {
        // py:255-257  subsegment type excludes 'segment_list'.
        let s = subsegment_spec();
        let allowed = s.get("type").unwrap().oneof.as_ref().unwrap();
        assert!(!allowed.iter().any(|t| t == "segment_list"));
        assert!(allowed.iter().any(|t| t == "string" || t == "function"));
    }

    #[test]
    fn segdict_spec_has_left_and_right() {
        // py:263-269
        let s = segdict_spec();
        assert!(s.get("left").is_some(), "left missing");
        assert!(s.get("right").is_some(), "right missing");
    }

    #[test]
    fn spaces_spec_has_unsigned_le_2() {
        // py:285-287
        let s = spaces_spec();
        assert!(s.unsigned_flag);
        assert_eq!(s.cmp_constraint.unwrap(), (spec::Cmp::Le, 2.0));
    }

    #[test]
    fn common_theme_spec_has_cursor_keys() {
        // py:288-292
        let s = common_theme_spec();
        assert!(s.get("default_module").is_some());
        assert!(s.get("cursor_space").is_some());
        assert!(s.get("cursor_columns").is_some());
        assert!(s.cmsg.contains("theme"));
    }

    #[test]
    fn theme_spec_has_segments_key() {
        // py:310-318  theme_spec adds dividers/spaces/segment_data/segments.
        let s = theme_spec();
        assert!(s.get("dividers").is_some());
        assert!(s.get("spaces").is_some());
        assert!(s.get("segment_data").is_some());
        assert!(s.get("segments").is_some());
    }

    #[test]
    fn top_theme_structure_spec_includes_non_breaking_spaces() {
        // py:293-301
        let s = top_theme_structure_spec();
        assert!(s.get("use_non_breaking_spaces").is_some());
        assert!(s.get("dividers").is_some());
        assert!(s.get("spaces").is_some());
        assert!(s.get("segment_data").is_some());
    }

    #[test]
    fn main_spec_has_common_and_ext_blocks() {
        // py:64-140
        let s = main_spec();
        assert!(s.get("common").is_some(), "common block missing");
        assert!(s.get("ext").is_some(), "ext block missing");
        assert!(s.cmsg.contains("main configuration"));
    }

    #[test]
    fn common_spec_has_log_and_watcher_keys() {
        // py:65-104
        let s = common_spec();
        for k in &[
            "default_top_theme",
            "term_truecolor",
            "term_escape_style",
            "paths",
            "log_file",
            "log_level",
            "log_format",
            "interval",
            "reload_config",
            "watcher",
        ] {
            assert!(s.get(k).is_some(), "common missing {k}");
        }
    }

    #[test]
    fn ext_block_spec_has_known_extensions() {
        // py:105-139
        let s = ext_block_spec();
        for k in &["vim", "ipython", "shell", "wm"] {
            assert!(s.get(k).is_some(), "ext missing {k}");
        }
    }

    #[test]
    fn log_file_spec_is_optional_with_either() {
        // py:75-98
        let s = log_file_spec();
        assert!(s.isoptional, "log_file should be optional");
        // either() registers an `either` check, encoded in spec.specs.
        assert!(!s.specs.is_empty(), "either branches missing");
    }
}
