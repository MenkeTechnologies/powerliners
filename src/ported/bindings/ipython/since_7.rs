// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/ipython/since_7.py`.
//!
//! IPython 7+ prompt-toolkit 3 era bindings. Structurally parallel
//! to `bindings/ipython/since_5.py` (the prompt-tokens cache + the
//! `ConfigurableIPythonPowerline` config-overlay extraction). Two
//! deltas:
//!
//! 1. Renderer module pin is `'.since_7'` (not `'.since_5'`)
//! 2. `do_setup(ip, prompts)` takes no `shutdown_hook` argument —
//!    cleanup is wired via `atexit(self.shutdown)` instead of a
//!    weakref into a separate hook object.
//!
//! Rust port surfaces the same shape as the since_5 binding plus an
//! `atexit_register` slot that the caller can use to wire the
//! shutdown callback. The IPython internals
//! (`_make_style_from_name` monkey-patch, `_style` swap,
//! `register_magics`) remain stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:1 (implicit)
// from weakref import ref                                                                  // py:2
// from atexit import register as atexit                                                    // py:3
// from IPython.terminal.prompts import Prompts                                            // py:5
// from pygments.token import Token                                                          // py:6
// from powerline.ipython import IPythonPowerline                                            // py:8
// from powerline.renderers.ipython.since_7 import PowerlinePromptStyle                       // py:9
// from powerline.bindings.ipython.post_0_11 import PowerlineMagics                          // py:10

use crate::ported::bindings::ipython::since_5::PromptKind;
use crate::ported::ipython::IPythonPowerline;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Port of `class ConfigurableIPythonPowerline(IPythonPowerline)`
/// from `powerline/bindings/ipython/since_7.py:13`.
///
/// Identical to the since_5 variant except the renderer module pin
/// is `.since_7` and `do_setup` takes two arguments instead of three.
pub struct ConfigurableIPythonPowerline {
    pub base: IPythonPowerline,
    /// Set when `do_setup` runs to mark that the atexit hook should
    /// fire `self.shutdown` on process exit (py:50 atexit(self.shutdown)).
    pub atexit_registered: bool,
}

impl Default for ConfigurableIPythonPowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurableIPythonPowerline {
    /// Constructs a fresh `ConfigurableIPythonPowerline`.
    pub fn new() -> Self {
        Self {
            base: IPythonPowerline::new(),
            atexit_registered: false,
        }
    }

    /// Port of `ConfigurableIPythonPowerline.init()` from
    /// `powerline/bindings/ipython/since_7.py:14`.
    ///
    /// Reads `ip.config.Powerline` for `config_overrides` /
    /// `theme_overrides` / `config_paths`. Returns the renderer
    /// module pin (`'.since_7'`).
    pub fn init(&mut self, powerline_config: &Map<String, Value>) -> &'static str {
        // py:14  def init(self, ip):
        // py:15  config = ip.config.Powerline
        // py:16  self.config_overrides = config.get('config_overrides')
        if let Some(overrides) = powerline_config
            .get("config_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.config_overrides = Some(overrides.clone());
        }
        // py:17  self.theme_overrides = config.get('theme_overrides', {})
        if let Some(themes) = powerline_config
            .get("theme_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.theme_overrides = themes.clone();
        }
        // py:18  self.config_paths = config.get('config_paths')
        if let Some(paths) = powerline_config
            .get("config_paths")
            .and_then(|v| v.as_array())
        {
            self.base.config_paths = paths
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        // py:19  super(ConfigurableIPythonPowerline, self).init(
        // py:20  renderer_module='.since_7')
        ".since_7"
    }

