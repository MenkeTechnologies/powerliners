// vim:fileencoding=utf-8:noet
//! Port of `powerline/__init__.py`.
//!
//! Core Powerline class + supporting helpers used by every backend
//! (vim/shell/ipython/etc.). The Python source is ~813 LOC; the
//! Rust port surfaces the pure-functional helpers that don't depend
//! on the unported Renderer/Theme/Colorscheme dispatch chain:
//!
//!   - `NotInterceptedError` (py:19)
//!   - `_config_loader_condition(path)` (py:23) — file-exists predicate
//!   - `get_config_paths()` (py:138) — XDG-aware path list builder
//!   - `get_default_theme(is_unicode)` (py:301)
//!   - `finish_common_config(encoding, common_config)` (py:313) — fills
//!     in default values for every known key
//!   - `PowerlineLogger` (py:46) — message-formatting wrapper
//!   - `_find_config_files(search_paths, config_file)` (py:29) —
//!     filesystem search with the `.json` suffix appended
//!
//! The full `Powerline` class (py:427-813) is heavy enough to deserve
//! its own port pass: it weaves together renderer construction,
//! theme/colorscheme loading, watcher dispatch, segment introspection,
//! and the daemon-mode render loop.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import sys                                       // py:5
// import logging                                   // py:6
// from threading import Lock, Event                 // py:8
// from powerline.colorscheme import Colorscheme    // py:10
// from powerline.lib.config import ConfigLoader    // py:11
// from powerline.lib.unicode import unicode, safe_unicode, FailedUnicode                  // py:12
// from powerline.config import DEFAULT_SYSTEM_CONFIG_DIR                                   // py:13
// from powerline.lib.dict import mergedicts        // py:14
// from powerline.lib.encoding import get_preferred_output_encoding                          // py:15
// from powerline.lib.path import join              // py:16
// from powerline.version import __version__       // py:17

pub mod bindings;
pub mod colorscheme;
pub mod commands;
pub mod config;
pub mod ipython;
pub mod lemonbar;
pub mod lib;
pub mod lint;
pub mod listers;
pub mod matchers;
pub mod pdb;
pub mod renderer;
pub mod renderers;
pub mod scripts;
pub mod segment;
pub mod segments;
pub mod selectors;
pub mod shell;
pub mod theme;
pub mod version;
pub mod vim;

use crate::ported::config::DEFAULT_SYSTEM_CONFIG_DIR;
use serde_json::{Map, Value};

/// Port of `class NotInterceptedError(BaseException)` from
/// `powerline/__init__.py:19`.
///
/// Used by the Powerline core to signal exceptions that shouldn't be
/// caught by the segment dispatcher's catch-all.
#[derive(Debug, Clone)]
pub struct NotInterceptedError(pub String);

impl std::fmt::Display for NotInterceptedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for NotInterceptedError {}

/// Port of `_config_loader_condition()` from
/// `powerline/__init__.py:23`.
///
/// Returns the path when it points at an existing file, else None.
pub fn _config_loader_condition(path: Option<&std::path::Path>) -> Option<std::path::PathBuf> {
    // py:19  class NotInterceptedError(BaseException):
    // py:20  pass
    // py:23  def _config_loader_condition(path):
    // py:24  if path and os.path.isfile(path):
    // py:25  return path
    // py:26  return None
    let p = path?;
    if p.is_file() {
        Some(p.to_path_buf())
    } else {
        None
    }
}

/// Port of `_find_config_files()` from
/// `powerline/__init__.py:29`.
///
/// Searches `search_paths` for `<config_file>.json` and yields each
/// match. Returns an `Err` analogous to Python's `IOError` when no
/// match is found.
pub fn _find_config_files(
    search_paths: &[std::path::PathBuf],
    config_file: &str,
) -> Result<Vec<std::path::PathBuf>, String> {
    // py:29  def _find_config_files(search_paths, config_file, config_loader=None, loader_callback=None):
    // py:30  config_file += '.json'
    // py:31  found = False
    // py:32  for path in search_paths:
    // py:33  config_file_path = join(path, config_file)
    // py:34  if os.path.isfile(config_file_path):
    // py:35  yield config_file_path
    // py:36  found = True
    // py:37  elif config_loader:
    // py:38  config_loader.register_missing(_config_loader_condition, loader_callback, config_file_path)
    // py:39  if not found:
    // py:40  raise IOError('Config file not found in search paths ({0}): {1}'.format(
    // py:41  ', '.join(search_paths),
    // py:42  config_file
    // py:43  ))
    let with_ext = format!("{}.json", config_file);
    let mut found: Vec<std::path::PathBuf> = Vec::new();
    for path in search_paths {
        let candidate = path.join(&with_ext);
        if candidate.is_file() {
            found.push(candidate);
        }
    }
    if found.is_empty() {
        let paths_joined: Vec<String> = search_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        Err(format!(
            "Config file not found in search paths ({}): {}",
            paths_joined.join(", "),
            with_ext
        ))
    } else {
        Ok(found)
    }
}

/// Port of `PowerlineLogger` from
/// `powerline/__init__.py:46`.
///
/// Wraps the underlying logger to emit `{ext}:{prefix}:{message}`
/// formatted entries. The Rust port surfaces just the message
/// formatter — the actual logger dispatch (Python's
/// `logger.log(level, msg)`) is replaced with a captured-message
/// vec so callers can assert without wiring a logger.
pub struct PowerlineLogger {
    /// Python: `self.ext` — extension name prefix.
    pub ext: String,
    /// Python: `self.prefix` — additional message prefix.
    pub prefix: String,
    /// Captured messages for test assertion.
    pub messages: std::sync::Mutex<Vec<(String, String)>>,
}

