// vim:fileencoding=utf-8:noet
//! Port of `powerline/ipython.py`.
//!
//! IPython-specific Powerline bindings:
//!   - `IPythonInfo` wraps the IPython shell to expose
//!     `prompt_count = shell.execution_count`
//!   - `RewriteResult` wraps a prompt string with concatenation
//!     semantics (Python `__add__` returns a new RewriteResult)
//!   - `IPythonPowerline` inherits from `Powerline` and pins
//!     `ext='ipython'` + `use_daemon_threads=True`, reads config /
//!     theme overrides from instance attributes (not args/env, cf.
//!     pdb.rs and shell.rs)
//!
//! Rust port surfaces all three structurally. `Powerline` base
//! init / get_config_paths fallback / weakref do_setup require the
//! unported base and are stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline import Powerline                                                          // py:4
// from powerline.lib.dict import mergedicts                                                 // py:5
// from powerline.lib.unicode import string                                                  // py:6

use crate::ported::lib::dict::mergedicts;
use serde_json::{Map, Value};

/// Port of `class IPythonInfo` from
/// `powerline/ipython.py:9`.
///
/// Wraps the IPython shell to expose its `execution_count` as the
/// `prompt_count` property used by the prompt-rendering segments.
#[derive(Debug, Clone)]
pub struct IPythonInfo {
    /// Python: `self._shell.execution_count`. The Rust port stores
    /// the count directly (no live shell ref).
    pub execution_count: u64,
}

impl IPythonInfo {
    /// Port of `IPythonInfo.__init__()` from
    /// `powerline/ipython.py:10`.
    pub fn new(execution_count: u64) -> Self {
        Self { execution_count }
    }

    /// Port of `IPythonInfo.prompt_count` (property) from
    /// `powerline/ipython.py:13`.
    pub fn prompt_count(&self) -> u64 {
        // py:14  return self._shell.execution_count
        self.execution_count
    }
}

/// Port of `class RewriteResult` from
/// `powerline/ipython.py:19`.
///
/// Wraps a string with a concatenation operator. Python's
/// `__add__` returns a new RewriteResult; the right-hand operand is
/// UTF-8 encoded if it's not a str.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteResult {
    /// Python: `self.prompt = string(prompt)` — the wrapped string.
    pub prompt: String,
}

impl RewriteResult {
    /// Port of `RewriteResult.__init__()` from
    /// `powerline/ipython.py:20`.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
        }
    }

    /// Port of `RewriteResult.__add__()` from
    /// `powerline/ipython.py:25`.
    ///
    /// Returns a new RewriteResult with the right-hand string
    /// appended. Python special-cases non-str inputs by attempting
    /// `s.encode('utf-8')`; the Rust port takes any string-like
    /// already as UTF-8.
    pub fn add(&self, s: &str) -> RewriteResult {
        // py:26-31  return RewriteResult(self.prompt + s)
        RewriteResult::new(format!("{}{}", self.prompt, s))
    }
}

impl std::fmt::Display for RewriteResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // py:22  return self.prompt
        write!(f, "{}", self.prompt)
    }
}

/// Port of `class IPythonPowerline(Powerline)` from
/// `powerline/ipython.py:34`.
///
/// Instance attributes used by the methods:
///   - `config_paths`: Optional path list
///   - `config_overrides`: Optional config overlay
///   - `theme_overrides`: dict of name → overlay
pub struct IPythonPowerline {
    /// Python: `self.config_paths` — set externally before init.
    pub config_paths: Vec<String>,
    /// Python: `self.config_overrides` — config overlay.
    pub config_overrides: Option<Map<String, Value>>,
    /// Python: `self.theme_overrides` — keyed by theme name.
    pub theme_overrides: Map<String, Value>,
}

impl Default for IPythonPowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonPowerline {
    /// Constructs an IPythonPowerline with empty config attrs.
    pub fn new() -> Self {
        Self {
            config_paths: Vec::new(),
            config_overrides: None,
            theme_overrides: Map::new(),
        }
    }

    /// Port of `IPythonPowerline.init()` from
    /// `powerline/ipython.py:35`.
    ///
    /// Returns the pinned (ext, use_daemon_threads) values the
    /// Python source passes to `super().init()`.
    pub fn init() -> (&'static str, bool) {
        // py:35  def init(self, **kwargs):
        // py:36  super(IPythonPowerline, self).init(
        // py:37  'ipython',
        // py:38  use_daemon_threads=True,
        // py:39  **kwargs
        // py:40  )
        ("ipython", true)
    }

    /// Port of `IPythonPowerline.get_config_paths()` from
    /// `powerline/ipython.py:42`.
    ///
    /// Returns `self.config_paths` if set, else `None` (base
    /// fallback is the caller's responsibility).
    pub fn get_config_paths(&self) -> Option<&[String]> {
        // py:42  def get_config_paths(self):
        // py:43  if self.config_paths:
        // py:44  return self.config_paths
        // py:45  else:
        // py:46  return super(IPythonPowerline, self).get_config_paths()
        if self.config_paths.is_empty() {
            None
        } else {
            Some(&self.config_paths)
        }
    }