    /// Port of `ConfigurableIPythonPowerline.do_setup()` from
    /// `powerline/bindings/ipython/since_7.py:22`.
    ///
    /// **Status:** structural surface. Sets
    /// `prompts.powerline = self` and marks the atexit slot. Stubs
    /// the `_make_style_from_name` / `_style` / `register_magics`
    /// monkey-patches.
    pub fn do_setup(&mut self, _ip: &mut Map<String, Value>, prompts: &mut Map<String, Value>) {
        // py:22  def do_setup(self, ip, prompts):
        // py:23  prompts.powerline = self
        prompts.insert(
            "powerline".to_string(),
            Value::String("<ConfigurableIPythonPowerline>".into()),
        );
        // py:25  msfn_missing = ()
        // py:26  saved_msfn = getattr(ip, '_make_style_from_name', msfn_missing)
        // py:28  if hasattr(saved_msfn, 'powerline_original'):
        // py:29  saved_msfn = saved_msfn.powerline_original
        // py:31  def _make_style_from_name(ip, name):
        // py:32  prev_style = saved_msfn(name)
        // py:33  new_style = PowerlinePromptStyle(lambda: prev_style)
        // py:34  return new_style
        // py:36  _make_style_from_name.powerline_original = saved_msfn
        // py:38  if not isinstance(ip._style, PowerlinePromptStyle):
        // py:39  prev_style = ip._style
        // py:40  ip._style = PowerlinePromptStyle(lambda: prev_style)
        // py:42  if not isinstance(saved_msfn, type(self.init)):
        // py:43  _saved_msfn = saved_msfn
        // py:44  saved_msfn = lambda: _saved_msfn(ip)
        // py:46  if saved_msfn is not msfn_missing:
        // py:47  ip._make_style_from_name = _make_style_from_name
        // py:49  magics = PowerlineMagics(ip, self)
        // py:50  ip.register_magics(magics)
        // py:52  atexit(self.shutdown)
        self.atexit_registered = true;
    }

    /// Port of the `self.shutdown` callable the atexit hook fires
    /// (delegated to the unported `Powerline` base in Python).
    /// Rust port is a stub that flips the `atexit_registered` flag
    /// to false to mark that shutdown ran.
    pub fn shutdown(&mut self) {
        // py:50  self.shutdown — base method
        self.atexit_registered = false;
    }
}

/// Port of `class PowerlinePrompts(Prompts)` from
/// `powerline/bindings/ipython/since_7.py:53`.
///
/// Structurally identical to the since_5 variant; the only difference
/// at this layer is that the wiring in `__init__` calls
/// `do_setup(shell, self)` without the shutdown_hook argument.
pub struct PowerlinePrompts {
    /// Python: `self.shell.execution_count` snapshot.
    pub shell_execution_count: u64,
    /// Python: `self.last_output_count`.
    pub last_output_count: Option<u64>,
    /// Python: `self.last_output` cache keyed by prompt name.
    pub last_output: HashMap<String, Vec<(String, String)>>,
}

impl Default for PowerlinePrompts {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PowerlinePrompts {
    /// Port of `PowerlinePrompts.__init__()` from
    /// `powerline/bindings/ipython/since_7.py:56`.
    pub fn new(shell_execution_count: u64) -> Self {
        Self {
            shell_execution_count,
            last_output_count: None,
            last_output: HashMap::new(),
        }
    }

    /// Port of the four `{prompt}_prompt_tokens` methods Python
    /// generates via `exec` at py:63-77.
    ///
    /// Returns the cached prompt-token stream, regenerating via
    /// `render(side, matcher_info, execution_count)` when the
    /// execution count advances or the cache lacks an entry.
    pub fn prompt_tokens<R>(&mut self, prompt: PromptKind, mut render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        // py:65-68  if last_output_count != shell.execution_count: clear
        if self.last_output_count != Some(self.shell_execution_count) {
            self.last_output.clear();
            self.last_output_count = Some(self.shell_execution_count);
        }
        let key = prompt.cache_key();
        // py:69-75  if key not in last_output: render and stash
        if !self.last_output.contains_key(key) {
            let mut tokens = render("left", prompt.matcher_info(), self.shell_execution_count);
            // py:73  + [(Token.Generic.Prompt, " ")]
            tokens.push(("Token.Generic.Prompt".to_string(), " ".to_string()));
            self.last_output.insert(key.to_string(), tokens);
        }
        // py:77  return last_output[key]
        self.last_output[key].clone()
    }