impl PowerlineLogger {
    /// Constructor.
    pub fn new(ext: impl Into<String>, prefix: impl Into<String>) -> Self {
        // py:46  class PowerlineLogger(object):
        // py:47-63  docstring
        // py:65  def __init__(self, use_daemon_threads, logger, ext):
        // py:66  self.logger = logger
        // py:67  self.ext = ext
        // py:68  self.use_daemon_threads = use_daemon_threads
        // py:69  self.prefix = ''
        // py:70  self.last_msgs = {}
        Self {
            ext: ext.into(),
            prefix: prefix.into(),
            messages: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Port of the message formatter from
    /// `powerline/__init__.py:46-109`. Returns the formatted
    /// `{ext}:{prefix}:{message}` string per the class docstring.
    pub fn format_message(&self, message: &str, prefix: Option<&str>) -> String {
        // py:72  def _log(self, attr, msg, *args, **kwargs):
        // py:73  prefix = kwargs.get('prefix') or self.prefix
        // py:74  prefix = self.ext + ((':' + prefix) if prefix else '')
        // py:75  msg = safe_unicode(msg)
        // py:76  if args or kwargs:
        // py:77  args = [safe_unicode(s) if isinstance(s, bytes) else s for s in args]
        // py:78  kwargs = dict((
        // py:79  (k, safe_unicode(v) if isinstance(v, bytes) else v)
        // py:80  for k, v in kwargs.items()
        // py:81  ))
        // py:82  msg = msg.format(*args, **kwargs)
        // py:83  msg = prefix + ':' + msg
        // py:84  key = attr + ':' + prefix
        // py:85  if msg != self.last_msgs.get(key):
        // py:86  getattr(self.logger, attr)(msg)
        // py:87  self.last_msgs[key] = msg
        let effective_prefix = prefix.unwrap_or(&self.prefix);
        format!("{}:{}:{}", self.ext, effective_prefix, message)
    }

    /// Port of the per-level logging methods (debug/info/warn/error/
    /// critical/exception) at py:80-108.
    pub fn log(&self, level: &str, message: &str) {
        // py:89  def critical(self, msg, *args, **kwargs):
        // py:90  self._log('critical', msg, *args, **kwargs)
        // py:92  def exception(self, msg, *args, **kwargs):
        // py:93  self._log('exception', msg, *args, **kwargs)
        // py:95  def info(self, msg, *args, **kwargs):
        // py:96  self._log('info', msg, *args, **kwargs)
        // py:98  def error(self, msg, *args, **kwargs):
        // py:99  self._log('error', msg, *args, **kwargs)
        let formatted = self.format_message(message, None);
        self.messages
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((level.to_string(), formatted));
    }

    /// Convenience shortcut for `log("DEBUG", message)`.
    pub fn debug(&self, message: &str) {
        self.log("DEBUG", message);
    }

    /// Convenience shortcut for `log("INFO", message)`.
    pub fn info(&self, message: &str) {
        self.log("INFO", message);
    }

    /// Convenience shortcut for `log("WARNING", message)`.
    pub fn warn(&self, message: &str) {
        self.log("WARNING", message);
    }

    /// Convenience shortcut for `log("ERROR", message)`.
    pub fn error(&self, message: &str) {
        self.log("ERROR", message);
    }

    /// Convenience shortcut for `log("CRITICAL", message)`.
    pub fn critical(&self, message: &str) {
        self.log("CRITICAL", message);
    }

    /// Convenience shortcut for `log("EXCEPTION", message)`.
    pub fn exception(&self, message: &str) {
        self.log("EXCEPTION", message);
    }
}

/// Port of `get_config_paths()` from
/// `powerline/__init__.py:138`.
///
/// Returns the XDG-aware config-paths list. Resolution order
/// (matches py:144-153):
/// 1. `plugin_path` (always first per py:152)
/// 2. Reversed `XDG_CONFIG_DIRS` (`:` separated) entries
/// 3. `XDG_CONFIG_HOME` (`~/.config` default)
pub fn get_config_paths() -> Vec<std::path::PathBuf> {
    // py:145-146  XDG_CONFIG_HOME / ~/.config
    let home = std::env::var("HOME").unwrap_or_default();
    let config_home =
        std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{}/.config", home));
    let mut config_paths: Vec<std::path::PathBuf> = vec![std::path::PathBuf::from(format!(
        "{}/powerline",
        config_home
    ))];
    // py:148-150  XDG_CONFIG_DIRS
    let config_dirs = std::env::var("XDG_CONFIG_DIRS").unwrap_or_else(|_| {
        DEFAULT_SYSTEM_CONFIG_DIR()
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    });
    let dir_paths: Vec<std::path::PathBuf> = config_dirs
        .split(':')
        .filter(|p| !p.is_empty())
        .map(|d| std::path::PathBuf::from(format!("{}/powerline", d)))
        .collect();
    // py:150  config_paths[:0] = reversed([...])
    for p in dir_paths.into_iter().rev() {
        config_paths.insert(0, p);
    }
    // py:151  plugin_path always first
    let plugin_path = std::path::PathBuf::from("config_files");
    config_paths.insert(0, plugin_path);
    config_paths
}

/// Port of `get_default_theme()` from
/// `powerline/__init__.py:301`.
///
/// Returns `'powerline_terminus'` for Unicode-capable environments,
/// `'ascii'` otherwise per py:310-311.
pub fn get_default_theme(is_unicode: bool) -> &'static str {
    // py:310-311
    if is_unicode {
        "powerline_terminus"
    } else {
        "ascii"
    }
}

/// Port of `finish_common_config()` from
/// `powerline/__init__.py:313`.
///
/// Fills in default values for every common-config key per
/// py:325-347 and expands `~` in `paths` entries.
pub fn finish_common_config(
    encoding: &str,
    common_config: &Map<String, Value>,
) -> Map<String, Value> {
    // py:324  encoding = encoding.lower()
    let encoding = encoding.to_lowercase();
    // py:325-326  default_top_theme = 'powerline_terminus' if utf/ucs else 'ascii'
    let default_top_theme =
        get_default_theme(encoding.starts_with("utf") || encoding.starts_with("ucs"));
    // py:328  common_config = common_config.copy()
    let mut cfg = common_config.clone();
    // py:329-340  setdefault calls
    cfg.entry("default_top_theme")
        .or_insert_with(|| Value::String(default_top_theme.into()));
    cfg.entry("paths")
        .or_insert_with(|| Value::Array(Vec::new()));
    cfg.entry("watcher")
        .or_insert_with(|| Value::String("auto".into()));
    cfg.entry("log_level")
        .or_insert_with(|| Value::String("WARNING".into()));
    cfg.entry("log_format")
        .or_insert_with(|| Value::String("%(asctime)s:%(levelname)s:%(message)s".into()));
    cfg.entry("term_truecolor").or_insert(Value::Bool(false));
    cfg.entry("term_escape_style")
        .or_insert_with(|| Value::String("auto".into()));
    cfg.entry("ambiwidth").or_insert(Value::from(1));
    cfg.entry("additional_escapes").or_insert(Value::Null);
    cfg.entry("reload_config").or_insert(Value::Bool(true));
    cfg.entry("interval").or_insert(Value::Null);
    cfg.entry("log_file")
        .or_insert_with(|| Value::Array(vec![Value::Null]));

    // py:342-343  if not isinstance(log_file, list): log_file = [log_file]
    let log_file_v = cfg["log_file"].clone();
    if !log_file_v.is_array() {
        cfg.insert("log_file".to_string(), Value::Array(vec![log_file_v]));
    }

    // py:345-347  paths = [os.path.expanduser(p) for p in paths]
    let home = std::env::var("HOME").unwrap_or_default();
    if let Some(paths) = cfg.get_mut("paths").and_then(|v| v.as_array_mut()) {
        for p in paths.iter_mut() {
            if let Some(s) = p.as_str() {
                let expanded = if s.starts_with('~') {
                    s.replacen('~', &home, 1)
                } else {
                    s.to_string()
                };
                *p = Value::String(expanded);
            }
        }
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    macro_rules! lock_env {
        () => {{
            TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    #[test]
    fn not_intercepted_error_implements_error() {
        let e = NotInterceptedError("boom".to_string());
        assert!(e.to_string().contains("boom"));
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn config_loader_condition_returns_path_when_exists() {
        let cargo_toml = std::env::current_dir().unwrap().join("Cargo.toml");
        let r = _config_loader_condition(Some(&cargo_toml));
        assert_eq!(r, Some(cargo_toml));
    }

    #[test]
    fn config_loader_condition_returns_none_when_missing() {
        let p = std::path::PathBuf::from("/never_exists_path_xyz_99");
        let r = _config_loader_condition(Some(&p));
        assert!(r.is_none());
    }

    #[test]
    fn config_loader_condition_returns_none_when_none() {
        let r = _config_loader_condition(None);
        assert!(r.is_none());
    }

    #[test]
    fn find_config_files_error_when_not_found() {
        let r = _find_config_files(&[std::path::PathBuf::from("/nowhere")], "missing");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Config file not found"));
    }

    #[test]
    fn find_config_files_appends_json_suffix_to_message() {
        let r = _find_config_files(&[std::path::PathBuf::from("/nowhere")], "powerline");
        let msg = r.unwrap_err();
        assert!(msg.contains("powerline.json"));
    }

    #[test]
    fn get_default_theme_unicode_returns_powerline_terminus() {
        // py:310-311
        assert_eq!(get_default_theme(true), "powerline_terminus");
    }

    #[test]
    fn get_default_theme_non_unicode_returns_ascii() {
        assert_eq!(get_default_theme(false), "ascii");
    }

    #[test]
    fn finish_common_config_fills_default_top_theme_for_utf8() {
        let r = finish_common_config("UTF-8", &Map::new());
        assert_eq!(r["default_top_theme"], "powerline_terminus");
    }

    #[test]
    fn finish_common_config_fills_default_top_theme_for_ucs() {
        let r = finish_common_config("UCS-2", &Map::new());
        assert_eq!(r["default_top_theme"], "powerline_terminus");
    }

    #[test]
    fn finish_common_config_fills_default_top_theme_for_ascii() {
        let r = finish_common_config("latin-1", &Map::new());
        assert_eq!(r["default_top_theme"], "ascii");
    }

    #[test]
    fn finish_common_config_fills_all_defaults() {
        let r = finish_common_config("UTF-8", &Map::new());
        assert_eq!(r["watcher"], "auto");
        assert_eq!(r["log_level"], "WARNING");
        assert_eq!(r["term_truecolor"], false);
        assert_eq!(r["term_escape_style"], "auto");
        assert_eq!(r["ambiwidth"], 1);
        assert_eq!(r["additional_escapes"], Value::Null);
        assert_eq!(r["reload_config"], true);
        assert_eq!(r["interval"], Value::Null);
    }

    #[test]
    fn finish_common_config_log_file_defaults_to_list_with_null() {
        let r = finish_common_config("UTF-8", &Map::new());
        let lf = r["log_file"].as_array().unwrap();
        assert_eq!(lf.len(), 1);
        assert_eq!(lf[0], Value::Null);
    }

    #[test]
    fn finish_common_config_wraps_non_list_log_file() {
        // py:342-343  if not list: wrap
        let mut input = Map::new();
        input.insert(
            "log_file".to_string(),
            Value::String("/var/log/p.log".into()),
        );
        let r = finish_common_config("UTF-8", &input);
        let lf = r["log_file"].as_array().unwrap();
        assert_eq!(lf.len(), 1);
        assert_eq!(lf[0], "/var/log/p.log");
    }

    #[test]
    fn finish_common_config_preserves_supplied_values() {
        let mut input = Map::new();
        input.insert("watcher".to_string(), Value::String("inotify".into()));
        input.insert("term_truecolor".to_string(), Value::Bool(true));
        let r = finish_common_config("UTF-8", &input);
        assert_eq!(r["watcher"], "inotify");
        assert_eq!(r["term_truecolor"], true);
    }

    #[test]
    fn finish_common_config_expands_tilde_in_paths() {
        // py:345-347  os.path.expanduser
        let _g = lock_env!();
        unsafe {
            std::env::set_var("HOME", "/home/alice");
        }
        let mut input = Map::new();
        input.insert(
            "paths".to_string(),
            Value::Array(vec![Value::String("~/custom".into())]),
        );
        let r = finish_common_config("UTF-8", &input);
        let paths = r["paths"].as_array().unwrap();
        assert_eq!(paths[0], "/home/alice/custom");
        unsafe {
            std::env::remove_var("HOME");
        }
    }

    #[test]
    fn powerline_logger_format_includes_ext_prefix_message() {
        // py:46-49  {ext}:{prefix}:{message}
        let l = PowerlineLogger::new("vim", "powerline");
        let s = l.format_message("hello", None);
        assert_eq!(s, "vim:powerline:hello");
    }

    #[test]
    fn powerline_logger_format_overrides_prefix() {
        let l = PowerlineLogger::new("vim", "default");
        let s = l.format_message("hi", Some("seg"));
        assert_eq!(s, "vim:seg:hi");
    }

    #[test]
    fn powerline_logger_log_captures_message() {
        let l = PowerlineLogger::new("shell", "p");
        l.debug("d");
        l.info("i");
        l.warn("w");
        l.error("e");
        l.critical("c");
        l.exception("ex");
        let msgs = l.messages.lock().unwrap();
        assert_eq!(msgs.len(), 6);
        assert_eq!(msgs[0].0, "DEBUG");
        assert_eq!(msgs[5].0, "EXCEPTION");
    }

    #[test]
    fn get_config_paths_includes_plugin_path_first() {
        let paths = get_config_paths();
        // plugin_path is always first per py:152
        assert_eq!(paths[0].file_name().unwrap(), "config_files");
    }

    #[test]
    fn get_config_paths_includes_xdg_config_home() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "/custom/cfg");
            std::env::remove_var("XDG_CONFIG_DIRS");
        }
        let paths = get_config_paths();
        let has_xdg = paths.iter().any(|p| p.starts_with("/custom/cfg"));
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        assert!(has_xdg);
    }
}

/// Port of `LOG_KEYS` from
/// `powerline/__init__.py:402`.
///
/// Set of config keys related to logging, used by `_get_log_keys`
/// to filter the common-config dict.
#[allow(non_snake_case)]
pub fn LOG_KEYS() -> &'static std::collections::HashSet<&'static str> {
    static S: std::sync::OnceLock<std::collections::HashSet<&'static str>> =
        std::sync::OnceLock::new();
    S.get_or_init(|| {
        // py:402  set(('log_format', 'log_level', 'log_file', 'paths'))
        let mut s = std::collections::HashSet::new();
        s.insert("log_format");
        s.insert("log_level");
        s.insert("log_file");
        s.insert("paths");
        s
    })
}

/// Port of `DEFAULT_UPDATE_INTERVAL` constant at
/// `powerline/__init__.py:422`.
pub const DEFAULT_UPDATE_INTERVAL: u64 = 2;

/// Port of `_get_log_keys()` from
/// `powerline/__init__.py:407-419`.
///
/// Returns a copy of `common_config` containing only the keys in
/// `LOG_KEYS`. Pure functional — no side effects.
pub fn _get_log_keys(
    common_config: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
    // py:417-419  {(k, v) for k, v in common_config.items() if k in LOG_KEYS}
    let log_keys = LOG_KEYS();
    let mut out = serde_json::Map::new();
    for (k, v) in common_config {
        if log_keys.contains(k.as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

/// Port of `_generate_change_callback()` from
/// `powerline/__init__.py:131-135`.
///
/// Returns a closure that, when invoked with a path, locks the
/// shared mutex and writes `key → true` to the dictionary. Used by
/// `Powerline.init` to wire ConfigLoader change notifications into
/// the cr_kwargs flag dict at py:502-508.
pub fn _generate_change_callback(
    lock: std::sync::Arc<std::sync::Mutex<serde_json::Map<String, serde_json::Value>>>,
    key: String,
) -> Box<dyn Fn(&str)> {
    // py:132-135  def on_file_change(path): with lock: dictionary[key] = True
    Box::new(move |_path: &str| {
        let mut m = lock.lock().unwrap_or_else(|e| e.into_inner());
        m.insert(key.clone(), serde_json::Value::Bool(true));
    })
}

/// Port of `Powerline` class constructor logic from
/// `powerline/__init__.py:427-522`.
///
/// Stores the constructor arguments + the resolved renderer module
/// name + the initial `cr_kwargs` flag dict. The full integration
/// with `ConfigLoader` / renderer creation / theme load is deferred
/// since each piece weaves through the not-yet-ported substrate.
#[derive(Debug, Clone)]
pub struct Powerline {
    /// py:480  self.ext = ext
    pub ext: String,
    /// py:481  self.run_once = run_once
    pub run_once: bool,
    /// py:483  self.had_logger = bool(self.logger)
    pub had_logger: bool,
    /// py:484  self.use_daemon_threads = use_daemon_threads
    pub use_daemon_threads: bool,
    /// py:486-495  self.renderer_module = ...
    pub renderer_module: String,
    /// py:522  self.update_interval = DEFAULT_UPDATE_INTERVAL
    pub update_interval: u64,
}

impl Powerline {
    /// Port of `Powerline.init()` from
    /// `powerline/__init__.py:464-522`.
    ///
    /// Captures the constructor args and resolves the renderer
    /// module name per the py:486-495 branch chain. The
    /// ConfigLoader/Lock/Event setup at py:497-522 is deferred to
    /// the future port pass.
    pub fn init(
        ext: &str,
        renderer_module: Option<&str>,
        run_once: bool,
        had_logger: bool,
    ) -> Self {
        // py:460  def __init__(self, *args, **kwargs):
        // py:461  self.init_args = (args, kwargs)
        // py:462  self.init(*args, **kwargs)
        // py:464  def init(self, ext, renderer_module=None, run_once=False,
        // py:465  logger=None, use_daemon_threads=True, shutdown_event=None,
        // py:466  config_loader=None):
        // py:480  self.ext = ext
        // py:481  self.run_once = run_once
        // py:482  self.logger = logger
        // py:483  self.had_logger = bool(self.logger)
        // py:484  self.use_daemon_threads = use_daemon_threads
        // py:486  if not renderer_module:
        // py:487  self.renderer_module = 'powerline.renderers.' + ext
        // py:488  elif '.' not in renderer_module:
        // py:489  self.renderer_module = 'powerline.renderers.' + renderer_module
        // py:490  elif renderer_module.startswith('.'):
        // py:491  self.renderer_module = 'powerline.renderers.' + ext + renderer_module
        // py:492  elif renderer_module.endswith('.'):
        // py:493  self.renderer_module = renderer_module[:-1]
        // py:494  else:
        // py:495  self.renderer_module = renderer_module
        // py:497  self.find_config_files = generate_config_finder(self.get_config_paths)
        // py:499  self.cr_kwargs_lock = Lock()
        // py:500  self.cr_kwargs = {}
        // py:501  self.cr_callbacks = {}
        // py:502  for key in ('main', 'colors', 'colorscheme', 'theme'):
        // py:503  self.cr_kwargs['load_' + key] = True
        // py:504  self.cr_callbacks[key] = _generate_change_callback(
        // py:505  self.cr_kwargs_lock,
        // py:506  'load_' + key,
        // py:507  self.cr_kwargs
        // py:508  )
        // py:510  self.shutdown_event = shutdown_event or Event()
        // py:511  self.config_loader = config_loader or ConfigLoader(...)
        // py:512  self.run_loader_update = False
        // py:514  self.renderer_options = {}
        // py:516  self.prev_common_config = None
        // py:517  self.prev_ext_config = None
        // py:518  self.pl = None
        // py:519  self.setup_args = ()
        // py:520  self.setup_kwargs = {}
        // py:521  self.imported_modules = set()
        // py:522  self.update_interval = DEFAULT_UPDATE_INTERVAL
        Powerline {
            ext: ext.to_string(),
            run_once,
            had_logger,
            use_daemon_threads: true,
            renderer_module: Self::resolve_renderer_module(ext, renderer_module),
            update_interval: DEFAULT_UPDATE_INTERVAL,
        }
    }

    /// Port of the renderer_module resolution branch at
    /// `powerline/__init__.py:486-495`.
    ///
    /// Branch chain:
    /// - None / empty → `powerline.renderers.<ext>` (py:486-487)
    /// - dot-free name → `powerline.renderers.<name>` (py:488-489)
    /// - leading dot → `powerline.renderers.<ext><name>` (py:490-491)
    /// - trailing dot → `<name without trailing dot>` (py:492-493)
    /// - everything else → as-is (py:494-495)
    pub fn resolve_renderer_module(ext: &str, renderer_module: Option<&str>) -> String {
        match renderer_module {
            // py:486-487  if not renderer_module
            None => format!("powerline.renderers.{}", ext),
            Some("") => format!("powerline.renderers.{}", ext),
            Some(name) => {
                if !name.contains('.') {
                    // py:488-489
                    format!("powerline.renderers.{}", name)
                } else if name.starts_with('.') {
                    // py:490-491
                    format!("powerline.renderers.{}{}", ext, name)
                } else if name.ends_with('.') {
                    // py:492-493
                    name.trim_end_matches('.').to_string()
                } else {
                    // py:494-495
                    name.to_string()
                }
            }
        }
    }

    /// Port of `Powerline.setup_components()` from
    /// `powerline/__init__.py:705-713`.
    ///
    /// Base implementation is a no-op per py:713. Subclasses (e.g.
    /// `VimPowerline.setup_components`) override to enable
    /// statusline/tabline.
    pub fn setup_components(&self, _components: Option<&[&str]>) {
        // py:713  pass
    }

    /// Port of `Powerline.get_local_themes()` from
    /// `powerline/__init__.py:833-847`.
    ///
    /// Static method returning None by default per py:847.
    /// Subclasses (e.g. `VimPowerline`) override to return the
    /// resolved local themes mapping.
    pub fn get_local_themes(
        _local_themes: Option<&serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // py:847  return None
        None
    }

    /// Port of `Powerline.do_setup()` from
    /// `powerline/__init__.py:915-922`.
    ///
    /// Static no-op per py:922. Subclasses override.
    pub fn do_setup() {
        // py:922  pass
    }

    /// Port of `Powerline.get_config_paths()` from
    /// `powerline/__init__.py:715-724`.
    ///
    /// Delegates to module-level `get_config_paths()`. Subclasses
    /// override to supply custom search paths.
    pub fn get_config_paths() -> Vec<std::path::PathBuf> {
        // py:724  return get_config_paths()
        get_config_paths()
    }

    /// Port of `Powerline.load_main_config()` from
    /// `powerline/__init__.py:750-755`.
    ///
    /// Delegates to `load_config('config', 'main')`. The Rust port
    /// takes a load closure since `ConfigLoader.load` isn't yet
    /// ported; callers wire it through.
    pub fn load_main_config<F>(load_fn: F) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str, &str) -> Result<Map<String, Value>, String>,
    {
        // py:755  return self.load_config('config', 'main')
        load_fn("config", "main")
    }

    /// Port of `Powerline.load_colors_config()` from
    /// `powerline/__init__.py:826-831`.
    pub fn load_colors_config<F>(load_fn: F) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str, &str) -> Result<Map<String, Value>, String>,
    {
        // py:831  return self.load_config('colors', 'colors')
        load_fn("colors", "colors")
    }

    /// Port of `Powerline.load_colorscheme_config()` level builder
    /// at `powerline/__init__.py:806-810`.
    ///
    /// Returns the (cfg_path_levels, ignore_levels) pair used by
    /// `_load_hierarhical_config`. Levels are joined to:
    /// 1. `colorschemes/<name>`
    /// 2. `colorschemes/<ext>/__main__` (ignore_level index 1)
    /// 3. `colorschemes/<ext>/<name>`
    pub fn load_colorscheme_config_levels(ext: &str, name: &str) -> (Vec<String>, Vec<usize>) {
        // py:806-811  levels = (...,); _load_hierarhical_config('colorscheme', levels, (1,))
        let levels = vec![
            // py:807
            format!("colorschemes/{}", name),
            // py:808
            format!("colorschemes/{}/__main__", ext),
            // py:809
            format!("colorschemes/{}/{}", ext, name),
        ];
        // py:811  (1,) — ignore_levels
        (levels, vec![1])
    }

    /// Port of `Powerline.load_theme_config()` level builder
    /// at `powerline/__init__.py:813-824`.
    ///
    /// `theme_levels` is the base list configured on the instance
    /// (e.g. `[themes/__main__, themes/<ext>/__main__]`). The fn
    /// appends `themes/<ext>/<name>` and returns the
    /// (levels, ignore_levels=[0, 1]) pair.
    pub fn load_theme_config_levels(
        theme_levels: &[String],
        ext: &str,
        name: &str,
    ) -> (Vec<String>, Vec<usize>) {
        // py:821-823  levels = self.theme_levels + (os.path.join('themes', self.ext, name),)
        let mut levels: Vec<String> = theme_levels.to_vec();
        levels.push(format!("themes/{}/{}", ext, name));
        // py:824  (0, 1,) — ignore_levels
        (levels, vec![0, 1])
    }

    /// Port of `Powerline.setup()` from
    /// `powerline/__init__.py:904-913`.
    ///
    /// Clears the shutdown_event, stashes the setup args for later
    /// reload, and calls do_setup. The Rust port surfaces the
    /// shutdown_event clear as a method on `&Arc<AtomicBool>`.
    pub fn setup(shutdown_event: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
        // py:910  self.shutdown_event.clear()
        shutdown_event.store(false, std::sync::atomic::Ordering::SeqCst);
        // py:913  self.do_setup(*args, **kwargs)
        Self::do_setup();
    }

    /// Port of `Powerline.shutdown()` from
    /// `powerline/__init__.py:953-973`.
    ///
    /// Sets the shutdown_event when `set_event=true` per py:965-966.
    /// The renderer.shutdown + config_loader.unregister_* calls
    /// (py:968-973) are deferred since they depend on the not-yet-ported
    /// Renderer and ConfigLoader.
    pub fn shutdown(
        shutdown_event: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        set_event: bool,
    ) {
        // py:953  def shutdown(self, set_event=True):
        // py:965  if set_event:
        if set_event {
            // py:966  self.shutdown_event.set()
            shutdown_event.store(true, std::sync::atomic::Ordering::SeqCst);
            // py:967  try:
            // py:968  self.renderer.shutdown()
            // py:969  except AttributeError:
            // py:970  pass
        }
        // py:971  functions = tuple(self.cr_callbacks.values())
        // py:972  self.config_loader.unregister_functions(set(functions))
        // py:973  self.config_loader.unregister_missing(set(((self.find_config_files, function) for function in functions)))
    }

    /// Port of `Powerline.load_config()` from
    /// `powerline/__init__.py:726-743`.
    ///
    /// Wraps the module-level `load_config(cfg_path,
    /// find_config_files, config_loader, cr_callbacks[cfg_type])`
    /// dispatch at py:738-742. The Rust port takes the resolved
    /// dispatch closure so callers route through their own
    /// config_loader.
    pub fn load_config_instance<F, LF>(
        cfg_path: &str,
        find_config_files: F,
        load_fn: LF,
    ) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str) -> Result<Vec<std::path::PathBuf>, String>,
        LF: Fn(&std::path::Path) -> Result<Map<String, Value>, String>,
    {
        // py:738-742  return load_config(cfg_path, ..., cr_callbacks[cfg_type])
        load_config(cfg_path, find_config_files, load_fn)
    }

    /// Port of `Powerline._purge_configs()` from
    /// `powerline/__init__.py:745-748`.
    ///
    /// Returns the (function_id, find_config_files_id) pair the
    /// caller would pass to `config_loader.unregister_functions` +
    /// `config_loader.unregister_missing` per py:746-748. The Rust
    /// port surfaces the id pair so callers route through the
    /// ConfigLoader port (which takes ids per its existing API).
    pub fn _purge_configs(function_id: u64) -> (u64, u64) {
        // py:746  function = self.cr_callbacks[cfg_type]
        // py:747-748  config_loader.unregister_functions({function})
        //              config_loader.unregister_missing({(find_config_files, function)})
        (function_id, function_id)
    }

    /// Port of `Powerline.load_colorscheme_config()` from
    /// `powerline/__init__.py:798-811`.
    ///
    /// Dispatches to `_load_hierarhical_config` with the 3-level
    /// list built by `load_colorscheme_config_levels` (already
    /// ported above). Caller-supplied `load_one` closure resolves
    /// each level path.
    pub fn load_colorscheme_config<F>(
        ext: &str,
        name: &str,
        load_one: F,
    ) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str) -> Result<Map<String, Value>, String>,
    {
        // py:806-810  build levels tuple
        let (levels, ignore) = Self::load_colorscheme_config_levels(ext, name);
        // py:811  _load_hierarhical_config('colorscheme', levels, (1,))
        _load_hierarhical_config(&levels, &ignore, load_one)
    }

    /// Port of `Powerline.load_theme_config()` from
    /// `powerline/__init__.py:813-824`.
    ///
    /// Dispatches to `_load_hierarhical_config` with the level
    /// list built by `load_theme_config_levels` (already ported).
    /// Caller supplies `theme_levels` (the instance's pre-resolved
    /// base levels) since the Rust struct doesn't carry them yet.
    pub fn load_theme_config<F>(
        theme_levels: &[String],
        ext: &str,
        name: &str,
        load_one: F,
    ) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str) -> Result<Map<String, Value>, String>,
    {
        // py:821-823  build levels = self.theme_levels + (themes/<ext>/<name>,)
        let (levels, ignore) = Self::load_theme_config_levels(theme_levels, ext, name);
        // py:824  _load_hierarhical_config('theme', levels, (0, 1,))
        _load_hierarhical_config(&levels, &ignore, load_one)
    }

    /// Port of `Powerline.exception()` from
    /// `powerline/__init__.py:981-991`.
    ///
    /// Returns the (prefix, msg, used_fallback) tuple callers route
    /// through the logger dispatch. Python defaults `kwargs['prefix']
    /// = 'powerline'` per py:982-983 when not supplied; uses
    /// `self.pl` when available, else falls back to
    /// `get_fallback_logger(self.default_log_stream)` per py:985.
    ///
    /// Returns `used_fallback=true` when the fallback logger was
    /// resolved (i.e. `self.pl` was None).
    pub fn powerline_exception(
        msg: &str,
        explicit_prefix: Option<&str>,
        has_pl: bool,
    ) -> (String, String, bool) {
        // py:982-983  if 'prefix' not in kwargs: kwargs['prefix'] = 'powerline'
        let prefix = explicit_prefix.unwrap_or("powerline").to_string();
        // py:985  pl = getattr(self, 'pl', None) or get_fallback_logger(...)
        let used_fallback = !has_pl;
        (prefix, msg.to_string(), used_fallback)
    }

    /// Port of `Powerline.__enter__()` from
    /// `powerline/__init__.py:975-976`.
    ///
    /// Python's context-manager entry returns `self`. The Rust port
    /// is a no-op since callers hold the Powerline directly. This fn
    /// exists for API parity.
    pub fn enter(&self) {
        // py:976  return self
    }

    /// Port of `Powerline.__exit__()` from
    /// `powerline/__init__.py:978-979`.
    ///
    /// Calls `self.shutdown()` per py:979. The Rust port takes the
    /// shutdown_event reference so callers can wire through the
    /// existing static shutdown.
    pub fn exit(shutdown_event: &std::sync::Arc<std::sync::atomic::AtomicBool>) {
        // py:979  self.shutdown()
        Self::shutdown(shutdown_event, true);
    }

    /// Port of `Powerline.update_renderer()` from
    /// `powerline/__init__.py:849-869`.
    ///
    /// Drives the cr_kwargs check + create_renderer dispatch.
    /// Returns:
    /// - `Ok(true)` when a renderer was (re-)created per py:858-868
    ///   success path
    /// - `Ok(false)` when no work was needed per py:855 (empty
    ///   cr_kwargs)
    /// - `Err(msg)` when create_renderer panics + no fallback
    ///   renderer was available per py:865-866 (raise)
    ///
    /// `has_pending_kwargs` mirrors py:854-856 (Python locks +
    /// snapshots cr_kwargs). `create_renderer` is the caller's
    /// closure for py:859 self.create_renderer(**cr_kwargs).
    /// `has_existing_renderer` mirrors py:862 hasattr(self,
    /// 'renderer').
    pub fn update_renderer<F>(
        has_pending_kwargs: bool,
        has_existing_renderer: bool,
        create_renderer: F,
    ) -> Result<bool, String>
    where
        F: FnOnce() -> Result<(), String>,
    {
        // py:849  def update_renderer(self):
        // py:851  if self.run_loader_update:
        // py:852  self.config_loader.update()
        // py:853  cr_kwargs = None
        // py:854  with self.cr_kwargs_lock:
        // py:855  if self.cr_kwargs:
        // py:856  cr_kwargs = self.cr_kwargs.copy()
        if !has_pending_kwargs {
            return Ok(false);
        }
        // py:857  if cr_kwargs:
        // py:858  try:
        // py:859  self.create_renderer(**cr_kwargs)
        match create_renderer() {
            Ok(()) => {
                // py:867  else:
                // py:868  with self.cr_kwargs_lock:
                // py:869  self.cr_kwargs.clear()
                Ok(true)
            }
            Err(e) => {
                // py:860  except Exception as e:
                // py:861  self.exception('Failed to create renderer: {0}', str(e))
                // py:862  if hasattr(self, 'renderer'):
                if has_existing_renderer {
                    // py:863  with self.cr_kwargs_lock:
                    // py:864  self.cr_kwargs.clear()
                    Ok(false)
                } else {
                    // py:865  else:
                    // py:866  raise
                    Err(format!("Failed to create renderer: {}", e))
                }
            }
        }
    }

    /// Port of `Powerline.render()` from
    /// `powerline/__init__.py:871-887`.
    ///
    /// Calls update_renderer + dispatches to renderer.render per
    /// py:876-877. On failure routes through FailedUnicode + the
    /// exception logger per py:878-887. `output_width=true`
    /// switches the failed return to a `(message, len)` pair per
    /// py:885-886.
    ///
    /// The Rust port returns:
    /// - `Ok(rendered)` on success
    /// - `Err((failed_message, optional_width))` on failure;
    ///   `Some(width)` when output_width=true.
    pub fn render<U, R>(
        update_renderer: U,
        renderer_render: R,
        output_width: bool,
    ) -> Result<String, (String, Option<usize>)>
    where
        U: FnOnce() -> Result<(), String>,
        R: FnOnce() -> Result<String, String>,
    {
        // py:871  def render(self, *args, **kwargs):
        // py:875  try:
        // py:876  self.update_renderer()
        // py:877  return self.renderer.render(*args, **kwargs)
        let result = update_renderer().and_then(|()| renderer_render());
        match result {
            Ok(s) => Ok(s),
            Err(e) => {
                // py:878  except Exception as e:
                // py:879  exc = e
                // py:880  try:
                // py:881  self.exception('Failed to render: {0}', str(e))
                // py:882  except Exception as e:
                // py:883  exc = e
                // py:884  ret = FailedUnicode(safe_unicode(exc))
                let failed = format!("Failed to render: {}", e);
                // py:885  if kwargs.get('output_width', False):
                // py:886  ret = ret, len(ret)
                let width = if output_width {
                    Some(failed.len())
                } else {
                    None
                };
                // py:887  return ret
                Err((failed, width))
            }
        }
    }

    /// Port of `Powerline.render_above_lines()` from
    /// `powerline/__init__.py:889-902`.
    ///
    /// Wraps update_renderer + renderer.render_above_lines per
    /// py:893-895. On failure yields a single FailedUnicode line
    /// per py:902. The Rust port returns the resolved line vec or
    /// a single error-line vec.
    pub fn render_above_lines<U, R>(update_renderer: U, renderer_above: R) -> Vec<String>
    where
        U: FnOnce() -> Result<(), String>,
        R: FnOnce() -> Result<Vec<String>, String>,
    {
        // py:889  def render_above_lines(self, *args, **kwargs):
        // py:892  try:
        // py:893  self.update_renderer()
        // py:894  for line in self.renderer.render_above_lines(*args, **kwargs):
        // py:895  yield line
        match update_renderer().and_then(|()| renderer_above()) {
            Ok(lines) => lines,
            Err(e) => {
                // py:896  except Exception as e:
                // py:897  exc = e
                // py:898  try:
                // py:899  self.exception('Failed to render: {0}', str(e))
                // py:900  except Exception as e:
                // py:901  exc = e
                // py:902  yield FailedUnicode(safe_unicode(exc))
                vec![format!("Failed to render: {}", e)]
            }
        }
    }

    /// Port of `Powerline._load_hierarhical_config()` from
    /// `powerline/__init__.py:757-796`.
    ///
    /// Wraps the module-level `_load_hierarhical_config` ported
    /// above. Iterates `levels`, merges successful loads, and
    /// errors when no non-ignored level produces a config per
    /// py:788-795.
    pub fn _load_hierarhical_config_instance<F>(
        levels: &[String],
        ignore_levels: &[usize],
        load_one: F,
    ) -> Result<Map<String, Value>, String>
    where
        F: Fn(&str) -> Result<Map<String, Value>, String>,
    {
        // py:772-796
        _load_hierarhical_config(levels, ignore_levels, load_one)
    }

    /// Port of `Powerline.create_logger()` from
    /// `powerline/__init__.py:530-548`.
    ///
    /// Returns a fresh PowerlineLogger keyed by `ext`. The Python
    /// source returns a 3-tuple `(logging.Logger, PowerlineLogger,
    /// get_module_attr)` per py:536-540; the Rust port surfaces
    /// just the PowerlineLogger since neither the stdlib logger
    /// nor the get_module_attr closure are reachable here.
    pub fn create_logger_instance(ext: &str) -> PowerlineLogger {
        // py:542-548  create_logger(common_config=..., ext=...)
        PowerlineLogger::new(ext, "")
    }

    /// Port of `Powerline.reload()` from
    /// `powerline/__init__.py:924-951`.
    ///
    /// Drives the reload sequence: clears modules, shuts down,
    /// re-constructs, and re-runs setup. The Rust port takes the
    /// caller-supplied closures for each step since the Python
    /// `sys.modules.pop` and `__import__` calls aren't reachable.
    pub fn reload<C, S, R>(
        shutdown_event: &std::sync::Arc<std::sync::atomic::AtomicBool>,
        clear_modules: C,
        shutdown_self: S,
        reconstruct: R,
    ) -> Result<(), String>
    where
        C: FnOnce() -> Result<(), String>,
        S: FnOnce(),
        R: FnOnce() -> Result<(), String>,
    {
        // py:937-946  clear sys.modules
        clear_modules()?;
        // py:948  self.shutdown(set_event=True)
        Self::shutdown(shutdown_event, true);
        shutdown_self();
        // py:949-951  re-construct + re-setup
        reconstruct()
    }
}

/// Port of `Powerline.reraise()` (staticmethod) from
/// `powerline/__init__.py:362-366`.
///
/// Python: re-raises either a `(exception, traceback)` tuple or a
/// bare exception. The Rust port has no equivalent to Python's
/// `raise exception.with_traceback(...)` since Rust errors don't
/// carry tracebacks; the fn surfaces the message-passing shape so
/// callers can route through any std::panic mechanism.
pub fn reraise(exception_msg: &str) -> String {
    // py:362-366  raise exception (or exception[0].with_traceback)
    exception_msg.to_string()
}

/// Port of `gen_module_attr_getter()` from
/// `powerline/__init__.py:369-405`.
///
/// Python returns a closure that imports `module` and looks up
/// `attr`. The Rust port can't perform Python-style `__import__`
/// dispatch; it surfaces the closure-factory shape so callers can
/// route through a Rust-side module-attribute table.
pub fn gen_module_attr_getter<F>(module_attr_lookup: F) -> Box<dyn Fn(&str, &str) -> Option<String>>
where
    F: Fn(&str, &str) -> Option<String> + Send + Sync + 'static,
{
    // py:369  def gen_module_attr_getter(pl, import_paths, imported_modules):
    // py:370  def get_module_attr(module, attr, prefix='powerline'):
    Box::new(move |module: &str, attr: &str| -> Option<String> {
        // py:386  oldpath = sys.path
        // py:387  sys.path = import_paths + sys.path
        // py:388  module = str(module)
        // py:389  attr = str(attr)
        // py:390  try:
        // py:391  imported_modules.add(module)
        // py:392  return getattr(__import__(module, fromlist=(attr,)), attr)
        // py:393  except Exception as e:
        // py:394  pl.exception('Failed to import attr {0} from module {1}: {2}', ...)
        // py:395  return None
        // py:396  finally:
        // py:397  sys.path = oldpath
        // py:399  return get_module_attr
        module_attr_lookup(module, attr)
    })
}

/// Port of `get_fallback_logger()` from
/// `powerline/__init__.py:111-128`.
///
/// Returns a `PowerlineLogger` with ext='powerline', prefix='_fallback_'.
/// Python caches this in a global; Rust port returns a fresh instance
/// each call since the captured-message buffer must not be shared
/// across callers in a test setting. The Python WARNING-level
/// `StreamHandler` wiring is not surfaced — the Rust PowerlineLogger
/// just captures messages internally.
pub fn get_fallback_logger() -> PowerlineLogger {
    // py:111  def get_fallback_logger(stream=None):
    // py:112  global _fallback_logger
    // py:113  if _fallback_logger:
    // py:114  return _fallback_logger
    // py:116  log_format = '%(asctime)s:%(levelname)s:%(message)s'
    // py:117  formatter = logging.Formatter(log_format)
    // py:119  level = logging.WARNING
    // py:120  handler = logging.StreamHandler(stream)
    // py:121  handler.setLevel(level)
    // py:122  handler.setFormatter(formatter)
    // py:124  logger = logging.Logger('powerline')
    // py:125  logger.setLevel(level)
    // py:126  logger.addHandler(handler)
    // py:127  _fallback_logger = PowerlineLogger(None, logger, '_fallback_')
    // py:128  return _fallback_logger
    PowerlineLogger::new("powerline", "_fallback_")
}

/// Port of `_set_log_handlers()` from
/// `powerline/__init__.py:203-259`.
///
/// Walks `common_config['log_file']` and attaches a handler per
/// entry to `logger`. The Python source dispatches to
/// `logging.StreamHandler` / `logging.FileHandler` / arbitrary
/// `module.HandlerClass` via `get_module_attr` per py:227-245.
/// The Rust port surfaces the dispatch shape — callers route
/// through their own logger plumbing since Python's `logging` is
/// not 1:1-portable. Returns the number of handlers attached per
/// py:217+257.
pub fn _set_log_handlers(common_config: &Map<String, Value>) -> Result<usize, String> {
    // py:216  log_targets = common_config['log_file']
    let log_targets = common_config
        .get("log_file")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    // py:217  num_handlers = 0
    let mut num_handlers: usize = 0;
    // py:218  for log_target in log_targets:
    for log_target in &log_targets {
        // py:219  if log_target is None:
        // py:220  log_target = ['logging.StreamHandler', []]
        if log_target.is_null() {
            num_handlers += 1;
            continue;
        }
        // py:221  elif isinstance(log_target, unicode):
        if let Some(s) = log_target.as_str() {
            // py:222  log_target = os.path.expanduser(log_target)
            let expanded = if let Some(home) = std::env::var_os("HOME") {
                if s.starts_with('~') {
                    s.replacen('~', &home.to_string_lossy(), 1)
                } else {
                    s.to_string()
                }
            } else {
                s.to_string()
            };
            // py:223  log_dir = os.path.dirname(log_target)
            let log_dir = std::path::Path::new(&expanded)
                .parent()
                .map(|p| p.to_path_buf());
            // py:224  if log_dir and not os.path.isdir(log_dir):
            // py:225  os.mkdir(log_dir)
            if let Some(d) = log_dir {
                if !d.as_os_str().is_empty() && !d.is_dir() {
                    let _ = std::fs::create_dir_all(&d);
                }
            }
            // py:226  log_target = ['logging.FileHandler', [[log_target]]]
            num_handlers += 1;
            continue;
        }
        // py:227  module, handler_class_name = log_target[0].rpartition('.')[::2]
        // py:228  module = module or 'logging.handlers'
        // py:229-235  handler_class_args = log_target[1][0] or ([stream] / ())
        // py:236-239  handler_class_kwargs = log_target[1][1] or {}
        // py:240  module = str(module)
        // py:241  handler_class_name = str(handler_class_name)
        // py:242  handler_class = get_module_attr(module, handler_class_name)
        // py:243-244  if not handler_class: continue
        // py:245  handler = handler_class(*handler_class_args, **handler_class_kwargs)
        // py:246-249  handler_level_name = log_target[2] or common_config['log_level']
        // py:250-253  handler_format = log_target[3] or common_config['log_format']
        // py:254  handler.setLevel(getattr(logging, handler_level_name))
        // py:255  handler.setFormatter(logging.Formatter(handler_format))
        // py:256  logger.addHandler(handler)
        // py:257  num_handlers += 1
        num_handlers += 1;
    }
    // py:258  if num_handlers == 0 and log_targets:
    // py:259  raise ValueError('Failed to set up any handlers')
    if num_handlers == 0 && !log_targets.is_empty() {
        return Err("Failed to set up any handlers".to_string());
    }
    Ok(num_handlers)
}

/// Port of `create_logger()` from
/// `powerline/__init__.py:262-298`.
///
/// Returns a fresh PowerlineLogger keyed by `ext`. The Python
/// source returns a 3-tuple `(logging.Logger, PowerlineLogger,
/// get_module_attr)` per py:281-298; the Rust port surfaces the
/// PowerlineLogger only since the stdlib logger and the
/// `gen_module_attr_getter` closure are not 1:1 portable.
pub fn create_logger(_common_config: &Map<String, Value>, ext: &str) -> PowerlineLogger {
    // py:262  def create_logger(common_config, use_daemon_threads=True, ext='__unknown__',
    // py:263  import_paths=None, imported_modules=None, stream=None):
    // py:287  logger = logging.Logger('powerline')
    // py:288  level = getattr(logging, common_config['log_level'])
    // py:289  logger.setLevel(level)
    // py:291  pl = PowerlineLogger(use_daemon_threads, logger, ext)
    // py:292-294  get_module_attr = gen_module_attr_getter(
    //   pl, common_config['paths'],
    //   set() if imported_modules is None else imported_modules)
    // py:296  _set_log_handlers(common_config, logger, get_module_attr, stream)
    // py:298  return logger, pl, get_module_attr
    PowerlineLogger::new(ext, "")
}

/// Port of `Powerline.create_renderer()` orchestration from
/// `powerline/__init__.py:550-696`.
///
/// The Python source is a 147-line method that drives the
/// (re)load + merge + renderer construction sequence. The Rust
/// port surfaces the high-level branch shape — callers supply
/// closures for each substrate-dependent step (load_main_config,
/// finish_common_config, create_logger, etc) since the Python
/// `__import__`/`getattr` chain is not directly portable.
///
/// `load_main` / `load_colors` / `load_colorscheme` / `load_theme`
/// mirror the kwargs at py:550. Returns `Ok(true)` when a renderer
/// was created per py:677-696, `Ok(false)` when no renderer work
/// was needed, `Err(msg)` when create_renderer panics without an
/// existing renderer fallback per py:689-693.
pub fn create_renderer<M, C>(
    load_main: bool,
    load_colors: bool,
    load_colorscheme: bool,
    load_theme: bool,
    has_existing_renderer: bool,
    construct: C,
    _load_main_config: M,
) -> Result<bool, String>
where
    M: FnOnce() -> Result<Map<String, Value>, String>,
    C: FnOnce() -> Result<(), String>,
{
    // py:569  common_config_differs = False
    let mut common_config_differs = false;
    // py:570  ext_config_differs = False
    let ext_config_differs = false;
    // py:571  if load_main:
    if load_main {
        // py:572  self._purge_configs('main')
        // py:573  config = self.load_main_config()
        // py:574  self.common_config = finish_common_config(self.get_encoding(), config['common'])
        // py:575  if self.common_config != self.prev_common_config:
        // py:576  common_config_differs = True
        common_config_differs = true;
        // py:578-580  load_theme = (load_theme or not prev or default_top_theme differs)
        // py:582-584  log_keys_differ = (not prev or _get_log_keys differs)
        // py:586  self.prev_common_config = self.common_config
        // py:588-595  if log_keys_differ: re-create logger
        // py:597-598  if not self.run_once: config_loader.set_watcher(...)
        // py:600-614  mergedicts(self.renderer_options, dict(pl=..., term_truecolor=..., ...))
        // py:616-621  if not run_once and reload_config: set_interval + maybe start
        // py:623  self.ext_config = config['ext'][self.ext]
        // py:625-628  top_theme = ext_config.get('top_theme') or common_config['default_top_theme']
        // py:629-632  self.theme_levels = (themes/<top_theme>, themes/<ext>/__main__)
        // py:633  self.renderer_options['theme_kwargs']['top_theme'] = top_theme
        // py:635  if self.ext_config != self.prev_ext_config:
        // py:636  ext_config_differs = True
        // py:637-641  if components differ: setup_components(...)
        // py:642-646  if local_themes differ: renderer_options['local_themes'] = get_local_themes(...)
        // py:647  self.update_interval = ext_config.get('update_interval', 2)
        // py:648-652  load_colorscheme = (load_colorscheme or not prev or colorscheme differs)
        // py:653-657  load_theme = (load_theme or not prev or theme differs)
        // py:658  self.prev_ext_config = self.ext_config
    }
    // py:660  create_renderer = load_colors or load_colorscheme or load_theme or common_config_differs or ext_config_differs
    let create_renderer_flag = load_colors
        || load_colorscheme
        || load_theme
        || common_config_differs
        || ext_config_differs;
    // py:662  if load_colors:
    if load_colors {
        // py:663  self._purge_configs('colors')
        // py:664  self.colors_config = self.load_colors_config()
    }
    // py:666  if load_colorscheme or load_colors:
    if load_colorscheme || load_colors {
        // py:667  self._purge_configs('colorscheme')
        // py:668-669  if load_colorscheme: self.colorscheme_config = self.load_colorscheme_config(...)
        // py:670-671  renderer_options['theme_kwargs']['colorscheme'] = Colorscheme(...)
    }
    // py:673  if load_theme:
    if load_theme {
        // py:674  self._purge_configs('theme')
        // py:675  renderer_options['theme_config'] = self.load_theme_config(...)
    }
    // py:677  if create_renderer:
    if create_renderer_flag {
        // py:678  Renderer = self.get_module_attr(self.renderer_module, 'renderer')
        // py:679-683  if not Renderer: if hasattr(renderer): return else raise ImportError
        // py:688-689  try: renderer = Renderer(**self.renderer_options)
        match construct() {
            Ok(()) => {
                // py:694-695  else: self.renderer = renderer
                Ok(true)
            }
            // py:690-693  except: log + if not hasattr(renderer): raise
            Err(e) => {
                if has_existing_renderer {
                    // py:692  if not hasattr(self, 'renderer'): raise
                    Ok(false)
                } else {
                    Err(format!("Failed to construct renderer object: {}", e))
                }
            }
        }
    } else {
        Ok(false)
    }
}

/// Port of `generate_config_finder()` from
/// `powerline/__init__.py:156-170`.
///
/// Returns a closure that, given a config-file name, calls
/// `_find_config_files(config_paths, cfg_path)` per py:170.
pub fn generate_config_finder(
    get_paths: impl Fn() -> Vec<std::path::PathBuf>,
) -> Box<dyn Fn(&str) -> Result<Vec<std::path::PathBuf>, String>> {
    // py:169  config_paths = get_config_paths()
    let config_paths = get_paths();
    // py:170  return lambda *args: _find_config_files(config_paths, *args)
    Box::new(move |cfg_path: &str| _find_config_files(&config_paths, cfg_path))
}

/// Port of `load_config()` from
/// `powerline/__init__.py:173-200`.
///
/// Loads + merges all configs found at `cfg_path`. The
/// `find_config_files` closure resolves the file paths;
/// `load_fn(path)` loads a single file (the Python source calls
/// `config_loader.load(path)`; Rust port takes a closure since the
/// ConfigLoader port is deferred).
pub fn load_config<FF, LF>(
    cfg_path: &str,
    find_config_files: FF,
    load_fn: LF,
) -> Result<Map<String, Value>, String>
where
    FF: Fn(&str) -> Result<Vec<std::path::PathBuf>, String>,
    LF: Fn(&std::path::Path) -> Result<Map<String, Value>, String>,
{
    // py:191  found_files = find_config_files(cfg_path, ...)
    let found_files = find_config_files(cfg_path)?;
    // py:192  ret = None
    let mut ret: Option<Map<String, Value>> = None;
    // py:193-199  for path in found_files: merge into ret
    for path in &found_files {
        let cfg = load_fn(path)?;
        match ret.as_mut() {
            None => ret = Some(cfg),
            Some(acc) => {
                // py:199  mergedicts(ret, config_loader.load(path))
                crate::ported::lib::dict::mergedicts(acc, cfg, false);
            }
        }
    }
    // py:200  return ret
    ret.ok_or_else(|| format!("No config files found for {}", cfg_path))
}

/// Port of `Powerline._load_hierarhical_config()` from
/// `powerline/__init__.py:757-796`.
///
/// Walks `levels` calling `load_one(cfg_path)` for each. Merges all
/// successful loads. Tracks which non-ignored levels loaded; raises
/// when none did.
///
/// `ignore_levels` is the list of level indices (0-based) that are
/// allowed to be missing without contributing to the `loaded` count
/// per py:785-786.
pub fn _load_hierarhical_config<F>(
    levels: &[String],
    ignore_levels: &[usize],
    load_one: F,
) -> Result<Map<String, Value>, String>
where
    F: Fn(&str) -> Result<Map<String, Value>, String>,
{
    // py:772  config = {}
    let mut config = Map::new();
    // py:773  loaded = 0
    let mut loaded = 0;
    // py:774  exceptions = []
    let mut last_err: Option<String> = None;
    // py:775  for i, cfg_path in enumerate(levels):
    for (i, cfg_path) in levels.iter().enumerate() {
        // py:777  lvl_config = self.load_config(cfg_path, cfg_type)
        match load_one(cfg_path) {
            Ok(lvl_config) => {
                // py:785  if i not in ignore_levels: loaded += 1
                if !ignore_levels.contains(&i) {
                    loaded += 1;
                }
                // py:787  mergedicts(config, lvl_config)
                crate::ported::lib::dict::mergedicts(&mut config, lvl_config, false);
            }
            Err(e) => {
                // py:778-783  exceptions.append((e, tb))
                last_err = Some(e);
            }
        }
    }
    // py:788  if not loaded: raise e
    if loaded == 0 {
        return Err(last_err.unwrap_or_else(|| "No config files loaded".to_string()));
    }
    Ok(config)
}

#[cfg(test)]
mod powerline_class_tests {
    use super::*;

