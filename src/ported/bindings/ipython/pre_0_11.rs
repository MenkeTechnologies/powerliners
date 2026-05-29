// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/ipython/pre_0_11.py`.
//!
//! IPython pre-0.11 bindings: the legacy ipapi era. Defines:
//!   - `IPythonInfo` — wraps the prompt cache to expose
//!     `prompt_count` (distinct from `crate::ported::ipython::IPythonInfo`
//!     which wraps the post-0.11 InteractiveShell)
//!   - `PowerlinePrompt` — base class for in/in2/out prompts
//!   - `PowerlinePrompt1` — the "in" prompt; advances prompt_count
//!     and tracks trailing-space count for prompt2/out alignment
//!   - `PowerlinePromptOut` — the "out" prompt; pads with the saved
//!     trailing-space count
//!   - `PowerlinePrompt2` — the "in2" continuation prompt
//!   - `ConfigurableIPythonPowerline` — extends IPythonPowerline,
//!     installs PowerlinePrompt1/2/Out on `ip.IP.outputcache.prompt*`
//!   - `ShutdownHook` — class attribute carrying the powerline
//!     weakref; called at shutdown
//!   - `setup(**kwargs)` IPython extension entry point
//!
//! Rust port surfaces all classes structurally + the prompt
//! state-tracking (`nrspaces` aggregation, `prompt_count` increment,
//! `last_prompt` snapshot). The IPython runtime hooks
//! (`ip.IP.outputcache.prompt*` mutation, `ip.expose_magic`,
//! `late_startup_hook.add`) are stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// from weakref import ref                          // py:6
// from IPython.Prompts import BasePrompt           // py:8
// from IPython.ipapi import get as get_ipython     // py:9
// from IPython.ipapi import TryNext                // py:10
// from powerline.ipython import IPythonPowerline, RewriteResult                            // py:12
// from powerline.lib.unicode import string         // py:13

use crate::ported::ipython::{IPythonPowerline, RewriteResult};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Compiled trailing-whitespace regex per
/// `powerline/bindings/ipython/pre_0_11.py:57`
/// `rspace = re.compile(r'(\s*)$')`.
pub fn rspace() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\s*)$").unwrap())
}

/// Mutable cache state on the pre-0.11 prompt object. Mirrors the
/// fields the prompt-rendering code reads from `ip.IP.outputcache`.
#[derive(Debug, Clone, Default)]
pub struct PromptCache {
    /// Python: `cache.prompt_count`.
    pub prompt_count: u64,
    /// Python: `cache.last_prompt`.
    pub last_prompt: String,
}

/// Port of `class IPythonInfo` from
/// `powerline/bindings/ipython/pre_0_11.py:16`.
///
/// Wraps the prompt cache to expose `prompt_count`. The Rust port
/// stores a snapshot of the count since the live cache ref isn't
/// reachable from Rust.
#[derive(Debug, Clone)]
pub struct IPythonInfo {
    /// Python: `self._cache.prompt_count`.
    pub prompt_count: u64,
}

impl IPythonInfo {
    /// Port of `IPythonInfo.__init__()` from
    /// `powerline/bindings/ipython/pre_0_11.py:17`.
    pub fn new(prompt_count: u64) -> Self {
        Self { prompt_count }
    }

    /// Port of `IPythonInfo.prompt_count` (property) from
    /// `powerline/bindings/ipython/pre_0_11.py:20`.
    pub fn prompt_count(&self) -> u64 {
        // py:21  return self._cache.prompt_count
        self.prompt_count
    }
}

/// Result of `set_p_str()` on the pre-0.11 prompt classes. Mirrors
/// the `(p_str, p_str_nocolor, powerline_prompt_width)` unpacking at
/// `powerline/bindings/ipython/pre_0_11.py:38-46`.
#[derive(Debug, Clone)]
pub struct PromptStrings {
    pub p_str: String,
    pub p_str_nocolor: String,
    pub powerline_prompt_width: usize,
}