    /// Convenience wrapper for the `in` prompt-tokens method.
    pub fn in_prompt_tokens<R>(&mut self, render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        self.prompt_tokens(PromptKind::In, render)
    }

    /// Convenience wrapper for the `continuation` prompt-tokens method.
    pub fn continuation_prompt_tokens<R>(&mut self, render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        self.prompt_tokens(PromptKind::Continuation, render)
    }

    /// Convenience wrapper for the `rewrite` prompt-tokens method.
    pub fn rewrite_prompt_tokens<R>(&mut self, render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        self.prompt_tokens(PromptKind::Rewrite, render)
    }

    /// Convenience wrapper for the `out` prompt-tokens method.
    pub fn out_prompt_tokens<R>(&mut self, render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        self.prompt_tokens(PromptKind::Out, render)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn init_returns_since_7_renderer_module() {
        // py:19-20  super().init(renderer_module='.since_7')
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        assert_eq!(c.init(&cfg), ".since_7");
    }

    #[test]
    fn init_reads_config_overrides_from_powerline_config() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut overrides = Map::new();
        overrides.insert("k".to_string(), json!(1));
        let mut cfg = Map::new();
        cfg.insert(
            "config_overrides".to_string(),
            Value::Object(overrides.clone()),
        );
        c.init(&cfg);
        assert_eq!(
            c.base.config_overrides.unwrap().get("k"),
            Some(&Value::from(1))
        );
    }

    #[test]
    fn init_reads_theme_overrides_from_powerline_config() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut themes = Map::new();
        themes.insert("t".to_string(), json!({"x": 1}));
        let mut cfg = Map::new();
        cfg.insert("theme_overrides".to_string(), Value::Object(themes.clone()));
        c.init(&cfg);
        assert_eq!(c.base.theme_overrides.get("t"), themes.get("t"));
    }

