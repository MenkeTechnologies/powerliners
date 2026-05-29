// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/__init__.py`.
//!
//! Lint-time JSON loader: parses a JSON stream into a tree of
//! `MarkedValue` instances that carry file:line:column metadata for
//! error messages. Used only by `powerline-lint` — the runtime config
//! loader (`lib/config.py:load_json_config`) uses plain `json.load`.
//!
//! Most of the submodule subtree (`composer`, `constructor`, `error`,
//! `events`, `loader`, `markedvalue`, `nodes`, `parser`, `reader`,
//! `resolver`, `scanner`, `tokens`) is unported; the
//! re-implementation of a marked-token YAML/JSON parser is a
//! ~2000-LOC subsystem (the lint phase per PORT_PLAN.md). Until then,
//! `load()` defers to `lib/config.py:load_json_config` for the
//! plain-JSON parse and stamps `haserrors=false` since we lack the
//! marked-token error trail.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lint.markedjson.loader import Loader                                      // py:4

pub mod composer;
pub mod constructor;
pub mod error;
pub mod events;
pub mod loader;
pub mod markedvalue;
pub mod nodes;
pub mod parser;
pub mod reader;
pub mod resolver;
pub mod scanner;
pub mod tokens;

use crate::ported::lib::config::load_json_config;
use serde_json::Value;
use std::path::Path;

/// Port of `load()` from `powerline/lint/markedjson/__init__.py:7`.
///
/// Parse JSON value and produce the corresponding object.
///
/// :return:
///     `(object, hadproblem)` — the tuple is reversed in the Rust port
///     so the value is first and the error flag is second (clearer at
///     call sites). Python returns `(r, loader.haserrors)`.
///
/// Until the full `markedjson` parser ports, this dispatches to
/// `lib/config.load_json_config` and reports `hadproblem = err.is_some()`.
pub fn load<P: AsRef<Path>>(stream: P) -> (Option<Value>, bool) {
    // py:7  def load(stream, Loader=Loader)
    // py:14  loader = Loader(stream)
    // py:15  try:
    // py:16  r = loader.get_single_data()
    // py:17  return r, loader.haserrors
    // py:18  finally:
    // py:19  loader.dispose()
    match load_json_config(stream) {
        Ok(v) => (Some(v), false),
        Err(_) => (None, true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_json(content: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-markedjson-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn load_returns_value_and_no_error() {
        let p = tmp_json(r#"{"a": 1}"#);
        let (v, err) = load(&p);
        assert!(!err);
        assert!(v.is_some());
        assert_eq!(v.unwrap()["a"], 1);
        std::fs::remove_file(&p).ok();
    }

    #[test]
    fn load_reports_error_on_bad_json() {
        let p = tmp_json(r#"{ this isn't json"#);
        let (v, err) = load(&p);
        assert!(err);
        assert!(v.is_none());
        std::fs::remove_file(&p).ok();
    }
}
