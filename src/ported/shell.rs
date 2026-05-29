// vim:fileencoding=utf-8:noet
//! Port of `powerline/shell.py`.
//!
//! Shell-specific Powerline bindings (the entry point for bash, zsh,
//! fish, tcsh segment rendering invoked via `powerline-render-shell`).
//! The Python `ShellPowerline` class inherits from `Powerline` and
//! reads its config overlays + path list directly from the parsed
//! CLI args namespace (`argparse.Namespace`) rather than from env
//! vars (cf. pdb.rs).
//!
//! Rust port surfaces:
//!   - `ShellArgs` struct mirroring the CLI args namespace fields
//!     the class reaches into (`ext`, `renderer_module`,
//!     `config_override`, `theme_override`, `config_path`)
//!   - `ShellPowerline::init/load_main_config/load_theme_config/
//!     get_config_paths/get_local_themes/do_setup` instance methods
//!
//! The Powerline base + `do_setup`'s actual side effect on `obj`
//! (`obj.powerline = self`) is structurally surfaced; the broader
//! base render flow is unported.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline import Powerline                                                          // py:4
// from powerline.lib.dict import mergedicts                                                 // py:5

use crate::ported::lib::dict::mergedicts;
use serde_json::{Map, Value};

/// Subset of the parsed `argparse.Namespace` the Python class reads.
///
/// Python: `self.args` is the full argparse namespace. The Rust port
/// names only the four fields actually used by ShellPowerline's
/// methods.
#[derive(Debug, Clone, Default)]
pub struct ShellArgs {
    /// Python: `args.ext` — a list; the class uses `ext[0]`.
    pub ext: Vec<String>,
    /// Python: `args.renderer_module`.
    pub renderer_module: Option<String>,
    /// Python: `args.config_override` — optional dict overlay.
    pub config_override: Option<Map<String, Value>>,
    /// Python: `args.theme_override` — optional dict keyed by theme
    /// name → per-theme overlay.
    pub theme_override: Option<Map<String, Value>>,
    /// Python: `args.config_path` — list of paths or None.
    pub config_path: Vec<String>,
}

/// Port of `class ShellPowerline(Powerline)` from
/// `powerline/shell.py:8`.
pub struct ShellPowerline {
    /// Python: `self.args` — set by `init()` (py:11).
    pub args: ShellArgs,
}

impl ShellPowerline {
    /// Port of `ShellPowerline.init()` from
    /// `powerline/shell.py:10`.
    ///
    /// Python stashes the args namespace and calls
    /// `super().init(args.ext[0], args.renderer_module, **kwargs)`.
    /// Rust port stores the args; the base init is stubbed. Returns
    /// `(ext, renderer_module)` for the caller's base init use.
    pub fn init(args: ShellArgs) -> (Self, String, Option<String>) {
        // py:11  self.args = args
        // py:12  super().init(args.ext[0], args.renderer_module, ...)
        let ext_first = args.ext.first().cloned().unwrap_or_default();
        let renderer_module = args.renderer_module.clone();
        (Self { args }, ext_first, renderer_module)
    }

    /// Port of `ShellPowerline.load_main_config()` from
    /// `powerline/shell.py:14`.
    ///
    /// Overlays `args.config_override` onto the base config dict.
    pub fn load_main_config(&self, base: &mut Map<String, Value>) {
        // py:15  r = super().load_main_config()  (caller-supplied via base)
        // py:16-17  if self.args.config_override: mergedicts(r, ...)
        if let Some(overlay) = &self.args.config_override {
            mergedicts(base, overlay.clone(), false);
        }
    }

    /// Port of `ShellPowerline.load_theme_config()` from
    /// `powerline/shell.py:20`.
    ///
    /// Overlays the matching entry from `args.theme_override` onto
    /// the base theme config.
    pub fn load_theme_config(&self, name: &str, base: &mut Map<String, Value>) {
        // py:21  r = super().load_theme_config(name)  (caller-supplied)
        // py:22-23  if self.args.theme_override and name in self.args.theme_override:
        if let Some(themes) = &self.args.theme_override {
            if let Some(overlay) = themes.get(name).and_then(|v| v.as_object()) {
                mergedicts(base, overlay.clone(), false);
            }
        }
    }

