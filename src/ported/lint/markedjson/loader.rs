// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/loader.py`.
//!
//! Multi-inheritance Loader class that mixes Reader+Scanner+Parser+
//! Composer+Constructor+Resolver. Used by `markedjson.load()` to parse
//! a JSON/YAML stream into a MarkedValue tree with file:line:column
//! provenance for the linter.
//!
//! Upstream relies on Python's MRO to dispatch each role's __init__.
//! Rust has no multi-inheritance; the port models the Loader as a
//! struct embedding the role-specific state. The role traits remain
//! stubbed (Reader/Scanner/Parser/Composer/Constructor/Resolver) until
//! the full ~2000-LOC markedjson subtree is ported.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lint.markedjson.reader import Reader                                     // py:4
// from powerline.lint.markedjson.scanner import Scanner                                    // py:5
// from powerline.lint.markedjson.parser import Parser                                      // py:6
// from powerline.lint.markedjson.composer import Composer                                  // py:7
// from powerline.lint.markedjson.constructor import Constructor                            // py:8
// from powerline.lint.markedjson.resolver import Resolver                                  // py:9
// from powerline.lint.markedjson.error import echoerr                                      // py:10

/// Port of `class Loader(Reader, Scanner, Parser, Composer,
/// Constructor, Resolver)` from
/// `powerline/lint/markedjson/loader.py:13`.
///
/// Multi-inheritance Loader: combines the six markedjson role classes
/// into a single dispatcher. Until the full marked-token parser
/// subtree is ported, `Loader` is a thin shell carrying the
/// `haserrors` flag — the actual parse dispatches to plain JSON via
/// `markedjson::load()` in the parent module.
pub struct Loader {
    /// The raw stream contents (mirrors Reader's responsibility).
    pub stream: String,
    /// Python: `self.haserrors = False` — py:21.
    pub haserrors: bool,
}

impl Loader {
    /// Port of `Loader.__init__()` from
    /// `powerline/lint/markedjson/loader.py:14`.
    pub fn new(stream: impl Into<String>) -> Self {
        // py:15  Reader.__init__(self, stream)
        // py:16  Scanner.__init__(self)
        // py:17  Parser.__init__(self)
        // py:18  Composer.__init__(self)
        // py:19  Constructor.__init__(self)
        // py:20  Resolver.__init__(self)
        // (All role-init stubs are no-ops until the marked-token parser
        //  subsystem is ported.)
        Self {
            stream: stream.into(),
            haserrors: false, // py:21
        }
    }

    /// Port of `Loader.echoerr()` from
    /// `powerline/lint/markedjson/loader.py:23`.
    ///
    /// Forwards to `markedjson.error.echoerr()` and flips the
    /// `haserrors` flag. The error sink is `stderr` until the linter's
    /// real error UI lands.
    pub fn echoerr(&mut self, msg: &str) {
        // py:24  echoerr(*args, **kwargs)
        eprintln!("powerline-lint: {}", msg);
        // py:25  self.haserrors = True
        self.haserrors = true;
    }

    /// Drain the loader and return the parsed value + the haserrors flag.
    ///
    /// Mirrors the upstream consume pattern used by `markedjson::load()`
    /// at the parent-module level: invoke `get_single_data()` then read
    /// `loader.haserrors`. The Rust port collapses to a `take` shape
    /// since the role-dispatch chain isn't wired.
    pub fn get_single_data(&mut self) -> Option<serde_json::Value> {
        match serde_json::from_str(&self.stream) {
            Ok(v) => Some(v),
            Err(e) => {
                self.echoerr(&format!("JSON parse error: {}", e));
                None
            }
        }
    }

    /// Port of `Loader.dispose()` (inherited from Reader). No-op in the
    /// Rust port since there's no underlying file handle to close.
    pub fn dispose(&mut self) {
        // py:18 (in __init__.py call): loader.dispose() — no-op stub
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_loader_has_no_errors() {
        let l = Loader::new("{}");
        assert!(!l.haserrors);
        assert_eq!(l.stream, "{}");
    }

    #[test]
    fn get_single_data_parses_valid_json() {
        let mut l = Loader::new(r#"{"a": 1}"#);
        let v = l.get_single_data().unwrap();
        assert_eq!(v["a"], 1);
        assert!(!l.haserrors);
    }

    #[test]
    fn get_single_data_flips_haserrors_on_bad_json() {
        let mut l = Loader::new("{ this isn't json");
        let v = l.get_single_data();
        assert!(v.is_none());
        assert!(l.haserrors);
    }

    #[test]
    fn echoerr_sets_haserrors() {
        let mut l = Loader::new("");
        l.echoerr("test error");
        assert!(l.haserrors);
    }

    #[test]
    fn dispose_does_not_panic() {
        let mut l = Loader::new("{}");
        l.dispose();
    }
}