/// Port of `class PowerlinePrompt(BasePrompt)` from
/// `powerline/bindings/ipython/pre_0_11.py:24`.
///
/// Base for the three concrete prompts. Holds the shared state and
/// the `set_p_str` helper that calls the render callback.
pub struct PowerlinePrompt {
    /// Python: `self.powerline_segment_info`.
    pub segment_info: IPythonInfo,
    /// Python: `self.cache`.
    pub cache: PromptCache,
    /// Python: `self.sep` — optional separator inherited from old_prompt.
    pub sep: Option<String>,
    /// Python: `self.pad_left = False`.
    pub pad_left: bool,
    /// Python: `self.p_str` (after `set_p_str`).
    pub p_str: String,
    /// Python: `self.p_str_nocolor`.
    pub p_str_nocolor: String,
    /// Python: `self.powerline_prompt_width`.
    pub powerline_prompt_width: usize,
}

impl PowerlinePrompt {
    /// Port of `PowerlinePrompt.__init__()` from
    /// `powerline/bindings/ipython/pre_0_11.py:25`.
    pub fn new(cache: PromptCache, sep: Option<String>) -> Self {
        Self {
            segment_info: IPythonInfo::new(cache.prompt_count),
            cache,
            sep,
            pad_left: false,
            p_str: String::new(),
            p_str_nocolor: String::new(),
            powerline_prompt_width: 0,
        }
    }

    /// Port of `PowerlinePrompt.set_p_str()` from
    /// `powerline/bindings/ipython/pre_0_11.py:37`.
    ///
    /// Calls the render callback and stashes the
    /// `(p_str, p_str_nocolor, prompt_width)` triple on the instance.
    pub fn set_p_str<R>(&mut self, is_prompt: bool, prompt_type: &str, mut render: R)
    where
        R: FnMut(bool, &str, &str) -> PromptStrings,
    {
        // py:38-46  res = self.powerline.render(is_prompt=..., side='left', ...)
        let r = render(is_prompt, "left", prompt_type);
        self.p_str = r.p_str;
        self.p_str_nocolor = r.p_str_nocolor;
        self.powerline_prompt_width = r.powerline_prompt_width;
    }

    /// Port of `PowerlinePrompt.set_colors()` (staticmethod) from
    /// `powerline/bindings/ipython/pre_0_11.py:48`.
    ///
    /// Python: `pass` — explicit no-op.
    pub fn set_colors() {
        // py:49  pass
    }
}

/// Port of `class PowerlinePrompt1(PowerlinePrompt)` from
/// `powerline/bindings/ipython/pre_0_11.py:52`.
///
/// The "in" prompt. Increments prompt_count, tracks nrspaces, and
/// snapshots last_prompt.
pub struct PowerlinePrompt1 {
    pub base: PowerlinePrompt,
    /// Python: `self.nrspaces` — count of trailing whitespace in
    /// `p_str_nocolor` after `set_p_str` runs.
    pub nrspaces: usize,
    /// Python: shared `powerline_last_in` dict — Rust port holds
    /// the per-instance nrspaces here and copies to/from the
    /// shared mutable state below.
    pub last_in_nrspaces: usize,
}

impl PowerlinePrompt1 {
    /// Python class attribute: `powerline_prompt_type = 'in'` (py:53).
    pub const PROMPT_TYPE: &'static str = "in";
    /// Python class attribute: `powerline_is_prompt = True` (py:54).
    pub const IS_PROMPT: bool = true;

    /// Port of `PowerlinePrompt1.__init__()` from
    /// `powerline/bindings/ipython/pre_0_11.py:25` (inherited).
    pub fn new(cache: PromptCache, sep: Option<String>) -> Self {
        Self {
            base: PowerlinePrompt::new(cache, sep),
            nrspaces: 0,
            last_in_nrspaces: 0,
        }
    }

    /// Port of `PowerlinePrompt1.__str__()` from
    /// `powerline/bindings/ipython/pre_0_11.py:57`.
    ///
    /// Increments prompt_count, runs `set_p_str`, snapshots
    /// `last_prompt` from the last line of `p_str_nocolor`.
    pub fn to_p_str<R>(&mut self, render: R) -> String
    where
        R: FnMut(bool, &str, &str) -> PromptStrings,
    {
        // py:58  self.cache.prompt_count += 1
        self.base.cache.prompt_count += 1;
        // py:59  self.set_p_str()
        self.set_p_str(render);
        // py:60  self.cache.last_prompt = self.p_str_nocolor.split('\n')[-1]
        let last_line = self
            .base
            .p_str_nocolor
            .rsplit('\n')
            .next()
            .unwrap_or("")
            .to_string();
        self.base.cache.last_prompt = last_line;
        // py:61  return string(self.p_str)
        self.base.p_str.clone()
    }