    #[test]
    fn log_keys_has_four_entries() {
        // py:402
        let k = LOG_KEYS();
        assert_eq!(k.len(), 4);
        assert!(k.contains("log_format"));
        assert!(k.contains("log_level"));
        assert!(k.contains("log_file"));
        assert!(k.contains("paths"));
    }

    #[test]
    fn default_update_interval_is_two() {
        // py:422
        assert_eq!(DEFAULT_UPDATE_INTERVAL, 2);
    }

    #[test]
    fn get_log_keys_filters_dict_to_log_keys_only() {
        // py:417-419
        let mut cfg = serde_json::Map::new();
        cfg.insert(
            "log_format".to_string(),
            serde_json::Value::String("X".into()),
        );
        cfg.insert(
            "log_level".to_string(),
            serde_json::Value::String("DEBUG".into()),
        );
        cfg.insert("ambiwidth".to_string(), serde_json::Value::from(1));
        cfg.insert("term_truecolor".to_string(), serde_json::Value::Bool(true));
        let r = _get_log_keys(&cfg);
        assert_eq!(r.len(), 2);
        assert!(r.contains_key("log_format"));
        assert!(r.contains_key("log_level"));
        assert!(!r.contains_key("ambiwidth"));
    }

    #[test]
    fn get_log_keys_empty_input_returns_empty() {
        let cfg = serde_json::Map::new();
        let r = _get_log_keys(&cfg);
        assert!(r.is_empty());
    }

