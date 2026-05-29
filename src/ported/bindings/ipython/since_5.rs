// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/ipython/since_5.py`.
//!
//! IPython 5+ prompt-toolkit 2 era bindings. Builds a
//! `PowerlinePrompts` class (subclass of IPython's `Prompts`) that
//! wraps a `ConfigurableIPythonPowerline` instance and emits a
//! Token-stream prompt via `<side>_prompt_tokens` methods for each of
//! `in` / `continuation` / `rewrite` / `out`.
//!
//! Rust port surfaces:
//!   - `ConfigurableIPythonPowerline` config-override extraction
//!   - `PowerlinePrompts` per-prompt cache and the prompt-method
//!     dispatch (the Python `exec`-generated bodies are surfaced as
//!     four named methods)
//!
//! Most of the IPython internals (`ip._style`, `_make_style_from_name`,
//! `register_magics`, `weakref.ref`) require the live IPython runtime
//! and are stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from weakref import ref                                                                  // py:4
// from IPython.terminal.prompts import Prompts                                            // py:6
// from pygments.token import Token                                                          // py:7
// from powerline.ipython import IPythonPowerline                                            // py:9
// from powerline.renderers.ipython.since_5 import PowerlinePromptStyle                       // py:10
// from powerline.bindings.ipython.post_0_11 import PowerlineMagics, ShutdownHook            // py:11

use crate::ported::ipython::IPythonPowerline;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Port of `class ConfigurableIPythonPowerline(IPythonPowerline)`
/// from `powerline/bindings/ipython/since_5.py:14`.
///
/// Extends IPythonPowerline with init() that reads config / theme
/// overrides + config paths from `ip.config.Powerline`.
pub struct ConfigurableIPythonPowerline {
    pub base: IPythonPowerline,
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
        }
    }

    /// Port of `ConfigurableIPythonPowerline.init()` from
    /// `powerline/bindings/ipython/since_5.py:15`.
    ///
    /// Reads `ip.config.Powerline` for `config_overrides` /
    /// `theme_overrides` / `config_paths`. The Python source passes
    /// the live IPython instance; the Rust port takes a parsed config
    /// Map directly.
    ///
    /// Returns the renderer module pin (`".since_5"`).
    pub fn init(&mut self, powerline_config: &Map<String, Value>) -> &'static str {
        // py:16-19  read config_overrides / theme_overrides / config_paths
        if let Some(overrides) = powerline_config
            .get("config_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.config_overrides = Some(overrides.clone());
        }
        if let Some(themes) = powerline_config
            .get("theme_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.theme_overrides = themes.clone();
        }
        if let Some(paths) = powerline_config
            .get("config_paths")
            .and_then(|v| v.as_array())
        {
            self.base.config_paths = paths
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        // py:20-21  super().init(renderer_module='.since_5')
        ".since_5"
    }

    /// Port of `ConfigurableIPythonPowerline.do_setup()` from
    /// `powerline/bindings/ipython/since_5.py:23`.
    ///
    /// **Status:** stub. The Python implementation wires
    /// `prompts.powerline = self`, monkey-patches
    /// `ip._make_style_from_name`, swaps `ip._style` to
    /// `PowerlinePromptStyle`, registers `PowerlineMagics`, and
    /// stores a weakref into `shutdown_hook.powerline`. The Rust
    /// port surfaces just the structural `prompts.powerline = self`
    /// signal by inserting a "powerline" key into the prompts Map.
    pub fn do_setup(
        &self,
        _ip: &mut Map<String, Value>,
        prompts: &mut Map<String, Value>,
        shutdown_hook: &mut Map<String, Value>,
    ) {
        // py:24  prompts.powerline = self
        prompts.insert(
            "powerline".to_string(),
            Value::String("<ConfigurableIPythonPowerline>".into()),
        );
        // py:52-53  shutdown_hook.powerline = ref(self)
        shutdown_hook.insert(
            "powerline".to_string(),
            Value::String("<weakref:ConfigurableIPythonPowerline>".into()),
        );
    }
}

/// Which prompt kind to render. Rust analog of the four iteration
/// strings in the `for prompt in ('in', 'continuation', 'rewrite',
/// 'out'):` block at py:64.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptKind {
    In,
    Continuation,
    Rewrite,
    Out,
}