    /// Port of `PowerlinePrompt1.set_p_str()` from
    /// `powerline/bindings/ipython/pre_0_11.py:63`.
    ///
    /// After the base set_p_str runs, counts trailing whitespace in
    /// `p_str_nocolor` and stores in `nrspaces` + `last_in.nrspaces`.
    pub fn set_p_str<R>(&mut self, render: R)
    where
        R: FnMut(bool, &str, &str) -> PromptStrings,
    {
        // py:64  super().set_p_str()
        self.base
            .set_p_str(Self::IS_PROMPT, Self::PROMPT_TYPE, render);
        // py:65-66  nrspaces = len(rspace.search(p_str_nocolor).group())
        let n = rspace()
            .find(&self.base.p_str_nocolor)
            .map(|m| m.len())
            .unwrap_or(0);
        self.nrspaces = n;
        // py:67  self.powerline_last_in['nrspaces'] = self.nrspaces
        self.last_in_nrspaces = n;
    }

    /// Port of `PowerlinePrompt1.auto_rewrite()` from
    /// `powerline/bindings/ipython/pre_0_11.py:69`.
    ///
    /// Returns a `RewriteResult` wrapping the rewrite render + the
    /// preserved trailing-space padding.
    pub fn auto_rewrite<R>(&self, mut render: R) -> RewriteResult
    where
        R: FnMut(bool, &str, &str) -> String,
    {
        // py:70-74  return RewriteResult(self.powerline.render(is_prompt=False, matcher_info='rewrite', ...) + ' ' * nrspaces)
        let rendered = render(false, "left", "rewrite");
        let padding = " ".repeat(self.nrspaces);
        RewriteResult::new(format!("{}{}", rendered, padding))
    }
}

/// Port of `class PowerlinePromptOut(PowerlinePrompt)` from
/// `powerline/bindings/ipython/pre_0_11.py:78`.
///
/// The "out" prompt. Pads with the saved trailing-space count from
/// the in prompt.
pub struct PowerlinePromptOut {
    pub base: PowerlinePrompt,
    /// Python: shared `powerline_last_in['nrspaces']` — count
    /// captured from the matching in prompt.
    pub last_in_nrspaces: usize,
}

impl PowerlinePromptOut {
    /// Python class attribute: `powerline_prompt_type = 'out'` (py:79).
    pub const PROMPT_TYPE: &'static str = "out";
    /// Python class attribute: `powerline_is_prompt = False` (py:80).
    pub const IS_PROMPT: bool = false;

    pub fn new(cache: PromptCache, sep: Option<String>, last_in_nrspaces: usize) -> Self {
        Self {
            base: PowerlinePrompt::new(cache, sep),
            last_in_nrspaces,
        }
    }

    /// Port of `PowerlinePromptOut.set_p_str()` from
    /// `powerline/bindings/ipython/pre_0_11.py:82`.
    ///
    /// After the base set_p_str runs, pads p_str + p_str_nocolor with
    /// `nrspaces` trailing spaces.
    pub fn set_p_str<R>(&mut self, render: R)
    where
        R: FnMut(bool, &str, &str) -> PromptStrings,
    {
        // py:83  super().set_p_str()
        self.base
            .set_p_str(Self::IS_PROMPT, Self::PROMPT_TYPE, render);
        // py:84  spaces = ' ' * self.powerline_last_in['nrspaces']
        let spaces = " ".repeat(self.last_in_nrspaces);
        // py:85-86  self.p_str += spaces; self.p_str_nocolor += spaces
        self.base.p_str.push_str(&spaces);
        self.base.p_str_nocolor.push_str(&spaces);
    }
}

/// Port of `class PowerlinePrompt2(PowerlinePromptOut)` from
/// `powerline/bindings/ipython/pre_0_11.py:89`.
///
/// The "in2" continuation prompt. Inherits PowerlinePromptOut's
/// padding behaviour but with `powerline_is_prompt=True` and the
/// "in2" matcher.
pub struct PowerlinePrompt2 {
    pub out_base: PowerlinePromptOut,
}