    /// Port of `ShellPowerline.get_config_paths()` from
    /// `powerline/shell.py:26`.
    ///
    /// Returns `args.config_path` if set, else falls back to the base
    /// (returned as `None` here — caller composes with their base).
    pub fn get_config_paths(&self) -> Option<&[String]> {
        // py:27  return self.args.config_path or super().get_config_paths()
        if self.args.config_path.is_empty() {
            None
        } else {
            Some(&self.args.config_path)
        }
    }

    /// Port of `ShellPowerline.get_local_themes()` from
    /// `powerline/shell.py:29`.
    ///
    /// Returns a dict of `{key: {'config': loaded_theme_config}}`
    /// pairs. Each loaded theme starts as an empty Map and is
    /// populated via `load_theme_config(val, &mut config)`.
    pub fn get_local_themes(&self, local_themes: &Map<String, Value>) -> Map<String, Value> {
        // py:30-31  if not local_themes: return {}
        if local_themes.is_empty() {
            return Map::new();
        }
        // py:33-36  dict((key, {'config': load_theme_config(val)}) for ...)
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

    /// Port of `ShellPowerline.do_setup()` from
    /// `powerline/shell.py:38`.
    ///
    /// Python: `obj.powerline = self`. The Rust port mutates the
    /// passed-in Map to mirror the attribute-assignment semantics.
    pub fn do_setup(&self, obj: &mut Map<String, Value>) {
        // py:39  obj.powerline = self
        obj.insert(
            "powerline".to_string(),
            Value::String("<ShellPowerline>".into()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn init_stores_args_and_returns_ext_first() {
        let args = ShellArgs {
            ext: vec!["shell".to_string()],
            renderer_module: Some("bash".to_string()),
            ..Default::default()
        };
        let (sp, ext, renderer) = ShellPowerline::init(args);
        assert_eq!(sp.args.ext, vec!["shell".to_string()]);
        assert_eq!(ext, "shell");
        assert_eq!(renderer, Some("bash".to_string()));
    }

    #[test]
    fn init_empty_ext_returns_empty_string() {
        let args = ShellArgs::default();
        let (_sp, ext, renderer) = ShellPowerline::init(args);
        assert_eq!(ext, "");
        assert_eq!(renderer, None);
    }

    #[test]
    fn load_main_config_overlays_config_override() {
        // py:16-17  if args.config_override: mergedicts(r, ...)
        let mut override_map = Map::new();
        override_map.insert("foo".to_string(), Value::from(1));
        let args = ShellArgs {
            ext: vec!["shell".to_string()],
            config_override: Some(override_map),
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);
        let mut base: Map<String, Value> = Map::new();
        base.insert("bar".to_string(), Value::from(2));
        sp.load_main_config(&mut base);
        assert_eq!(base.get("foo"), Some(&Value::from(1)));
        assert_eq!(base.get("bar"), Some(&Value::from(2)));
    }

    #[test]
    fn load_main_config_no_override_leaves_base() {
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let mut base: Map<String, Value> = Map::new();
        base.insert("k".to_string(), Value::from(7));
        sp.load_main_config(&mut base);
        assert_eq!(base.get("k"), Some(&Value::from(7)));
        assert_eq!(base.len(), 1);
    }

    #[test]
    fn load_theme_config_overlays_matching_theme() {
        // py:22-23  if name in theme_override: mergedicts(r, theme_override[name])
        let mut overlay = Map::new();
        overlay.insert("foo".to_string(), Value::String("bar".into()));
        let mut themes = Map::new();
        themes.insert("default".to_string(), Value::Object(overlay));
        let args = ShellArgs {
            theme_override: Some(themes),
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);
        let mut base: Map<String, Value> = Map::new();
        sp.load_theme_config("default", &mut base);
        assert_eq!(base.get("foo"), Some(&Value::String("bar".into())));
    }

    #[test]
    fn load_theme_config_ignores_non_matching_theme() {
        let mut overlay = Map::new();
        overlay.insert("foo".to_string(), Value::String("bar".into()));
        let mut themes = Map::new();
        themes.insert("default".to_string(), Value::Object(overlay));
        let args = ShellArgs {
            theme_override: Some(themes),
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);
        let mut base: Map<String, Value> = Map::new();
        sp.load_theme_config("other", &mut base);
        assert!(base.get("foo").is_none());
    }

    #[test]
    fn load_theme_config_no_overrides_leaves_base() {
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let mut base: Map<String, Value> = Map::new();
        base.insert("untouched".to_string(), Value::from(1));
        sp.load_theme_config("any", &mut base);
        assert_eq!(base.get("untouched"), Some(&Value::from(1)));
    }

    #[test]
    fn get_config_paths_returns_args_value_when_set() {
        let args = ShellArgs {
            config_path: vec!["/etc/powerline".to_string(), "/usr/share".to_string()],
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);
        let paths = sp.get_config_paths().unwrap();
        assert_eq!(
            paths,
            &["/etc/powerline".to_string(), "/usr/share".to_string()]
        );
    }

    #[test]
    fn get_config_paths_returns_none_when_args_empty() {
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        assert!(sp.get_config_paths().is_none());
    }

    #[test]
    fn get_local_themes_empty_input_returns_empty() {
        // py:30-31  if not local_themes: return {}
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let empty = Map::new();
        let result = sp.get_local_themes(&empty);
        assert!(result.is_empty());
    }

    #[test]
    fn get_local_themes_wraps_each_value_in_config_key() {
        // py:33-36  dict((key, {'config': load_theme_config(val)}) for ...)
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let mut input = Map::new();
        input.insert("matcher_a".to_string(), Value::String("theme_a".into()));
        input.insert("matcher_b".to_string(), Value::String("theme_b".into()));
        let result = sp.get_local_themes(&input);
        assert_eq!(result.len(), 2);
        // Each entry shape: {key: {'config': {...}}}
        for (_k, v) in &result {
            assert!(v.as_object().unwrap().contains_key("config"));
        }
    }

    #[test]
    fn get_local_themes_loaded_config_picks_up_theme_override() {
        // load_theme_config is invoked per-key with the value name as
        // the theme name; the override should apply.
        let mut overlay = Map::new();
        overlay.insert("seg".to_string(), Value::String("custom".into()));
        let mut themes = Map::new();
        themes.insert("theme_a".to_string(), Value::Object(overlay));
        let args = ShellArgs {
            theme_override: Some(themes),
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);
        let mut input = Map::new();
        input.insert("matcher_a".to_string(), Value::String("theme_a".into()));
        let result = sp.get_local_themes(&input);
        let entry = result.get("matcher_a").unwrap().as_object().unwrap();
        let config = entry.get("config").unwrap().as_object().unwrap();
        assert_eq!(config.get("seg"), Some(&Value::String("custom".into())));
    }

    #[test]
    fn do_setup_assigns_powerline_attribute() {
        // py:39  obj.powerline = self
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let mut obj = Map::new();
        sp.do_setup(&mut obj);
        assert!(obj.contains_key("powerline"));
    }

    #[test]
    fn shell_args_default_has_empty_fields() {
        let a = ShellArgs::default();
        assert!(a.ext.is_empty());
        assert!(a.renderer_module.is_none());
        assert!(a.config_override.is_none());
        assert!(a.theme_override.is_none());
        assert!(a.config_path.is_empty());
    }

    #[test]
    fn load_main_config_overlay_recurses_into_nested_dict() {
        // mergedicts is recursive; nested overlays should merge.
        let mut inner_override = Map::new();
        inner_override.insert("nested".to_string(), Value::from(99));
        let mut override_map = Map::new();
        override_map.insert("outer".to_string(), Value::Object(inner_override));
        let args = ShellArgs {
            config_override: Some(override_map),
            ..Default::default()
        };
        let (sp, _, _) = ShellPowerline::init(args);

        let mut base_inner = Map::new();
        base_inner.insert("other".to_string(), Value::from(1));
        let mut base: Map<String, Value> = Map::new();
        base.insert("outer".to_string(), Value::Object(base_inner));
        sp.load_main_config(&mut base);

        let merged_outer = base.get("outer").unwrap().as_object().unwrap();
        assert_eq!(merged_outer.get("other"), Some(&Value::from(1)));
        assert_eq!(merged_outer.get("nested"), Some(&Value::from(99)));
    }

    #[test]
    fn local_theme_input_with_non_string_value_uses_empty_theme_name() {
        // Defensive: if Map value isn't a string (shouldn't happen
        // for valid configs), treat as empty theme name.
        let args = ShellArgs::default();
        let (sp, _, _) = ShellPowerline::init(args);
        let mut input = Map::new();
        input.insert("matcher".to_string(), json!(42));
        let result = sp.get_local_themes(&input);
        // Should still produce a wrapper entry.
        assert!(result.contains_key("matcher"));
    }
}