impl PromptKind {
    /// Returns the lower-case prompt name used as the cache key
    /// (`"in"` / `"continuation"` / `"rewrite"` / `"out"`).
    pub fn cache_key(&self) -> &'static str {
        match self {
            PromptKind::In => "in",
            PromptKind::Continuation => "continuation",
            PromptKind::Rewrite => "rewrite",
            PromptKind::Out => "out",
        }
    }

    /// Returns the `matcher_info` string passed to `powerline.render()`.
    /// Python source: `'in2' if prompt == 'continuation' else prompt`.
    pub fn matcher_info(&self) -> &'static str {
        // py:76  'in2' if prompt == 'continuation' else prompt
        match self {
            PromptKind::Continuation => "in2",
            PromptKind::In => "in",
            PromptKind::Rewrite => "rewrite",
            PromptKind::Out => "out",
        }
    }
}

/// Port of `class PowerlinePrompts(Prompts)` from
/// `powerline/bindings/ipython/since_5.py:55`.
pub struct PowerlinePrompts {
    /// Python: `self.shell` — the IPython InteractiveShell instance.
    /// Stored as the `execution_count` since that's the only field
    /// the prompt methods touch.
    pub shell_execution_count: u64,
    /// Python: `self.last_output_count` — the execution_count at
    /// last cache fill (None initially).
    pub last_output_count: Option<u64>,
    /// Python: `self.last_output` — cache keyed by prompt name.
    pub last_output: HashMap<String, Vec<(String, String)>>,
}

impl Default for PowerlinePrompts {
    fn default() -> Self {
        Self::new(0)
    }
}

impl PowerlinePrompts {
    /// Port of `PowerlinePrompts.__init__()` from
    /// `powerline/bindings/ipython/since_5.py:58`.
    ///
    /// The Python init wires `ShutdownHook`,
    /// `ConfigurableIPythonPowerline`, `do_setup()` and stores
    /// `last_output_count = None; last_output = {}`. Rust port
    /// surfaces just the cache initialization; the wiring lives in
    /// `ConfigurableIPythonPowerline::do_setup`.
    pub fn new(shell_execution_count: u64) -> Self {
        Self {
            shell_execution_count,
            last_output_count: None,
            last_output: HashMap::new(),
        }
    }