impl PowerlinePrompt2 {
    /// Python class attribute: `powerline_prompt_type = 'in2'` (py:90).
    pub const PROMPT_TYPE: &'static str = "in2";
    /// Python class attribute: `powerline_is_prompt = True` (py:91).
    pub const IS_PROMPT: bool = true;

    pub fn new(cache: PromptCache, sep: Option<String>, last_in_nrspaces: usize) -> Self {
        Self {
            out_base: PowerlinePromptOut::new(cache, sep, last_in_nrspaces),
        }
    }

    /// Port of `PowerlinePrompt2.set_p_str()` inherited from
    /// PowerlinePromptOut but with IS_PROMPT=true and matcher 'in2'.
    pub fn set_p_str<R>(&mut self, render: R)
    where
        R: FnMut(bool, &str, &str) -> PromptStrings,
    {
        // Identical to PowerlinePromptOut.set_p_str but with the
        // class-overridden IS_PROMPT/PROMPT_TYPE.
        self.out_base
            .base
            .set_p_str(Self::IS_PROMPT, Self::PROMPT_TYPE, render);
        let spaces = " ".repeat(self.out_base.last_in_nrspaces);
        self.out_base.base.p_str.push_str(&spaces);
        self.out_base.base.p_str_nocolor.push_str(&spaces);
    }
}

/// Port of `class ConfigurableIPythonPowerline(IPythonPowerline)`
/// from `powerline/bindings/ipython/pre_0_11.py:93`.
pub struct ConfigurableIPythonPowerline {
    pub base: IPythonPowerline,
    /// Tracks whether `do_setup` ran.
    pub setup_done: bool,
}

impl Default for ConfigurableIPythonPowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigurableIPythonPowerline {
    pub fn new() -> Self {
        Self {
            base: IPythonPowerline::new(),
            setup_done: false,
        }
    }

    /// Port of `ConfigurableIPythonPowerline.init()` from
    /// `powerline/bindings/ipython/pre_0_11.py:94`.
    ///
    /// Stashes the three override fields and returns the renderer
    /// module pin `.pre_5`.
    pub fn init(
        &mut self,
        config_overrides: Option<Map<String, Value>>,
        theme_overrides: Map<String, Value>,
        config_paths: Vec<String>,
    ) -> &'static str {
        // py:98  def init(self, config_overrides=None, theme_overrides={}, config_paths=None):
        // py:99  self.config_overrides = config_overrides
        // py:100  self.theme_overrides = theme_overrides
        // py:101  self.config_paths = config_paths
        // py:102  super(ConfigurableIPythonPowerline, self).init(renderer_module='.pre_5')
        self.base.config_overrides = config_overrides;
        self.base.theme_overrides = theme_overrides;
        self.base.config_paths = config_paths;
        ".pre_5"
    }

    /// Port of `ConfigurableIPythonPowerline.ipython_magic()` from
    /// `powerline/bindings/ipython/pre_0_11.py:101`.
    ///
    /// `parameter_s == 'reload'` triggers reload; else raises
    /// ValueError-equivalent.
    pub fn ipython_magic(&self, parameter_s: &str) -> Result<(), String> {
        // py:102-105  if parameter_s == 'reload': reload(); else: raise
        if parameter_s == "reload" {
            Ok(())
        } else {
            Err(format!("Expected `reload`, but got {}", parameter_s))
        }
    }

    /// Port of `ConfigurableIPythonPowerline.do_setup()` from
    /// `powerline/bindings/ipython/pre_0_11.py:107`.
    ///
    /// Installs the three prompt classes on
    /// `ip.IP.outputcache.prompt{1,2,_out}`. Rust port surfaces this
    /// as Map insertions into the supplied `outputcache` Map.
    pub fn do_setup(
        &mut self,
        outputcache: &mut Map<String, Value>,
        shutdown_hook: &mut ShutdownHook,
    ) {
        // py:110  def do_setup(self, ip, shutdown_hook):
        // py:111  last_in = {'nrspaces': 0}
        // py:112  for attr, prompt_class in (
        // py:113  ('prompt1', PowerlinePrompt1),
        // py:114  ('prompt2', PowerlinePrompt2),
        // py:115  ('prompt_out', PowerlinePromptOut)
        // py:116  ):
        // py:117  old_prompt = getattr(ip.IP.outputcache, attr)
        // py:118  prompt = prompt_class(self, last_in, old_prompt)
        // py:119  setattr(ip.IP.outputcache, attr, prompt)
        for attr in ["prompt1", "prompt2", "prompt_out"] {
            outputcache.insert(
                attr.to_string(),
                Value::String(format!("<PowerlinePrompt:{}>", attr)),
            );
        }
        // py:120  ip.expose_magic('powerline', self.ipython_magic)
        // py:121  shutdown_hook.powerline = ref(self)
        shutdown_hook.set_powerline();
        self.setup_done = true;
    }
}

