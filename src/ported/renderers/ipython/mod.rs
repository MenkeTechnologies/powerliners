// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/ipython/__init__.py`.
//!
//! Powerline ipython segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import Theme                // py:4
// from powerline.renderers.shell import PromptRenderer                                     // py:5

pub mod pre_5;
pub mod since_5;
pub mod since_7;

use serde_json::{Map, Value};

/// Port of `class IPythonRenderer(PromptRenderer)` from
/// `powerline/renderers/ipython/__init__.py:8`.
///
/// Powerline ipython segment renderer. Inherits from PromptRenderer
/// (unported); the Rust port carries the IPython-specific methods.
pub struct IPythonRenderer {
    /// Python: `self.segment_info` (inherited from Renderer base).
    pub segment_info: Map<String, Value>,
    /// Python: `self.theme` — the default theme for the "in" matcher
    /// at py:16. Modelled as a JSON Value placeholder since the Theme
    /// class isn't yet ported.
    pub theme: Value,
    /// Python: `self.local_themes` — dict from matcher_info name to
    /// match dict per py:18-25. The match dict carries the `config`
    /// for lazy Theme construction and a `theme` key once constructed.
    pub local_themes: std::collections::HashMap<String, Map<String, Value>>,
    /// Python: `self.theme_config` (inherited) — used as the
    /// main_theme_config when building local themes at py:22.
    pub theme_config: Value,
    /// Python: `self.theme_kwargs` (inherited) — kwargs passed to
    /// Theme construction at py:24.
    pub theme_kwargs: Map<String, Value>,
    /// Set of names whose shutdown was called. Used in lieu of the
    /// Python Theme.shutdown() side effect since the Theme class
    /// isn't yet ported.
    pub shutdown_called: std::sync::Mutex<Vec<String>>,
}