    /// Port of the `{prompt}_prompt_tokens()` method body from
    /// `powerline/bindings/ipython/since_5.py:65-78` (Python `exec`-
    /// generated for each of the four prompt kinds).
    ///
    /// Returns the cached prompt-token stream, regenerating via
    /// `render(side, matcher_info, segment_info, execution_count)`
    /// when the execution_count advances or the cache lacks an entry.
    pub fn prompt_tokens<R>(&mut self, prompt: PromptKind, mut render: R) -> Vec<(String, String)>
    where
        R: FnMut(&str, &str, u64) -> Vec<(String, String)>,
    {
        // py:67-70  if last_output_count != shell.execution_count: clear cache
        if self.last_output_count != Some(self.shell_execution_count) {
            self.last_output.clear();
            self.last_output_count = Some(self.shell_execution_count);
        }
        let key = prompt.cache_key();
        // py:71-77  if key not in last_output: render and stash
        if !self.last_output.contains_key(key) {
            let mut tokens = render("left", prompt.matcher_info(), self.shell_execution_count);
            // py:75  + [(Token.Generic.Prompt, " ")]
            tokens.push(("Token.Generic.Prompt".to_string(), " ".to_string()));
            self.last_output.insert(key.to_string(), tokens);
        }
        // py:78  return last_output[key]
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
    fn configurable_ipython_powerline_init_returns_renderer_module() {
        // py:20-21  super().init(renderer_module='.since_5')
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        let renderer = c.init(&cfg);
        assert_eq!(renderer, ".since_5");
    }

    #[test]
    fn init_reads_config_overrides_from_powerline_config() {
        // py:17  self.config_overrides = config.get('config_overrides')
        let mut c = ConfigurableIPythonPowerline::new();
        let mut overrides = Map::new();
        overrides.insert("foo".to_string(), Value::from(1));
        let mut cfg = Map::new();
        cfg.insert(
            "config_overrides".to_string(),
            Value::Object(overrides.clone()),
        );
        c.init(&cfg);
        assert!(c.base.config_overrides.is_some());
        assert_eq!(
            c.base.config_overrides.unwrap().get("foo"),
            Some(&Value::from(1))
        );
    }

    #[test]
    fn init_reads_theme_overrides_from_powerline_config() {
        // py:18  self.theme_overrides = config.get('theme_overrides', {})
        let mut c = ConfigurableIPythonPowerline::new();
        let mut themes = Map::new();
        themes.insert("default".to_string(), json!({"seg": "v"}));
        let mut cfg = Map::new();
        cfg.insert("theme_overrides".to_string(), Value::Object(themes.clone()));
        c.init(&cfg);
        assert_eq!(c.base.theme_overrides.get("default"), themes.get("default"));
    }

    #[test]
    fn init_reads_config_paths_from_powerline_config() {
        // py:19  self.config_paths = config.get('config_paths')
        let mut c = ConfigurableIPythonPowerline::new();
        let mut cfg = Map::new();
        cfg.insert("config_paths".to_string(), json!(["/a", "/b"]));
        c.init(&cfg);
        assert_eq!(
            c.base.config_paths,
            vec!["/a".to_string(), "/b".to_string()]
        );
    }

    #[test]
    fn init_missing_config_keys_leaves_base_attrs_empty() {
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        c.init(&cfg);
        assert!(c.base.config_overrides.is_none());
        assert!(c.base.theme_overrides.is_empty());
        assert!(c.base.config_paths.is_empty());
    }

    #[test]
    fn do_setup_attaches_powerline_to_prompts_and_shutdown_hook() {
        // py:24, py:52-53  prompts.powerline = self; shutdown_hook.powerline = ref(self)
        let c = ConfigurableIPythonPowerline::new();
        let mut ip = Map::new();
        let mut prompts = Map::new();
        let mut shutdown = Map::new();
        c.do_setup(&mut ip, &mut prompts, &mut shutdown);
        assert!(prompts.contains_key("powerline"));
        assert!(shutdown.contains_key("powerline"));
    }

    #[test]
    fn prompt_kind_cache_keys_match_upstream() {
        assert_eq!(PromptKind::In.cache_key(), "in");
        assert_eq!(PromptKind::Continuation.cache_key(), "continuation");
        assert_eq!(PromptKind::Rewrite.cache_key(), "rewrite");
        assert_eq!(PromptKind::Out.cache_key(), "out");
    }

    #[test]
    fn prompt_kind_matcher_info_maps_continuation_to_in2() {
        // py:76  'in2' if prompt == 'continuation' else prompt
        assert_eq!(PromptKind::In.matcher_info(), "in");
        assert_eq!(PromptKind::Continuation.matcher_info(), "in2");
        assert_eq!(PromptKind::Rewrite.matcher_info(), "rewrite");
        assert_eq!(PromptKind::Out.matcher_info(), "out");
    }

    #[test]
    fn prompt_tokens_caches_within_same_execution_count() {
        // py:71  if key not in last_output → render only once per key
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
        // py:67-70  if last_output_count != shell.execution_count: clear
        let mut render_calls = 0;
        let mut render = |_side: &str, _matcher: &str, _count: u64| {
            render_calls += 1;
            vec![("Generic".to_string(), "X".to_string())]
        };
        let mut p = PowerlinePrompts::new(1);
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        // Advance the execution count.
        p.shell_execution_count = 2;
        let _ = p.prompt_tokens(PromptKind::In, &mut render);
        assert_eq!(render_calls, 2);
    }

    #[test]
    fn prompt_tokens_appends_trailing_space_token() {
        // py:75  + [(Token.Generic.Prompt, " ")]
        let mut p = PowerlinePrompts::new(1);
        let tokens = p.prompt_tokens(PromptKind::In, |_s, _m, _c| Vec::new());
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].0, "Token.Generic.Prompt");
        assert_eq!(tokens[0].1, " ");
    }

    #[test]
    fn prompt_tokens_passes_side_left() {
        // py:73  side="left"
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
    fn prompt_tokens_passes_continuation_matcher_as_in2() {
        // py:76  matcher_info='in2' for continuation
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
    fn continuation_prompt_tokens_helper_uses_in2_matcher() {
        let mut p = PowerlinePrompts::new(1);
        let mut last_matcher = String::new();
        let mut render = |_side: &str, matcher: &str, _count: u64| {
            last_matcher = matcher.to_string();
            Vec::new()
        };
        let _ = p.continuation_prompt_tokens(&mut render);
        assert_eq!(last_matcher, "in2");
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
    fn powerline_prompts_init_starts_with_empty_cache() {
        let p = PowerlinePrompts::new(5);
        assert_eq!(p.shell_execution_count, 5);
        assert!(p.last_output_count.is_none());
        assert!(p.last_output.is_empty());
    }
}