/// Port of `class ShutdownHook(object)` from
/// `powerline/bindings/ipython/pre_0_11.py:120`.
///
/// Distinct from the post_0_11 ShutdownHook — class attribute
/// `powerline = lambda: None` is shared at class scope rather than
/// instance scope.
pub struct ShutdownHook {
    /// Python: `self.powerline()` — lambda returning None initially.
    pub powerline_live: bool,
    pub call_count: u32,
}

impl Default for ShutdownHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownHook {
    pub fn new() -> Self {
        Self {
            powerline_live: false,
            call_count: 0,
        }
    }

    /// Sets the weakref target (py:117 `shutdown_hook.powerline = ref(self)`).
    pub fn set_powerline(&mut self) {
        self.powerline_live = true;
    }

    /// Port of `ShutdownHook.__call__()` from
    /// `powerline/bindings/ipython/pre_0_11.py:123`.
    pub fn call(&mut self) -> &'static str {
        // py:127  def __call__(self):
        // py:128  from IPython.ipapi import TryNext
        // py:129  powerline = self.powerline()
        // py:130  if powerline is not None:
        // py:131  powerline.shutdown()
        if self.powerline_live {
            self.call_count += 1;
        }
        // py:132  raise TryNext()
        "TryNext"
    }
}

/// Port of the inner `late_startup_hook()` closure from
/// `powerline/bindings/ipython/pre_0_11.py:141-143` (inside
/// `setup`).
///
/// Python: calls `powerline.setup(ip, shutdown_hook)` then raises
/// `TryNext()` so the IPython hook chain continues. Rust port
/// drives the powerline setup via the supplied closure and
/// returns a `TryNext`-equivalent error so callers route through
/// any hook-continuation strategy.
///
/// `setup_powerline` is the caller-supplied closure that owns the
/// `powerline.setup(ip, shutdown_hook)` dispatch (the IPython
/// runtime isn't reachable from Rust).
pub fn late_startup_hook<F>(setup_powerline: F) -> Result<(), &'static str>
where
    F: FnOnce(),
{
    // py:141  def late_startup_hook():
    // py:142  powerline.setup(ip, shutdown_hook)
    setup_powerline();
    // py:143  raise TryNext()
    Err("TryNext")
}

