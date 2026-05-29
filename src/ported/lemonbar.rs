// vim:fileencoding=utf-8:noet
//! Port of `powerline/lemonbar.py`.
//!
//! lemonbar-specific Powerline subclass — wires `ext='wm'` +
//! `renderer_module='lemonbar'` into the base Powerline constructor.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline import Powerline                  // py:4
// from powerline.lib.dict import mergedicts        // py:5

use serde_json::{Map, Value};
use std::collections::HashMap;

/// Port of `class LemonbarPowerline(Powerline)` from
/// `powerline/lemonbar.py:8`.
///
/// Subclasses the (still unported) `Powerline` class. The Rust port
/// carries the per-class config defaults + the local-themes adapter;
/// the actual orchestrator behaviour lands when `Powerline` lands.
pub struct LemonbarPowerline;

impl LemonbarPowerline {
    /// Port of `LemonbarPowerline.init()` from
    /// `powerline/lemonbar.py:9`.
    ///
    /// Python: `super().init(ext='wm', renderer_module='lemonbar')`.
    #[allow(non_upper_case_globals)]
    pub const init_ext: &'static str = "wm";
    #[allow(non_upper_case_globals)]
    pub const init_renderer_module: &'static str = "lemonbar";

    /// Port of `LemonbarPowerline.get_encoding` (staticmethod lambda)
    /// from `powerline/lemonbar.py:13`.
    pub fn get_encoding() -> &'static str {
        "utf-8" // py:13  lambda: 'utf-8'
    }

    /// Port of `LemonbarPowerline.get_local_themes()` from
    /// `powerline/lemonbar.py:15`.
    ///
    /// Returns `{}` when local_themes is empty (py:17). Otherwise
    /// builds `{key: {'config': self.load_theme_config(val)}}` —
    /// without the `Powerline.load_theme_config` orchestrator, this
    /// port forwards the val as-is into the config slot.
    pub fn get_local_themes(local_themes: Option<&HashMap<String, Value>>) -> Map<String, Value> {
        // py:16-17  if not local_themes: return {}
        let themes = match local_themes {
            None => return Map::new(),
            Some(t) if t.is_empty() => return Map::new(),
            Some(t) => t,
        };
        // py:19-22  dict((key, {'config': ...}) for ...)
        let mut out = Map::new();
        for (key, val) in themes {
            // load_theme_config stub: pass the value through.
            let mut wrapper = Map::new();
            wrapper.insert("config".to_string(), val.clone());
            out.insert(key.clone(), Value::Object(wrapper));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn init_ext_is_wm() {
        assert_eq!(LemonbarPowerline::init_ext, "wm");
    }

    #[test]
    fn init_renderer_module_is_lemonbar() {
        assert_eq!(LemonbarPowerline::init_renderer_module, "lemonbar");
    }

    #[test]
    fn get_encoding_is_utf8() {
        assert_eq!(LemonbarPowerline::get_encoding(), "utf-8");
    }

    #[test]
    fn get_local_themes_none_returns_empty() {
        let result = LemonbarPowerline::get_local_themes(None);
        assert!(result.is_empty());
    }

    #[test]
    fn get_local_themes_wraps_each_value_in_config_key() {
        let mut input = HashMap::new();
        input.insert("default".to_string(), json!({"theme": "dark"}));
        let result = LemonbarPowerline::get_local_themes(Some(&input));
        assert_eq!(result.len(), 1);
        assert_eq!(result["default"]["config"], json!({"theme": "dark"}));
    }
}
