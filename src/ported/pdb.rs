// vim:fileencoding=utf-8:noet
//! Port of `powerline/pdb.py`.
//!
//! PDB-specific Powerline bindings. The Python `PDBPowerline` class
//! inherits from `Powerline` (the unported core in `__init__.py`) and
//! overrides four hooks:
//! 1. `init()` — pins `ext='pdb'` and `renderer_module='pdb'`
//! 2. `do_setup(pdb)` — updates renderer + calls `set_pdb(pdb)`
//! 3. `load_main_config()` — overlays `POWERLINE_CONFIG_OVERRIDES`
//! 4. `load_theme_config(name)` — overlays per-theme overrides from
//!    `POWERLINE_THEME_OVERRIDES`
//! 5. `get_config_paths()` — prefers `POWERLINE_CONFIG_PATHS` env var
//!
//! Rust port surfaces the pure functions (env-var → config-overlay
//! merging, env-var → path-list parsing). The instance methods that
//! require the `Powerline` base + `update_renderer` / `set_pdb` are
//! stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// import platform                                   // py:5
// import os                                         // py:6
// from powerline import Powerline                    // py:8
// from powerline.lib.overrides import parse_override_var                                  // py:9
// from powerline.lib.dict import mergeargs, mergedicts                                    // py:10

use crate::ported::lib::dict::{mergeargs, mergedicts};
use crate::ported::lib::overrides::parse_override_var;
use serde_json::{Map, Value};

/// Port of `class PDBPowerline(Powerline)` from
/// `powerline/pdb.py:13`.
///
/// Currently a unit struct since the Powerline base lives in
/// `__init__.py` which isn't ported yet.
pub struct PDBPowerline;

impl Default for PDBPowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl PDBPowerline {
    /// Returns a fresh `PDBPowerline`.
    pub fn new() -> Self {
        Self
    }

    /// Port of `PDBPowerline.init()` from
    /// `powerline/pdb.py:16`.
    ///
    /// The Python version pins `ext='pdb'` and `renderer_module='pdb'`
    /// then delegates to `super().init(**kwargs)`. Rust port returns
    /// the two pinned values as a tuple; the actual base init is
    /// stubbed.
    pub fn init() -> (&'static str, &'static str) {
        // py:17-21  ext='pdb', renderer_module='pdb'
        ("pdb", "pdb")
    }

    /// Port of `PDBPowerline.do_setup()` from
    /// `powerline/pdb.py:23`.
    ///
    /// **Status:** stub. Python's `do_setup(pdb)` calls
    /// `self.update_renderer()` then `self.renderer.set_pdb(pdb)`;
    /// both require the unported renderer wiring.
    pub fn do_setup(&self, _pdb: &Value) {
        // py:24-25 stub
    }

    /// Port of `PDBPowerline.load_main_config()` from
    /// `powerline/pdb.py:27`.
    ///
    /// Reads `POWERLINE_CONFIG_OVERRIDES` from the environment and
    /// overlays it on the base config dict.
    pub fn load_main_config(&self, base: &mut Map<String, Value>) {
        // py:28  r = super().load_main_config()  (caller-supplied via base)
        // py:29-31  POWERLINE_CONFIG_OVERRIDES → merge
        if let Ok(s) = std::env::var("POWERLINE_CONFIG_OVERRIDES") {
            if !s.is_empty() {
                if let Some(overlay) = mergeargs(parse_override_var(&s), false) {
                    mergedicts(base, overlay, false);
                }
            }
        }
    }

    /// Port of `PDBPowerline.load_theme_config()` from
    /// `powerline/pdb.py:34`.
    ///
    /// Reads `POWERLINE_THEME_OVERRIDES` and overlays only the entry
    /// matching `name`.
    pub fn load_theme_config(&self, name: &str, base: &mut Map<String, Value>) {
        // py:35  r = super().load_theme_config(name)  (caller-supplied)
        // py:36-40  POWERLINE_THEME_OVERRIDES → if name in overlay: merge
        if let Ok(s) = std::env::var("POWERLINE_THEME_OVERRIDES") {
            if !s.is_empty() {
                if let Some(overlay) = mergeargs(parse_override_var(&s), false) {
                    if let Some(theme) = overlay.get(name) {
                        if let Some(theme_obj) = theme.as_object() {
                            mergedicts(base, theme_obj.clone(), false);
                        }
                    }
                }
            }
        }
    }