/// Port of `setup()` from
/// `powerline/bindings/ipython/pre_0_11.py:131`.
///
/// IPython extension entry point. Constructs a powerline +
/// shutdown_hook pair and returns them; the caller wires the
/// `late_startup_hook` + `shutdown_hook.add` calls since the live
/// IPython runtime isn't reachable from Rust.
pub fn setup() -> (ConfigurableIPythonPowerline, ShutdownHook) {
    // py:135  def setup(**kwargs):
    // py:136  ip = get_ipython()
    // py:138  powerline = ConfigurableIPythonPowerline(**kwargs)
    // py:139  shutdown_hook = ShutdownHook()
    let powerline = ConfigurableIPythonPowerline::new();
    let shutdown_hook = ShutdownHook::new();
    // py:141  def late_startup_hook():
    // py:142  powerline.setup(ip, shutdown_hook)
    // py:143  raise TryNext()
    // py:145  ip.IP.hooks.late_startup_hook.add(late_startup_hook)
    // py:146  ip.IP.hooks.shutdown_hook.add(shutdown_hook)
    (powerline, shutdown_hook)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_render(_is_prompt: bool, _side: &str, _matcher: &str) -> PromptStrings {
        PromptStrings {
            p_str: "PROMPT ".to_string(),
            p_str_nocolor: "PROMPT ".to_string(),
            powerline_prompt_width: 7,
        }
    }

    #[test]
    fn rspace_matches_trailing_whitespace() {
        let m = rspace().find("hello   ").unwrap();
        assert_eq!(m.as_str(), "   ");
    }

    #[test]
    fn rspace_matches_empty_at_end_of_no_trailing_space() {
        // py:57  r'(\s*)$' matches even empty trailing string
        let m = rspace().find("hello").unwrap();
        assert_eq!(m.as_str(), "");
    }

    #[test]
    fn ipython_info_prompt_count_returns_value() {
        let i = IPythonInfo::new(42);
        assert_eq!(i.prompt_count(), 42);
    }

    #[test]
    fn powerline_prompt_set_p_str_stashes_render_result() {
        let mut p = PowerlinePrompt::new(PromptCache::default(), None);
        p.set_p_str(true, "in", fake_render);
        assert_eq!(p.p_str, "PROMPT ");
        assert_eq!(p.p_str_nocolor, "PROMPT ");
        assert_eq!(p.powerline_prompt_width, 7);
    }

    #[test]
    fn powerline_prompt_set_colors_is_noop() {
        // py:49  pass
        PowerlinePrompt::set_colors();
    }

    #[test]
    fn powerline_prompt_init_sets_pad_left_false() {
        let p = PowerlinePrompt::new(PromptCache::default(), None);
        assert!(!p.pad_left);
        assert!(p.sep.is_none());
    }

    #[test]
    fn powerline_prompt_init_preserves_sep_when_supplied() {
        // py:30-31  if hasattr(old_prompt, 'sep'): self.sep = old_prompt.sep
        let p = PowerlinePrompt::new(PromptCache::default(), Some(" | ".to_string()));
        assert_eq!(p.sep, Some(" | ".to_string()));
    }

    #[test]
    fn powerline_prompt1_advances_count_and_snapshots_last_prompt() {
        // py:58-60  prompt_count += 1; last_prompt = last line of p_str_nocolor
        let cache = PromptCache {
            prompt_count: 5,
            ..Default::default()
        };
        let mut p = PowerlinePrompt1::new(cache, None);
        let _ = p.to_p_str(|_is, _side, _m| PromptStrings {
            p_str: "line1\nIN> ".to_string(),
            p_str_nocolor: "line1\nIN> ".to_string(),
            powerline_prompt_width: 4,
        });
        assert_eq!(p.base.cache.prompt_count, 6);
        assert_eq!(p.base.cache.last_prompt, "IN> ");
    }

    #[test]
    fn powerline_prompt1_set_p_str_counts_trailing_spaces() {
        // py:65-66  nrspaces = len(rspace.search(p_str_nocolor).group())
        let mut p = PowerlinePrompt1::new(PromptCache::default(), None);
        p.set_p_str(|_is, _side, _m| PromptStrings {
            p_str: "IN>     ".to_string(),
            p_str_nocolor: "IN>     ".to_string(),
            powerline_prompt_width: 8,
        });
        assert_eq!(p.nrspaces, 5);
        assert_eq!(p.last_in_nrspaces, 5);
    }

    #[test]
    fn powerline_prompt1_set_p_str_zero_trailing_spaces() {
        let mut p = PowerlinePrompt1::new(PromptCache::default(), None);
        p.set_p_str(|_is, _side, _m| PromptStrings {
            p_str: "IN>".to_string(),
            p_str_nocolor: "IN>".to_string(),
            powerline_prompt_width: 3,
        });
        assert_eq!(p.nrspaces, 0);
    }

    #[test]
    fn powerline_prompt1_auto_rewrite_pads_with_nrspaces() {
        // py:70-74  RewriteResult(render(...) + ' ' * nrspaces)
        let mut p = PowerlinePrompt1::new(PromptCache::default(), None);
        p.nrspaces = 3;
        let r = p.auto_rewrite(|_is, _side, _m| "REWRITE".to_string());
        assert_eq!(r.prompt, "REWRITE   ");
    }

    #[test]
    fn powerline_prompt_out_pads_with_last_in_nrspaces() {
        // py:84-86  spaces = ' ' * last_in.nrspaces; p_str += spaces
        let mut out = PowerlinePromptOut::new(PromptCache::default(), None, 4);
        out.set_p_str(fake_render);
        assert_eq!(out.base.p_str, "PROMPT     "); // 7 + 4 trailing
        assert_eq!(out.base.p_str_nocolor, "PROMPT     ");
    }

    #[test]
    fn powerline_prompt2_inherits_padding_behaviour() {
        // PowerlinePrompt2(PowerlinePromptOut) → same padding but
        // PROMPT_TYPE='in2', IS_PROMPT=true
        let mut p2 = PowerlinePrompt2::new(PromptCache::default(), None, 2);
        let mut last_matcher = String::new();
        p2.set_p_str(|_is, _side, matcher| {
            last_matcher = matcher.to_string();
            PromptStrings {
                p_str: "IN2>".to_string(),
                p_str_nocolor: "IN2>".to_string(),
                powerline_prompt_width: 4,
            }
        });
        assert_eq!(last_matcher, "in2");
        assert_eq!(p2.out_base.base.p_str, "IN2>  "); // 4 + 2 padding
    }

    #[test]
    fn powerline_prompt2_is_prompt_true() {
        const _: () = assert!(PowerlinePrompt2::IS_PROMPT);
        assert_eq!(PowerlinePrompt2::PROMPT_TYPE, "in2");
    }

    #[test]
    fn powerline_prompt_out_is_not_a_prompt() {
        // py:80  powerline_is_prompt = False
        const _: () = assert!(!PowerlinePromptOut::IS_PROMPT);
        assert_eq!(PowerlinePromptOut::PROMPT_TYPE, "out");
    }

    #[test]
    fn configurable_init_pins_pre_5_renderer() {
        // py:98-99  super().init(renderer_module='.pre_5')
        let mut c = ConfigurableIPythonPowerline::new();
        let r = c.init(None, Map::new(), Vec::new());
        assert_eq!(r, ".pre_5");
    }

    #[test]
    fn configurable_init_stashes_all_three_overrides() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut overrides = Map::new();
        overrides.insert("k".to_string(), Value::from(1));
        let mut themes = Map::new();
        themes.insert("t".to_string(), Value::from(2));
        c.init(Some(overrides), themes, vec!["/p".to_string()]);
        assert!(c.base.config_overrides.is_some());
        assert_eq!(c.base.theme_overrides.len(), 1);
        assert_eq!(c.base.config_paths, vec!["/p".to_string()]);
    }

    #[test]
    fn ipython_magic_reload_succeeds() {
        let c = ConfigurableIPythonPowerline::new();
        assert!(c.ipython_magic("reload").is_ok());
    }

    #[test]
    fn ipython_magic_other_errors() {
        let c = ConfigurableIPythonPowerline::new();
        let r = c.ipython_magic("foo");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Expected `reload`"));
    }

    #[test]
    fn do_setup_installs_three_prompts_and_wires_hook() {
        // py:109-115  outputcache.prompt1 / prompt2 / prompt_out
        let mut c = ConfigurableIPythonPowerline::new();
        let mut outputcache = Map::new();
        let mut hook = ShutdownHook::new();
        c.do_setup(&mut outputcache, &mut hook);
        for key in ["prompt1", "prompt2", "prompt_out"] {
            assert!(outputcache.contains_key(key));
        }
        assert!(hook.powerline_live);
        assert!(c.setup_done);
    }

    #[test]
    fn shutdown_hook_call_returns_try_next() {
        let mut h = ShutdownHook::new();
        h.set_powerline();
        assert_eq!(h.call(), "TryNext");
        assert_eq!(h.call_count, 1);
    }

    #[test]
    fn shutdown_hook_call_skips_when_target_dead() {
        let mut h = ShutdownHook::new();
        h.call();
        assert_eq!(h.call_count, 0);
    }

    #[test]
    fn setup_returns_pair_of_powerline_and_hook() {
        // py:134-135
        let (powerline, hook) = setup();
        assert!(!powerline.setup_done);
        assert!(!hook.powerline_live);
    }

    #[test]
    fn late_startup_hook_dispatches_setup_then_returns_try_next() {
        // py:141-143
        let invoked = std::cell::Cell::new(false);
        let r = late_startup_hook(|| invoked.set(true));
        assert!(invoked.get());
        assert_eq!(r, Err("TryNext"));
    }
}