impl Default for IPythonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonRenderer {
    /// Construct a fresh IPythonRenderer.
    pub fn new() -> Self {
        Self {
            segment_info: Map::new(),
            theme: Value::Null,
            local_themes: std::collections::HashMap::new(),
            theme_config: Value::Null,
            theme_kwargs: Map::new(),
            shutdown_called: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Port of `IPythonRenderer.get_segment_info()` from
    /// `powerline/renderers/ipython/__init__.py:10`.
    ///
    /// Returns a copy of `self.segment_info` with `ipython` key
    /// patched in from the supplied per-call segment_info.
    pub fn get_segment_info(&self, segment_info: &Value) -> Map<String, Value> {
        // py:11-13  r = segment_info.copy(); r['ipython'] = segment_info; return r
        let mut r = self.segment_info.clone();
        r.insert("ipython".to_string(), segment_info.clone());
        r
    }

    /// Port of `IPythonRenderer.get_theme()` from
    /// `powerline/renderers/ipython/__init__.py:15-25`.
    ///
    /// Returns the resolved theme for the given `matcher_info` name.
    /// If `matcher_info == "in"` returns `self.theme` (py:16-17).
    /// Otherwise resolves to `local_themes[matcher_info]['theme']`,
    /// constructing it lazily from `config` + `theme_config` +
    /// `theme_kwargs` per py:21-25.
    ///
    /// The Rust port mutates `self.local_themes` to cache the
    /// constructed theme back into the match dict per py:22, requiring
    /// `&mut self`. The Theme class isn't yet ported so the constructed
    /// "theme" is represented as a JSON object snapshot of the
    /// arguments.
    pub fn get_theme(&mut self, matcher_info: &str) -> Value {
        // py:16-17  if matcher_info == 'in': return self.theme
        if matcher_info == "in" {
            return self.theme.clone();
        }
        // py:18  match = self.local_themes[matcher_info]
        let match_entry = match self.local_themes.get(matcher_info) {
            Some(m) => m.clone(),
            None => return Value::Null,
        };
        // py:19-20  return match['theme'] if present
        if let Some(t) = match_entry.get("theme") {
            return t.clone();
        }
        // py:21-25  Construct Theme lazily:
        //   match['theme'] = Theme(
        //       theme_config=match['config'],
        //       main_theme_config=self.theme_config,
        //       **self.theme_kwargs)
        let constructed = serde_json::json!({
            "theme_config": match_entry.get("config").cloned().unwrap_or(Value::Null),
            "main_theme_config": self.theme_config.clone(),
            "theme_kwargs": Value::Object(self.theme_kwargs.clone()),
        });
        // Cache back into local_themes per py:22
        if let Some(m) = self.local_themes.get_mut(matcher_info) {
            m.insert("theme".to_string(), constructed.clone());
        }
        constructed
    }

    /// Port of `IPythonRenderer.shutdown()` from
    /// `powerline/renderers/ipython/__init__.py:27-31`.
    ///
    /// Calls `theme.shutdown()` + every local theme's `shutdown()`
    /// per py:28-31. The Rust port records the shutdown order in
    /// `shutdown_called` for test assertion since `Theme.shutdown` is
    /// not yet ported.
    pub fn shutdown(&self) {
        // py:28  self.theme.shutdown()
        let mut log = self
            .shutdown_called
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        log.push("theme".to_string());
        // py:29-31  for match in self.local_themes.values(): if 'theme' in match: match['theme'].shutdown()
        for (name, match_entry) in &self.local_themes {
            if match_entry.contains_key("theme") {
                log.push(name.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_segment_info_patches_ipython_key() {
        let mut r = IPythonRenderer::new();
        r.segment_info
            .insert("color_scheme".into(), json!("monokai"));
        let payload = json!({"prompt_count": 42});
        let out = r.get_segment_info(&payload);
        assert_eq!(out["ipython"]["prompt_count"], 42);
        assert_eq!(out["color_scheme"], "monokai");
    }

    #[test]
    fn get_theme_in_returns_self_theme() {
        // py:16-17
        let mut r = IPythonRenderer::new();
        r.theme = json!({"name": "default"});
        let t = r.get_theme("in");
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn get_theme_returns_existing_local_theme() {
        // py:19-20  if 'theme' in match: return match['theme']
        let mut r = IPythonRenderer::new();
        let mut entry = Map::new();
        entry.insert("theme".to_string(), json!({"name": "ipython-out"}));
        r.local_themes.insert("out".to_string(), entry);
        let t = r.get_theme("out");
        assert_eq!(t["name"], "ipython-out");
    }

    #[test]
    fn get_theme_constructs_lazy_theme_from_config() {
        // py:21-25
        let mut r = IPythonRenderer::new();
        r.theme_config = json!({"colorscheme": "default"});
        r.theme_kwargs
            .insert("extra".to_string(), json!("kw_value"));
        let mut entry = Map::new();
        entry.insert("config".to_string(), json!({"segments": []}));
        r.local_themes.insert("rewrite".to_string(), entry);

        let t = r.get_theme("rewrite");
        assert_eq!(t["theme_config"]["segments"], json!([]));
        assert_eq!(t["main_theme_config"]["colorscheme"], "default");
        assert_eq!(t["theme_kwargs"]["extra"], "kw_value");
    }

    #[test]
    fn get_theme_caches_constructed_theme() {
        // py:22  match['theme'] = Theme(...)
        let mut r = IPythonRenderer::new();
        let mut entry = Map::new();
        entry.insert("config".to_string(), json!({"a": 1}));
        r.local_themes.insert("rewrite".to_string(), entry);

        let _ = r.get_theme("rewrite");
        let cached = r
            .local_themes
            .get("rewrite")
            .and_then(|m| m.get("theme"))
            .cloned();
        assert!(cached.is_some(), "constructed theme not cached");
    }

    #[test]
    fn shutdown_records_main_theme_first() {
        // py:28
        let r = IPythonRenderer::new();
        r.shutdown();
        let log = r.shutdown_called.lock().unwrap();
        assert_eq!(log[0], "theme");
    }

    #[test]
    fn shutdown_walks_local_themes_with_theme_key() {
        // py:29-31  only matches WITH 'theme' key get shutdown
        let mut r = IPythonRenderer::new();
        let mut with_theme = Map::new();
        with_theme.insert("theme".to_string(), json!({}));
        let no_theme: Map<String, Value> = Map::new();
        r.local_themes.insert("ready".to_string(), with_theme);
        r.local_themes.insert("not_ready".to_string(), no_theme);
        r.shutdown();
        let log = r.shutdown_called.lock().unwrap();
        assert!(log.contains(&"ready".to_string()));
        assert!(!log.contains(&"not_ready".to_string()));
    }

    #[test]
    fn get_theme_missing_matcher_returns_null() {
        let mut r = IPythonRenderer::new();
        let t = r.get_theme("nonexistent");
        assert_eq!(t, Value::Null);
    }
}
