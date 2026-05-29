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
    // py:24-26  if path and os.path.isfile(path): return path
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
    // py:30  config_file += '.json'
    let with_ext = format!("{}.json", config_file);
    let mut found: Vec<std::path::PathBuf> = Vec::new();
    for path in search_paths {
        let candidate = path.join(&with_ext);
        if candidate.is_file() {
            found.push(candidate);
        }
    }
    if found.is_empty() {
        // py:40-43  IOError('Config file not found in search paths ...')
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
        // py:47-49  '{ext}:{prefix}:{message}'
        let effective_prefix = prefix.unwrap_or(&self.prefix);
        format!("{}:{}:{}", self.ext, effective_prefix, message)
    }

    /// Port of the per-level logging methods (debug/info/warn/error/
    /// critical/exception) at py:80-108.
    pub fn log(&self, level: &str, message: &str) {
        // py:80-108  self.logger.log(level, ...)
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
        Powerline {
            ext: ext.to_string(),
            run_once,
            had_logger,
            // py:484
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
        if set_event {
            // py:966  self.shutdown_event.set()
            shutdown_event.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        // py:971-973  config_loader.unregister_functions(...) — deferred
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
    // py:124-127  Logger('powerline'), PowerlineLogger(None, logger, '_fallback_')
    PowerlineLogger::new("powerline", "_fallback_")
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
        let finder = move |p: &str| _find_config_files(&[d_clone.clone()], p);
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
            move |p| _find_config_files(&[d_clone.clone()], p),
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
}
