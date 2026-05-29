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
    // py:31  return open(path, 'rb')
    std::fs::read(path)
}

/// Port of `function_name_re` from
/// `powerline/lint/__init__.py:43`.
///
/// Pattern: `^(\w+\.)*[a-zA-Z_]\w*$` — dotted Python identifier
/// path, used for validating segment function references.
pub fn function_name_re() -> &'static Regex {
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
    // py:322  register_common_name('player', '..._player', '_player')
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
    // py:326-330  with open_file: try load; except MarkedError: return
    // (true, None, str(e))
    if !path.exists() {
        return LoadJsonResult {
            hadproblem: true,
            config: None,
            error: Some(format!("Path not found: {}", path.display())),
        };
    }
    // py:328  config, hadproblem = load(F)
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
    let path = d
        .get("path")
        .and_then(|v| v.as_str())
        .map(std::path::PathBuf::from);
    let path = match path {
        Some(p) => p,
        None => return,
    };
    let r = load_json_file(&path);
    // py:338-340  hadproblem / config / error
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
    let mut out = Map::new();
    for (k, v) in d {
        // py:391  dict(v) — shallow copy
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
}
