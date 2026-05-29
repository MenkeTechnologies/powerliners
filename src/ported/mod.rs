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
