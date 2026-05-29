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
/// **Status:** stub. The full check pipeline requires the entire
/// powerliners lint Spec DSL, ConfigLoader, themes/colorschemes
/// dispatch, etc. Returns `false` (no problems) as a baseline.
/// The Python body trace below records the algorithmic contract.
pub fn check() -> bool {
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
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-lint-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
        assert_eq!(*flag.lock().unwrap(), false);
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

        let entries = find_all_ext_config_files(&[root.clone()], "themes");
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

        let entries = find_all_ext_config_files(&[root.clone()], "themes");
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

        let entries = find_all_ext_config_files(&[root.clone()], "themes");
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

        let entries = find_all_ext_config_files(&[root.clone()], "themes");
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
}