    /// Port of `IPythonPowerline.get_local_themes()` from
    /// `powerline/ipython.py:48`.
    ///
    /// Wraps each `{type: theme_name}` entry into
    /// `{type: {'config': loaded_theme_config}}`.
    pub fn get_local_themes(&self, local_themes: &Map<String, Value>) -> Map<String, Value> {
        // py:49  dict(((type, {'config': load_theme_config(name)}) for ...))
        let mut out = Map::new();
        for (key, val) in local_themes {
            let theme_name = val.as_str().unwrap_or("");
            let mut config = Map::new();
            self.load_theme_config(theme_name, &mut config);
            let mut entry = Map::new();
            entry.insert("config".to_string(), Value::Object(config));
            out.insert(key.clone(), Value::Object(entry));
        }
        out
    }

    /// Port of `IPythonPowerline.load_main_config()` from
    /// `powerline/ipython.py:51`.
    pub fn load_main_config(&self, base: &mut Map<String, Value>) {
        // py:51  def load_main_config(self):
        // py:52  r = super(IPythonPowerline, self).load_main_config()
        // py:53  if self.config_overrides:
        // py:54  mergedicts(r, self.config_overrides)
        if let Some(overlay) = &self.config_overrides {
            mergedicts(base, overlay.clone(), false);
        }
        // py:55  return r
    }

    /// Port of `IPythonPowerline.load_theme_config()` from
    /// `powerline/ipython.py:57`.
    pub fn load_theme_config(&self, name: &str, base: &mut Map<String, Value>) {
        // py:57  def load_theme_config(self, name):
        // py:58  r = super(IPythonPowerline, self).load_theme_config(name)
        // py:59  if name in self.theme_overrides:
        // py:60  mergedicts(r, self.theme_overrides[name])
        if let Some(overlay) = self.theme_overrides.get(name).and_then(|v| v.as_object()) {
            mergedicts(base, overlay.clone(), false);
        }
        // py:61  return r
    }