    #[test]
    fn init_reads_config_paths_from_powerline_config() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut cfg = Map::new();
        cfg.insert("config_paths".to_string(), json!(["/x", "/y"]));
        c.init(&cfg);
        assert_eq!(
            c.base.config_paths,
            vec!["/x".to_string(), "/y".to_string()]
        );
    }

    #[test]
    fn init_missing_keys_leaves_base_attrs_empty() {
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        c.init(&cfg);
        assert!(c.base.config_overrides.is_none());
        assert!(c.base.theme_overrides.is_empty());
        assert!(c.base.config_paths.is_empty());
    }

    #[test]
    fn do_setup_attaches_powerline_to_prompts() {
        // py:23  prompts.powerline = self
        let mut c = ConfigurableIPythonPowerline::new();
        let mut ip = Map::new();
        let mut prompts = Map::new();
        c.do_setup(&mut ip, &mut prompts);
        assert!(prompts.contains_key("powerline"));
    }

    #[test]
    fn do_setup_marks_atexit_registered() {
        // py:50  atexit(self.shutdown)
        let mut c = ConfigurableIPythonPowerline::new();
        assert!(!c.atexit_registered);
        let mut ip = Map::new();
        let mut prompts = Map::new();
        c.do_setup(&mut ip, &mut prompts);
        assert!(c.atexit_registered);
    }

    #[test]
    fn shutdown_clears_atexit_flag() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut ip = Map::new();
        let mut prompts = Map::new();
        c.do_setup(&mut ip, &mut prompts);
        assert!(c.atexit_registered);
        c.shutdown();
        assert!(!c.atexit_registered);
    }

    #[test]
    fn prompt_tokens_caches_within_same_execution_count() {
        // py:69  if key not in last_output → render only once per key
        let mut p = PowerlinePrompts::new(1);
        let mut render_calls = 0;
        let mut render = |_side: &str, _matcher: &str, _count: u64| {
            render_calls += 1;
            vec![("Generic".to_string(), "X".to_string())]
        };
        let a = p.prompt_tokens(PromptKind::In, &mut render);
        let b = p.prompt_tokens(PromptKind::In, &mut render);
        assert_eq!(a, b);
        assert_eq!(render_calls, 1);
    }

    #[test]
    fn prompt_tokens_renders_again_when_execution_count_changes() {
        let mut render_calls = 0;
        let mut render = |_side: &str, _matcher: &str, _count: u64| {
            render_calls += 1;
            Vec::new()
        };
        let mut p = PowerlinePrompts::new(1);
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        p.shell_execution_count = 2;
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        assert_eq!(render_calls, 2);
    }

    #[test]
    fn prompt_tokens_appends_trailing_space_token() {
        // py:73  + [(Token.Generic.Prompt, " ")]
        let mut p = PowerlinePrompts::new(1);
        let tokens = p.prompt_tokens(PromptKind::In, |_s, _m, _c| Vec::new());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, "Token.Generic.Prompt");
        assert_eq!(tokens[0].1, " ");
    }

    #[test]
    fn prompt_tokens_passes_side_left() {
        // py:71  side="left"
        let mut p = PowerlinePrompts::new(1);
        let mut last_side = String::new();
        let mut render = |side: &str, _matcher: &str, _count: u64| {
            last_side = side.to_string();
            Vec::new()
        };
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        assert_eq!(last_side, "left");
    }

    #[test]
    fn prompt_tokens_continuation_uses_in2_matcher() {
        let mut p = PowerlinePrompts::new(1);
        let mut last_matcher = String::new();
        let mut render = |_side: &str, matcher: &str, _count: u64| {
            last_matcher = matcher.to_string();
            Vec::new()
        };
        let _ = p.prompt_tokens(PromptKind::Continuation, &mut render);
        assert_eq!(last_matcher, "in2");
    }

    #[test]
    fn prompt_tokens_distinct_keys_cache_separately() {
        let mut render_calls = 0;
        let mut render = |_side: &str, _matcher: &str, _count: u64| {
            render_calls += 1;
            Vec::new()
        };
        let mut p = PowerlinePrompts::new(1);
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        let _ = p.prompt_tokens(PromptKind::Out, &mut render);
        let _ = p.prompt_tokens(PromptKind::Rewrite, &mut render);
        let _ = p.prompt_tokens(PromptKind::Continuation, &mut render);
        assert_eq!(render_calls, 4);
        assert_eq!(p.last_output.len(), 4);
    }

    #[test]
    fn in_prompt_tokens_helper_works() {
        let mut p = PowerlinePrompts::new(1);
        let tokens = p.in_prompt_tokens(|_s, _m, _c| Vec::new());
        assert_eq!(tokens.len(), 1);
    }

    #[test]
    fn out_prompt_tokens_helper_uses_out_matcher() {
        let mut p = PowerlinePrompts::new(1);
        let mut last_matcher = String::new();
        let mut render = |_side: &str, matcher: &str, _count: u64| {
            last_matcher = matcher.to_string();
            Vec::new()
        };
        let _ = p.out_prompt_tokens(&mut render);
        assert_eq!(last_matcher, "out");
    }

    #[test]
    fn rewrite_prompt_tokens_helper_uses_rewrite_matcher() {
        let mut p = PowerlinePrompts::new(1);
        let mut last_matcher = String::new();
        let mut render = |_side: &str, matcher: &str, _count: u64| {
            last_matcher = matcher.to_string();
            Vec::new()
        };
        let _ = p.rewrite_prompt_tokens(&mut render);
        assert_eq!(last_matcher, "rewrite");
    }

    #[test]
    fn powerline_prompts_init_has_empty_cache() {
        let p = PowerlinePrompts::new(7);
        assert_eq!(p.shell_execution_count, 7);
        assert!(p.last_output_count.is_none());
        assert!(p.last_output.is_empty());
    }
}