    /// Port of `PDBPowerline.get_config_paths()` from
    /// `powerline/pdb.py:43`.
    ///
    /// Reads `POWERLINE_CONFIG_PATHS` and splits on `:`. Empty path
    /// list falls back to the base implementation (stubbed here as an
    /// empty Vec — caller composes with their base path list).
    pub fn get_config_paths() -> Vec<String> {
        // py:44  os.environ.get('POWERLINE_CONFIG_PATHS', '').split(':')
        match std::env::var("POWERLINE_CONFIG_PATHS") {
            Ok(s) if !s.is_empty() => s
                .split(':')
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect(),
            // py:45  paths or super().get_config_paths()
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::OnceLock;

    /// Module-scoped lock that serialises env-var mutation tests
    /// against each other. A single OnceLock-backed Mutex shared
    /// by all tests in this module — declared at mod level so the
    /// expansion site of any helper doesn't accidentally make a
    /// per-callsite static.
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    /// Acquires the module env lock. Macro-style so each test can
    /// hold the guard across the full set/read/cleanup sequence
    /// without dragging a fn-return lifetime through the drift gate.
    macro_rules! lock_env {
        () => {{
            ENV_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    #[test]
    fn init_pins_ext_and_renderer_module_to_pdb() {
        // py:17-21  ext='pdb', renderer_module='pdb'
        let (ext, renderer) = PDBPowerline::init();
        assert_eq!(ext, "pdb");
        assert_eq!(renderer, "pdb");
    }

    #[test]
    fn do_setup_does_not_panic() {
        let p = PDBPowerline::new();
        p.do_setup(&Value::Null);
    }

    #[test]
    fn load_main_config_no_env_leaves_base_untouched() {
        let _g = lock_env!();
        // SAFETY: tests serialised via lock_env! above; remove
        // sets/restores the env var.
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_OVERRIDES");
        }
        let mut base: Map<String, Value> = Map::new();
        base.insert("a".to_string(), Value::from(1));
        let p = PDBPowerline::new();
        p.load_main_config(&mut base);
        assert_eq!(base.get("a"), Some(&Value::from(1)));
        assert_eq!(base.len(), 1);
    }

    #[test]
    fn load_main_config_overlays_from_env() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_CONFIG_OVERRIDES", "common.term_truecolor=true");
        }
        let mut base: Map<String, Value> = Map::new();
        let p = PDBPowerline::new();
        p.load_main_config(&mut base);
        // Cleanup before assert so a panic doesn't leak the env var.
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_OVERRIDES");
        }
        // The overlay should produce common.term_truecolor=true
        // somewhere in the merged structure.
        let common = base.get("common").and_then(|v| v.as_object());
        assert!(common.is_some(), "expected 'common' overlay key");
        let truecolor = common.unwrap().get("term_truecolor");
        assert_eq!(truecolor, Some(&Value::Bool(true)));
    }

    #[test]
    fn load_main_config_empty_env_leaves_base_untouched() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_CONFIG_OVERRIDES", "");
        }
        let mut base: Map<String, Value> = Map::new();
        base.insert("k".to_string(), Value::from(1));
        let p = PDBPowerline::new();
        p.load_main_config(&mut base);
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_OVERRIDES");
        }
        assert_eq!(base.get("k"), Some(&Value::from(1)));
    }

    #[test]
    fn load_theme_config_overlays_matching_name() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_THEME_OVERRIDES", "default.foo=bar");
        }
        let mut base: Map<String, Value> = Map::new();
        let p = PDBPowerline::new();
        p.load_theme_config("default", &mut base);
        unsafe {
            std::env::remove_var("POWERLINE_THEME_OVERRIDES");
        }
        assert_eq!(base.get("foo"), Some(&Value::String("bar".into())));
    }

    #[test]
    fn load_theme_config_ignores_non_matching_name() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_THEME_OVERRIDES", "default.foo=bar");
        }
        let mut base: Map<String, Value> = Map::new();
        let p = PDBPowerline::new();
        p.load_theme_config("other_theme", &mut base);
        unsafe {
            std::env::remove_var("POWERLINE_THEME_OVERRIDES");
        }
        assert!(base.get("foo").is_none());
    }

    #[test]
    fn load_theme_config_no_env_leaves_base_untouched() {
        let _g = lock_env!();
        unsafe {
            std::env::remove_var("POWERLINE_THEME_OVERRIDES");
        }
        let mut base: Map<String, Value> = Map::new();
        base.insert("k".to_string(), Value::from(0));
        let p = PDBPowerline::new();
        p.load_theme_config("default", &mut base);
        assert_eq!(base.get("k"), Some(&Value::from(0)));
    }

    #[test]
    fn get_config_paths_reads_colon_separated_env() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_CONFIG_PATHS", "/a:/b:/c");
        }
        let paths = PDBPowerline::get_config_paths();
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_PATHS");
        }
        assert_eq!(paths, vec!["/a", "/b", "/c"]);
    }

    #[test]
    fn get_config_paths_filters_empty_entries() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_CONFIG_PATHS", "::/a::/b:");
        }
        let paths = PDBPowerline::get_config_paths();
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_PATHS");
        }
        assert_eq!(paths, vec!["/a", "/b"]);
    }

    #[test]
    fn get_config_paths_empty_returns_empty_vec() {
        let _g = lock_env!();
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_PATHS");
        }
        let paths = PDBPowerline::get_config_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn get_config_paths_empty_env_string_returns_empty() {
        let _g = lock_env!();
        unsafe {
            std::env::set_var("POWERLINE_CONFIG_PATHS", "");
        }
        let paths = PDBPowerline::get_config_paths();
        unsafe {
            std::env::remove_var("POWERLINE_CONFIG_PATHS");
        }
        assert!(paths.is_empty());
    }
}