    /// Port of `IPythonPowerline.do_setup()` from
    /// `powerline/ipython.py:63`.
    ///
    /// Python iterates a list of weakref callables, dereferences each,
    /// and assigns `obj.powerline = self` to each live target. Rust
    /// port mirrors the structural intent: for each entry that is
    /// dereferenceable (non-None Option), insert the "powerline" key.
    pub fn do_setup(&self, wrefs: &mut [Option<Map<String, Value>>]) {
        // py:64-67  for wref in wrefs: obj = wref(); if obj is not None: setattr
        for wref in wrefs.iter_mut() {
            if let Some(obj) = wref.as_mut() {
                obj.insert(
                    "powerline".to_string(),
                    Value::String("<IPythonPowerline>".into()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ipython_info_prompt_count_returns_execution_count() {
        // py:13-14  property: return self._shell.execution_count
        let i = IPythonInfo::new(42);
        assert_eq!(i.prompt_count(), 42);
    }

    #[test]
    fn ipython_info_construct_from_zero() {
        let i = IPythonInfo::new(0);
        assert_eq!(i.prompt_count(), 0);
    }

    #[test]
    fn rewrite_result_constructs_from_string() {
        // py:20  self.prompt = string(prompt)
        let r = RewriteResult::new("hello");
        assert_eq!(r.prompt, "hello");
    }

    #[test]
    fn rewrite_result_str_returns_prompt() {
        // py:22  return self.prompt
        let r = RewriteResult::new("hi");
        assert_eq!(format!("{}", r), "hi");
    }

    #[test]
    fn rewrite_result_add_concatenates() {
        // py:25-31  return RewriteResult(self.prompt + s)
        let r = RewriteResult::new("hello ");
        let r2 = r.add("world");
        assert_eq!(r2.prompt, "hello world");
        // Original is unchanged (functional concat).
        assert_eq!(r.prompt, "hello ");
    }

    #[test]
    fn rewrite_result_add_chain() {
        let r = RewriteResult::new("a").add("b").add("c");
        assert_eq!(r.prompt, "abc");
    }

    #[test]
    fn ipython_powerline_init_pins_ext_and_daemon_threads() {
        // py:36-40  super().init('ipython', use_daemon_threads=True, ...)
        let (ext, daemon) = IPythonPowerline::init();
        assert_eq!(ext, "ipython");
        assert!(daemon);
    }

    #[test]
    fn ipython_powerline_new_has_empty_attrs() {
        let p = IPythonPowerline::new();
        assert!(p.config_paths.is_empty());
        assert!(p.config_overrides.is_none());
        assert!(p.theme_overrides.is_empty());
    }

    #[test]
    fn get_config_paths_returns_some_when_set() {
        let mut p = IPythonPowerline::new();
        p.config_paths.push("/etc/powerline".to_string());
        let paths = p.get_config_paths().unwrap();
        assert_eq!(paths, &["/etc/powerline".to_string()]);
    }

    #[test]
    fn get_config_paths_returns_none_when_empty() {
        let p = IPythonPowerline::new();
        assert!(p.get_config_paths().is_none());
    }

    #[test]
    fn load_main_config_overlays_config_overrides() {
        // py:53-54  if self.config_overrides: mergedicts(r, ...)
        let mut p = IPythonPowerline::new();
        let mut overlay = Map::new();
        overlay.insert("k".to_string(), Value::from(1));
        p.config_overrides = Some(overlay);
        let mut base = Map::new();
        base.insert("orig".to_string(), Value::from(0));
        p.load_main_config(&mut base);
        assert_eq!(base.get("k"), Some(&Value::from(1)));
        assert_eq!(base.get("orig"), Some(&Value::from(0)));
    }

    #[test]
    fn load_main_config_no_overrides_passes_through() {
        let p = IPythonPowerline::new();
        let mut base = Map::new();
        base.insert("k".to_string(), Value::from(7));
        p.load_main_config(&mut base);
        assert_eq!(base.get("k"), Some(&Value::from(7)));
        assert_eq!(base.len(), 1);
    }

    #[test]
    fn load_theme_config_overlays_matching_name() {
        let mut p = IPythonPowerline::new();
        let mut overlay = Map::new();
        overlay.insert("seg".to_string(), Value::String("custom".into()));
        p.theme_overrides
            .insert("default".to_string(), Value::Object(overlay));
        let mut base = Map::new();
        p.load_theme_config("default", &mut base);
        assert_eq!(base.get("seg"), Some(&Value::String("custom".into())));
    }

    #[test]
    fn load_theme_config_ignores_non_matching_name() {
        let mut p = IPythonPowerline::new();
        let mut overlay = Map::new();
        overlay.insert("seg".to_string(), Value::String("x".into()));
        p.theme_overrides
            .insert("default".to_string(), Value::Object(overlay));
        let mut base = Map::new();
        p.load_theme_config("other", &mut base);
        assert!(base.get("seg").is_none());
    }

    #[test]
    fn get_local_themes_wraps_each_value_in_config_key() {
        // py:49  dict(((type, {'config': load_theme_config(name)}) for ...))
        let p = IPythonPowerline::new();
        let mut input = Map::new();
        input.insert("type_a".to_string(), Value::String("theme_a".into()));
        let result = p.get_local_themes(&input);
        let entry = result.get("type_a").unwrap().as_object().unwrap();
        assert!(entry.contains_key("config"));
    }

    #[test]
    fn get_local_themes_empty_input_returns_empty() {
        let p = IPythonPowerline::new();
        let empty = Map::new();
        let result = p.get_local_themes(&empty);
        assert!(result.is_empty());
    }

    #[test]
    fn get_local_themes_applies_theme_override() {
        let mut p = IPythonPowerline::new();
        let mut overlay = Map::new();
        overlay.insert("seg".to_string(), Value::String("v".into()));
        p.theme_overrides
            .insert("theme_a".to_string(), Value::Object(overlay));
        let mut input = Map::new();
        input.insert("matcher".to_string(), Value::String("theme_a".into()));
        let result = p.get_local_themes(&input);
        let entry = result.get("matcher").unwrap().as_object().unwrap();
        let config = entry.get("config").unwrap().as_object().unwrap();
        assert_eq!(config.get("seg"), Some(&Value::String("v".into())));
    }

    #[test]
    fn do_setup_assigns_powerline_to_each_live_wref() {
        // py:64-67  for wref in wrefs: obj = wref(); if obj is not None: setattr
        let p = IPythonPowerline::new();
        let mut wrefs: Vec<Option<Map<String, Value>>> =
            vec![Some(Map::new()), None, Some(Map::new())];
        p.do_setup(&mut wrefs);
        // Live entries get the powerline key.
        assert!(wrefs[0].as_ref().unwrap().contains_key("powerline"));
        assert!(wrefs[2].as_ref().unwrap().contains_key("powerline"));
        // None entry untouched.
        assert!(wrefs[1].is_none());
    }

    #[test]
    fn rewrite_result_eq_compares_prompt() {
        assert_eq!(RewriteResult::new("a"), RewriteResult::new("a"));
        assert_ne!(RewriteResult::new("a"), RewriteResult::new("b"));
    }

    #[test]
    fn get_local_themes_non_string_value_uses_empty_theme_name() {
        let p = IPythonPowerline::new();
        let mut input = Map::new();
        input.insert("k".to_string(), json!(42));
        let result = p.get_local_themes(&input);
        // Still produces a wrapper entry.
        assert!(result.contains_key("k"));
    }
}