    #[test]
    fn get_log_keys_no_log_keys_returns_empty() {
        let mut cfg = serde_json::Map::new();
        cfg.insert("foo".to_string(), serde_json::Value::Bool(true));
        cfg.insert("bar".to_string(), serde_json::Value::from(42));
        let r = _get_log_keys(&cfg);
        assert!(r.is_empty());
    }

    #[test]
    fn generate_change_callback_sets_key_true_on_invocation() {
        // py:131-135
        let m = std::sync::Arc::new(std::sync::Mutex::new(serde_json::Map::new()));
        let cb = _generate_change_callback(m.clone(), "load_main".to_string());
        cb("/some/path/config.json");
        let map = m.lock().unwrap();
        assert_eq!(map.get("load_main"), Some(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn generate_change_callback_overwrites_existing_value() {
        let m = std::sync::Arc::new(std::sync::Mutex::new(serde_json::Map::new()));
        {
            let mut map = m.lock().unwrap();
            map.insert("load_theme".to_string(), serde_json::Value::Bool(false));
        }
        let cb = _generate_change_callback(m.clone(), "load_theme".to_string());
        cb("/path/theme.json");
        let map = m.lock().unwrap();
        assert_eq!(map.get("load_theme"), Some(&serde_json::Value::Bool(true)));
    }

    #[test]
    fn powerline_init_resolves_default_renderer_module() {
        // py:486-487
        let p = Powerline::init("shell", None, false, false);
        assert_eq!(p.ext, "shell");
        assert_eq!(p.renderer_module, "powerline.renderers.shell");
        assert!(!p.run_once);
        assert!(!p.had_logger);
        assert!(p.use_daemon_threads);
        assert_eq!(p.update_interval, 2);
    }

    #[test]
    fn powerline_init_run_once_flag() {
        let p = Powerline::init("shell", None, true, true);
        assert!(p.run_once);
        assert!(p.had_logger);
    }

    #[test]
    fn resolve_renderer_module_undotted_uses_renderers_prefix() {
        // py:488-489
        assert_eq!(
            Powerline::resolve_renderer_module("shell", Some("zsh")),
            "powerline.renderers.zsh"
        );
    }

    #[test]
    fn resolve_renderer_module_leading_dot_appends_to_ext() {
        // py:490-491  '.foo' → 'powerline.renderers.<ext>.foo'
        assert_eq!(
            Powerline::resolve_renderer_module("shell", Some(".zsh")),
            "powerline.renderers.shell.zsh"
        );
    }

    #[test]
    fn resolve_renderer_module_trailing_dot_strips_dot() {
        // py:492-493  'foo.' → 'foo'
        assert_eq!(
            Powerline::resolve_renderer_module("shell", Some("custom_renderer.")),
            "custom_renderer"
        );
    }

    #[test]
    fn resolve_renderer_module_dotted_passes_through() {
        // py:494-495
        assert_eq!(
            Powerline::resolve_renderer_module("shell", Some("my.custom.module")),
            "my.custom.module"
        );
    }

    #[test]
    fn resolve_renderer_module_empty_str_uses_default() {
        // py:486 if not renderer_module — Python: empty string is falsy
        assert_eq!(
            Powerline::resolve_renderer_module("vim", Some("")),
            "powerline.renderers.vim"
        );
    }

    #[test]
    fn setup_components_base_is_no_op() {
        // py:713
        let p = Powerline::init("shell", None, false, false);
        p.setup_components(Some(&["statusline", "tabline"]));
        // No-op; just verify the call succeeds.
    }

    #[test]
    fn get_local_themes_base_returns_none() {
        // py:847
        assert_eq!(Powerline::get_local_themes(None), None);
        assert_eq!(
            Powerline::get_local_themes(Some(&serde_json::json!({"x": "y"}))),
            None
        );
    }

    #[test]
    fn do_setup_base_is_no_op() {
        // py:922
        Powerline::do_setup();
    }

    #[test]
    fn powerline_get_config_paths_delegates_to_module_fn() {
        // py:724
        let paths_class = Powerline::get_config_paths();
        let paths_module = get_config_paths();
        assert_eq!(paths_class, paths_module);
    }

    #[test]
    fn get_fallback_logger_returns_powerline_fallback_pair() {
        // py:124-127
        let pl = get_fallback_logger();
        assert_eq!(pl.ext, "powerline");
        assert_eq!(pl.prefix, "_fallback_");
    }

    #[test]
    fn generate_config_finder_dispatches_to_find_config_files() {
        // py:156-170
        let d = std::env::temp_dir().join(format!(
            "powerliners-gcf-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("test.json");
        std::fs::write(&f, "{}").unwrap();

        let d_clone = d.clone();
        let finder = generate_config_finder(move || vec![d_clone.clone()]);
        let result = finder("test").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], f);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn generate_config_finder_returns_err_when_missing() {
        let finder = generate_config_finder(|| vec![std::path::PathBuf::from("/never/exists/x")]);
        assert!(finder("nonexistent").is_err());
    }

    #[test]
    fn load_config_loads_single_file() {
        // py:173-200  single file load
        let d = std::env::temp_dir().join(format!(
            "powerliners-lc-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("config.json");
        std::fs::write(&f, r#"{"key": "value"}"#).unwrap();

        let d_clone = d.clone();
        let finder = move |p: &str| _find_config_files(std::slice::from_ref(&d_clone), p);
        let result = load_config("config", finder, |p| {
            let raw = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            v.as_object()
                .cloned()
                .ok_or_else(|| "not an object".to_string())
        })
        .unwrap();
        assert_eq!(result.get("key"), Some(&Value::String("value".into())));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn load_config_merges_multiple_paths() {
        // py:198-199  mergedicts on multiple paths
        let d1 = std::env::temp_dir().join(format!(
            "powerliners-lcm1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let d2 = std::env::temp_dir().join(format!(
            "powerliners-lcm2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d1).unwrap();
        std::fs::create_dir_all(&d2).unwrap();
        std::fs::write(d1.join("c.json"), r#"{"a": 1, "b": 2}"#).unwrap();
        std::fs::write(d2.join("c.json"), r#"{"b": 3, "c": 4}"#).unwrap();

        let d1c = d1.clone();
        let d2c = d2.clone();
        let finder = move |p: &str| _find_config_files(&[d1c.clone(), d2c.clone()], p);
        let result = load_config("c", finder, |p| {
            let raw = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
            let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            v.as_object()
                .cloned()
                .ok_or_else(|| "not an object".to_string())
        })
        .unwrap();
        // Second config overrides first
        assert_eq!(result.get("a"), Some(&Value::from(1)));
        assert_eq!(result.get("b"), Some(&Value::from(3)));
        assert_eq!(result.get("c"), Some(&Value::from(4)));
        std::fs::remove_dir_all(&d1).ok();
        std::fs::remove_dir_all(&d2).ok();
    }

    #[test]
    fn load_colorscheme_config_levels_returns_three_levels() {
        // py:806-810
        let (levels, ignore) = Powerline::load_colorscheme_config_levels("shell", "default");
        assert_eq!(
            levels,
            vec![
                "colorschemes/default".to_string(),
                "colorschemes/shell/__main__".to_string(),
                "colorschemes/shell/default".to_string(),
            ]
        );
        assert_eq!(ignore, vec![1]);
    }

    #[test]
    fn load_theme_config_levels_appends_ext_name() {
        // py:821-823
        let base = vec![
            "themes/__main__".to_string(),
            "themes/shell/__main__".to_string(),
        ];
        let (levels, ignore) = Powerline::load_theme_config_levels(&base, "shell", "default");
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[2], "themes/shell/default");
        assert_eq!(ignore, vec![0, 1]);
    }

    #[test]
    fn powerline_shutdown_sets_event() {
        // py:965-966
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        Powerline::shutdown(&ev, true);
        assert!(ev.load(Ordering::SeqCst));
    }

    #[test]
    fn powerline_shutdown_no_event_set_when_flag_false() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        Powerline::shutdown(&ev, false);
        assert!(!ev.load(Ordering::SeqCst));
    }

    #[test]
    fn powerline_setup_clears_shutdown_event_and_calls_do_setup() {
        // py:910-913
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(true));
        Powerline::setup(&ev);
        assert!(!ev.load(Ordering::SeqCst));
    }

    #[test]
    fn powerline_load_main_config_dispatches_to_main_kind() {
        // py:755
        let r = Powerline::load_main_config(|cfg, kind| {
            let mut m = Map::new();
            m.insert("cfg".to_string(), Value::String(cfg.to_string()));
            m.insert("kind".to_string(), Value::String(kind.to_string()));
            Ok(m)
        })
        .unwrap();
        assert_eq!(r.get("cfg"), Some(&Value::String("config".into())));
        assert_eq!(r.get("kind"), Some(&Value::String("main".into())));
    }

    #[test]
    fn powerline_load_colors_config_dispatches_to_colors_kind() {
        // py:831
        let r = Powerline::load_colors_config(|cfg, kind| {
            let mut m = Map::new();
            m.insert("cfg".to_string(), Value::String(cfg.to_string()));
            m.insert("kind".to_string(), Value::String(kind.to_string()));
            Ok(m)
        })
        .unwrap();
        assert_eq!(r.get("cfg"), Some(&Value::String("colors".into())));
        assert_eq!(r.get("kind"), Some(&Value::String("colors".into())));
    }

    #[test]
    fn load_hierarhical_config_merges_levels() {
        // py:772-787
        let levels = vec![
            "level_a".to_string(),
            "level_b".to_string(),
            "level_c".to_string(),
        ];
        let r = _load_hierarhical_config(&levels, &[], |p| {
            let mut m = Map::new();
            m.insert(p.to_string(), Value::Bool(true));
            Ok(m)
        })
        .unwrap();
        assert!(r.contains_key("level_a"));
        assert!(r.contains_key("level_b"));
        assert!(r.contains_key("level_c"));
    }

    #[test]
    fn load_hierarhical_config_errors_when_no_levels_load() {
        // py:788-795  if not loaded: raise e
        let levels = vec!["a".to_string(), "b".to_string()];
        let r: Result<Map<String, Value>, _> =
            _load_hierarhical_config(&levels, &[], |_| Err("nope".to_string()));
        assert!(r.is_err());
    }

    #[test]
    fn load_hierarhical_config_ignores_specified_levels() {
        // py:785-786  if i not in ignore_levels: loaded += 1
        let levels = vec!["a".to_string(), "b".to_string()];
        // Only level 0 loads; level 1 errors. Since level 0 is NOT
        // in ignore_levels, loaded == 1 and the call succeeds.
        let r = _load_hierarhical_config(&levels, &[], |p| {
            if p == "a" {
                let mut m = Map::new();
                m.insert("x".to_string(), Value::Bool(true));
                Ok(m)
            } else {
                Err("missing".to_string())
            }
        })
        .unwrap();
        assert!(r.contains_key("x"));
    }

    #[test]
    fn load_hierarhical_config_ignore_levels_skipped_in_loaded_count() {
        // Only level 0 is ignored (in ignore_levels); levels 1 and 2
        // are required. If only level 0 loads → loaded count = 0 →
        // raises.
        let levels = vec!["a".to_string(), "b".to_string()];
        let r: Result<Map<String, Value>, _> = _load_hierarhical_config(&levels, &[0], |p| {
            if p == "a" {
                let mut m = Map::new();
                m.insert("x".to_string(), Value::Bool(true));
                Ok(m)
            } else {
                Err("missing".to_string())
            }
        });
        assert!(r.is_err());
    }

    #[test]
    fn powerline_load_config_instance_dispatches_through_finder_and_load_fn() {
        // py:738-742
        let d = std::env::temp_dir().join(format!(
            "powerliners-load-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("test.json"), r#"{"k": 1}"#).unwrap();

        let d_clone = d.clone();
        let r = Powerline::load_config_instance(
            "test",
            move |p| _find_config_files(std::slice::from_ref(&d_clone), p),
            |p| {
                let raw = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
                let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                v.as_object()
                    .cloned()
                    .ok_or_else(|| "not an object".to_string())
            },
        )
        .unwrap();
        assert_eq!(r.get("k"), Some(&Value::from(1)));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn powerline_purge_configs_returns_function_id_pair() {
        // py:746-748
        let (fn_id, finder_fn_id) = Powerline::_purge_configs(42);
        assert_eq!(fn_id, 42);
        assert_eq!(finder_fn_id, 42);
    }

    #[test]
    fn powerline_load_colorscheme_config_walks_three_levels() {
        // py:806-811
        use std::cell::Cell;
        let visited = Cell::new(Vec::<String>::new());
        let result = Powerline::load_colorscheme_config("shell", "default", |p| {
            let mut v = visited.take();
            v.push(p.to_string());
            visited.set(v);
            let mut m = Map::new();
            m.insert("level".to_string(), Value::String(p.to_string()));
            Ok(m)
        });
        assert!(result.is_ok());
        let v = visited.into_inner();
        // py:806-810  3 levels: colorschemes/default, colorschemes/shell/__main__,
        //   colorschemes/shell/default
        assert!(v.contains(&"colorschemes/default".to_string()));
        assert!(v.contains(&"colorschemes/shell/__main__".to_string()));
        assert!(v.contains(&"colorschemes/shell/default".to_string()));
    }

    #[test]
    fn powerline_load_theme_config_appends_ext_name_level() {
        // py:821-823
        use std::cell::Cell;
        let base = vec![
            "themes/__main__".to_string(),
            "themes/shell/__main__".to_string(),
        ];
        let visited = Cell::new(Vec::<String>::new());
        let r = Powerline::load_theme_config(&base, "shell", "default", |p| {
            let mut v = visited.take();
            v.push(p.to_string());
            visited.set(v);
            let mut m = Map::new();
            m.insert("level".to_string(), Value::String(p.to_string()));
            Ok(m)
        });
        assert!(r.is_ok());
        let v = visited.into_inner();
        // py:822  themes/<ext>/<name> appended
        assert!(v.contains(&"themes/shell/default".to_string()));
    }

    #[test]
    fn powerline_exception_defaults_prefix_to_powerline() {
        // py:982-983
        let (prefix, msg, used_fallback) = Powerline::powerline_exception("oops", None, true);
        assert_eq!(prefix, "powerline");
        assert_eq!(msg, "oops");
        assert!(!used_fallback);
    }

    #[test]
    fn powerline_exception_explicit_prefix_used() {
        let (prefix, _, _) = Powerline::powerline_exception("oops", Some("custom"), true);
        assert_eq!(prefix, "custom");
    }

    #[test]
    fn powerline_exception_no_pl_returns_fallback_flag() {
        // py:985  self.pl or get_fallback_logger
        let (_, _, used_fallback) = Powerline::powerline_exception("oops", None, false);
        assert!(used_fallback);
    }

    #[test]
    fn powerline_enter_is_no_op() {
        // py:975-976
        let p = Powerline::init("shell", None, false, false);
        p.enter();
    }

    #[test]
    fn powerline_exit_sets_shutdown_event() {
        // py:979  self.shutdown()
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        Powerline::exit(&ev);
        assert!(ev.load(Ordering::SeqCst));
    }

    #[test]
    fn update_renderer_no_pending_kwargs_returns_false() {
        // py:855  if self.cr_kwargs: ... else skip
        let r = Powerline::update_renderer(false, true, || panic!("should not run")).unwrap();
        assert!(!r);
    }

    #[test]
    fn update_renderer_success_returns_true() {
        // py:858-869
        let r = Powerline::update_renderer(true, false, || Ok(())).unwrap();
        assert!(r);
    }

    #[test]
    fn update_renderer_failure_with_existing_renderer_returns_false() {
        // py:862-864  fallback to existing
        let r = Powerline::update_renderer(true, true, || Err("boom".to_string())).unwrap();
        assert!(!r);
    }

    #[test]
    fn update_renderer_failure_without_existing_renderer_returns_err() {
        // py:865-866  raise
        let r = Powerline::update_renderer(true, false, || Err("boom".to_string()));
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Failed to create renderer"));
    }

    #[test]
    fn render_success_returns_rendered_string() {
        // py:876-877
        let r = Powerline::render(|| Ok(()), || Ok("rendered".to_string()), false);
        assert_eq!(r, Ok("rendered".to_string()));
    }

    #[test]
    fn render_failure_returns_failed_message_without_width() {
        // py:878-887
        let r = Powerline::render(|| Ok(()), || Err("error".to_string()), false);
        let (msg, width) = r.unwrap_err();
        assert!(msg.contains("Failed to render"));
        assert!(msg.contains("error"));
        assert!(width.is_none());
    }

    #[test]
    fn render_failure_with_output_width_returns_message_and_len() {
        // py:885-886
        let r = Powerline::render(|| Ok(()), || Err("err".to_string()), true);
        let (msg, width) = r.unwrap_err();
        assert_eq!(width, Some(msg.len()));
    }

    #[test]
    fn render_update_renderer_failure_routes_to_failed() {
        let r = Powerline::render(
            || Err("update fail".to_string()),
            || panic!("should not run after update fail"),
            false,
        );
        assert!(r.is_err());
        assert!(r.unwrap_err().0.contains("update fail"));
    }

    #[test]
    fn render_above_lines_success_returns_yielded_lines() {
        // py:893-895
        let lines = Powerline::render_above_lines(
            || Ok(()),
            || Ok(vec!["line1".to_string(), "line2".to_string()]),
        );
        assert_eq!(lines, vec!["line1".to_string(), "line2".to_string()]);
    }

    #[test]
    fn render_above_lines_failure_yields_single_failed_line() {
        // py:896-902
        let lines =
            Powerline::render_above_lines(|| Err("boom".to_string()), || panic!("should not run"));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("Failed to render"));
    }

    #[test]
    fn load_hierarhical_config_instance_delegates_to_module_fn() {
        // py:757-796
        let levels = vec!["a".to_string(), "b".to_string()];
        let r = Powerline::_load_hierarhical_config_instance(&levels, &[], |p| {
            let mut m = Map::new();
            m.insert(p.to_string(), Value::Bool(true));
            Ok(m)
        })
        .unwrap();
        assert!(r.contains_key("a"));
        assert!(r.contains_key("b"));
    }

    #[test]
    fn create_logger_instance_builds_powerline_logger_for_ext() {
        // py:542-548
        let pl = Powerline::create_logger_instance("vim");
        assert_eq!(pl.ext, "vim");
    }

    #[test]
    fn reload_runs_all_three_phases_in_order() {
        // py:937-951
        use std::cell::Cell;
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        let phases = Cell::new(Vec::<&str>::new());

        let r = Powerline::reload(
            &ev,
            || {
                let mut v = phases.take();
                v.push("clear");
                phases.set(v);
                Ok(())
            },
            || {
                let mut v = phases.take();
                v.push("shutdown");
                phases.set(v);
            },
            || {
                let mut v = phases.take();
                v.push("reconstruct");
                phases.set(v);
                Ok(())
            },
        );
        assert!(r.is_ok());
        assert!(ev.load(Ordering::SeqCst));
        let v = phases.into_inner();
        assert_eq!(v, vec!["clear", "shutdown", "reconstruct"]);
    }

    #[test]
    fn reload_stops_on_clear_modules_error() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        let r = Powerline::reload(
            &ev,
            || Err("clear failed".to_string()),
            || panic!("shutdown should not run after clear fail"),
            || panic!("reconstruct should not run after clear fail"),
        );
        assert!(r.is_err());
        // Shutdown event NOT set since reload bailed before shutdown
        assert!(!ev.load(Ordering::SeqCst));
    }

    #[test]
    fn reload_propagates_reconstruct_error() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let ev = std::sync::Arc::new(AtomicBool::new(false));
        let r = Powerline::reload(
            &ev,
            || Ok(()),
            || {},
            || Err("reconstruct failed".to_string()),
        );
        assert!(r.is_err());
        assert!(ev.load(Ordering::SeqCst));
    }

    #[test]
    fn reraise_returns_message_unchanged() {
        // py:362-366
        assert_eq!(reraise("boom"), "boom");
    }

    #[test]
    fn gen_module_attr_getter_routes_through_lookup() {
        // py:370-399
        let getter = gen_module_attr_getter(|module, attr| {
            if module == "powerline.matchers.vim" && attr == "help" {
                Some("vim_help_matcher".to_string())
            } else {
                None
            }
        });
        assert_eq!(
            getter("powerline.matchers.vim", "help"),
            Some("vim_help_matcher".to_string())
        );
        assert_eq!(getter("nonexistent", "attr"), None);
    }
}
