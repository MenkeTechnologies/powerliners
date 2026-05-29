// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/ipython/post_0_11.py`.
//!
//! IPython 0.11+ (pre-5) bindings. Defines:
//!   - `PowerlineMagics` — the `%powerline reload` line-magic
//!   - `ShutdownHook` — registered into `ip.hooks.shutdown_hook`
//!     to call `powerline.shutdown()` at exit
//!   - `PowerlinePromptManager` (only present when
//!     `IPython.core.prompts.PromptManager` is importable; pre-5
//!     IPython) — wraps `Powerline.render()` and unpacks the
//!     `(text, raw, width)` tuple per `is_prompt`/`color`
//!   - `ConfigurableIPythonPowerline` extending IPythonPowerline
//!     with init() / do_setup() that wires the prompt manager
//!   - `load_ipython_extension` / `unload_ipython_extension` IPython
//!     entry points
//!
//! Rust port surfaces:
//!   - All five classes as structs
//!   - `PowerlinePromptManager.render()` data-shape unpacking
//!   - The `'reload'` line-arg validation in PowerlineMagics
//!   - `old_prompt_manager` module-level snapshot via OnceLock<Mutex>
//!
//! The actual IPython runtime calls (`ip.prompt_manager`,
//! `ip.hooks.shutdown_hook.add`, `ip.register_magics`, `TryNext`,
//! `weakref.ref`, `IPython.core.prompts.PromptManager`) are stubbed
//! since they require the live IPython process.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from weakref import ref                                                                  // py:5
// from warnings import warn                                                                // py:6
// from IPython.core.prompts import PromptManager  (optional)                              // py:8
// from IPython.core.magic import Magics, magics_class, line_magic                          // py:13
// from powerline.ipython import IPythonPowerline, IPythonInfo                              // py:15
// from powerline.ipython import RewriteResult  (when has_prompt_manager)                   // py:18

use crate::ported::ipython::{IPythonInfo, IPythonPowerline, RewriteResult};
use serde_json::{Map, Value};
use std::sync::{Mutex, OnceLock};

/// Module-level `old_prompt_manager` snapshot from
/// `powerline/bindings/ipython/post_0_11.py:36`.
///
/// Python: `old_prompt_manager = None` until `do_setup` snapshots
/// the live `ip.prompt_manager`. Rust uses OnceLock<Mutex<Option>>
/// for the same lazy-snapshot pattern.
pub fn old_prompt_manager() -> &'static Mutex<Option<Value>> {
    static M: OnceLock<Mutex<Option<Value>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(None))
}

/// Port of `class PowerlineMagics(Magics)` from
/// `powerline/bindings/ipython/post_0_11.py:21`.
///
/// IPython line-magic class registered as `%powerline`. The only
/// supported arg is `reload`; any other input raises ValueError.
pub struct PowerlineMagics {
    /// Stores whether `reload` was the last invocation. Used by tests
    /// to verify the magic dispatch.
    pub last_reload_call: bool,
}

impl Default for PowerlineMagics {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerlineMagics {
    /// Port of `PowerlineMagics.__init__()` from
    /// `powerline/bindings/ipython/post_0_11.py:23`.
    pub fn new() -> Self {
        Self {
            last_reload_call: false,
        }
    }

    /// Port of `PowerlineMagics.powerline()` (line_magic) from
    /// `powerline/bindings/ipython/post_0_11.py:27`.
    ///
    /// `line == 'reload'` triggers `self._powerline.reload()`;
    /// any other input raises ValueError.
    pub fn powerline(&mut self, line: &str) -> Result<(), String> {
        // py:27  @line_magic
        // py:28  def powerline(self, line):
        // py:29  if line == 'reload':
        // py:30  self._powerline.reload()
        // py:31  else:
        // py:32  raise ValueError('Expected `reload`, but got {0}'.format(line))
        if line == "reload" {
            self.last_reload_call = true;
            Ok(())
        } else {
            Err(format!("Expected `reload`, but got {}", line))
        }
    }
}

/// Port of `class ShutdownHook(object)` from
/// `powerline/bindings/ipython/post_0_11.py:39`.
///
/// Registered into IPython's `ip.hooks.shutdown_hook`. When invoked,
/// derefs the weakref to powerline and calls `shutdown()`, then raises
/// `TryNext` so subsequent hooks fire too. Rust port stores the
/// powerline reference directly (Rust has no weakref equivalent in
/// std) and returns a sentinel "try_next" string for the raise.
pub struct ShutdownHook {
    /// Python: `self.powerline = lambda: None` initially; replaced by
    /// `ref(self)` after do_setup runs. Rust port uses Option<()> as
    /// the "live/None" sentinel since the actual weakref target lives
    /// elsewhere in the test stack.
    pub powerline_live: bool,
    /// Set when `__call__` fires.
    pub call_count: u32,
}

impl Default for ShutdownHook {
    fn default() -> Self {
        Self::new()
    }
}

impl ShutdownHook {
    /// Port of `ShutdownHook.__init__()` from
    /// `powerline/bindings/ipython/post_0_11.py:40`.
    pub fn new() -> Self {
        // py:41  self.powerline = lambda: None  → represents the
        // weakref to a dead/never-set target.
        Self {
            powerline_live: false,
            call_count: 0,
        }
    }

    /// Sets the powerline weakref target. Called by
    /// `ConfigurableIPythonPowerline.do_setup` (py:97
    /// `shutdown_hook.powerline = ref(self)`).
    pub fn set_powerline(&mut self) {
        // py:97  shutdown_hook.powerline = ref(self)
        self.powerline_live = true;
    }

    /// Port of `ShutdownHook.__call__()` from
    /// `powerline/bindings/ipython/post_0_11.py:44`.
    ///
    /// Returns the string `"TryNext"` after calling shutdown (or
    /// silently skipping when the weakref target is dead). Python
    /// raises `TryNext` to signal the next hook should fire.
    pub fn call(&mut self) -> &'static str {
        // py:45  from IPython.core.hooks import TryNext
        // py:46-48  powerline = self.powerline(); if powerline: shutdown()
        if self.powerline_live {
            self.call_count += 1;
        }
        // py:49  raise TryNext()
        "TryNext"
    }
}

/// Port of `class PowerlinePromptManager(PromptManager)` from
/// `powerline/bindings/ipython/post_0_11.py:53`.
///
/// Wraps `IPythonPowerline.render()` for use by IPython's
/// `PromptManager`. Only available when pre-5 IPython is installed.
pub struct PowerlinePromptManager {
    /// Python: `self.powerline_segment_info = IPythonInfo(shell)`.
    pub segment_info: IPythonInfo,
    /// Python: `self.txtwidth` — set on each render() call.
    pub txtwidth: usize,
    /// Python: `self.width` — same value as txtwidth (py:71-72).
    pub width: usize,
}

impl PowerlinePromptManager {
    /// Port of `PowerlinePromptManager.__init__()` from
    /// `powerline/bindings/ipython/post_0_11.py:54`.
    pub fn new(segment_info: IPythonInfo) -> Self {
        Self {
            segment_info,
            txtwidth: 0,
            width: 0,
        }
    }

    /// Port of `PowerlinePromptManager.render()` from
    /// `powerline/bindings/ipython/post_0_11.py:58`.
    ///
    /// Calls the supplied `render` callback with the unpacked
    /// `is_prompt`/`side`/`output_width`/`output_raw`/`matcher_info`
    /// args and unpacks the returned `(text, raw, width)` tuple per
    /// `color` and `name == 'rewrite'`.
    pub fn render<R>(&mut self, name: &str, color: bool, mut render: R) -> PromptManagerResult
    where
        R: FnMut(bool, &str, bool, bool, &str) -> (String, String, usize),
    {
        // py:58  def render(self, name, color=True, *args, **kwargs):
        // py:59  res = self.powerline.render(
        // py:60  is_prompt=name.startswith('in'),
        // py:61  side='left',
        // py:62  output_width=True,
        // py:63  output_raw=not color,
        // py:64  matcher_info=name,
        // py:65  segment_info=self.powerline_segment_info,
        // py:66  )
        let is_prompt = name.starts_with("in");
        let res = render(is_prompt, "left", true, !color, name);
        // py:67  self.txtwidth = res[-1]
        // py:68  self.width = res[-1]
        self.txtwidth = res.2;
        self.width = res.2;
        // py:69  ret = res[0] if color else res[1]
        let ret = if color { res.0 } else { res.1 };
        // py:70  if name == 'rewrite':
        // py:71  return RewriteResult(ret)
        // py:72  else:
        // py:73  return ret
        if name == "rewrite" {
            PromptManagerResult::Rewrite(RewriteResult::new(ret))
        } else {
            PromptManagerResult::Plain(ret)
        }
    }
}

/// Return type for `PowerlinePromptManager::render()`. Mirrors the
/// Python branch where `name == 'rewrite'` returns `RewriteResult`
/// and other names return a plain string.
#[derive(Debug, Clone)]
pub enum PromptManagerResult {
    Rewrite(RewriteResult),
    Plain(String),
}

/// Port of `class ConfigurableIPythonPowerline(IPythonPowerline)`
/// from `powerline/bindings/ipython/post_0_11.py:76`.
///
/// Note: distinct from the since_5/since_7 ConfigurableIPythonPowerline
/// since the renderer module pin and do_setup signature differ.
pub struct ConfigurableIPythonPowerline {
    pub base: IPythonPowerline,
    /// Tracks whether `do_setup` wired the shutdown_hook.
    pub setup_done: bool,
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
            setup_done: false,
        }
    }

    /// Port of `ConfigurableIPythonPowerline.init()` from
    /// `powerline/bindings/ipython/post_0_11.py:77`.
    ///
    /// Reads `ip.config.Powerline` for the three override keys. The
    /// renderer module pin is `'.pre_5'` when `has_prompt_manager`,
    /// else `'.since_7'`. The Rust port takes the flag as an
    /// argument since we can't autodetect the IPython runtime.
    pub fn init(
        &mut self,
        powerline_config: &Map<String, Value>,
        has_prompt_manager: bool,
    ) -> &'static str {
        // py:76  def init(self, ip):
        // py:77  config = ip.config.Powerline
        // py:78  self.config_overrides = config.get('config_overrides')
        if let Some(overrides) = powerline_config
            .get("config_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.config_overrides = Some(overrides.clone());
        }
        // py:79  self.theme_overrides = config.get('theme_overrides', {})
        if let Some(themes) = powerline_config
            .get("theme_overrides")
            .and_then(|v| v.as_object())
        {
            self.base.theme_overrides = themes.clone();
        }
        // py:80  self.config_paths = config.get('config_paths')
        if let Some(paths) = powerline_config
            .get("config_paths")
            .and_then(|v| v.as_array())
        {
            self.base.config_paths = paths
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        // py:81  if has_prompt_manager:
        // py:82  renderer_module = '.pre_5'
        // py:83  else:
        // py:84  renderer_module = '.since_7'
        // py:85  super(ConfigurableIPythonPowerline, self).init(
        // py:86  renderer_module=renderer_module)
        if has_prompt_manager {
            ".pre_5"
        } else {
            ".since_7"
        }
    }

    /// Port of `ConfigurableIPythonPowerline.do_setup()` from
    /// `powerline/bindings/ipython/post_0_11.py:89`.
    ///
    /// Snapshots `ip.prompt_manager` into the module-level
    /// `old_prompt_manager` slot, installs a `PowerlinePromptManager`
    /// in its place, registers `PowerlineMagics`, and stores a
    /// weakref into `shutdown_hook.powerline`.
    pub fn do_setup(&mut self, ip: &mut Map<String, Value>, shutdown_hook: &mut ShutdownHook) {
        // py:88  def do_setup(self, ip, shutdown_hook):
        // py:89  global old_prompt_manager
        // py:91  if old_prompt_manager is None:
        // py:92  old_prompt_manager = ip.prompt_manager
        let mut slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = ip
                .get("prompt_manager")
                .cloned()
                .or(Some(Value::String("<unset>".into())));
        }
        // py:93  prompt_manager = PowerlinePromptManager(
        // py:94  powerline=self,
        // py:95  shell=ip.prompt_manager.shell,
        // py:96  )
        // py:97  ip.prompt_manager = prompt_manager
        ip.insert(
            "prompt_manager".to_string(),
            Value::String("<PowerlinePromptManager>".into()),
        );
        // py:99  magics = PowerlineMagics(ip, self)
        // py:100  shutdown_hook.powerline = ref(self)
        // py:101  ip.register_magics(magics)
        shutdown_hook.set_powerline();
        self.setup_done = true;
    }
}

/// Port of `load_ipython_extension()` from
/// `powerline/bindings/ipython/post_0_11.py:104`.
///
/// IPython extension entry point. Returns `LoadResult::PromptManager`
/// when has_prompt_manager (pre-5 path) or `LoadResult::Deprecated`
/// (the IPython 5+ path that warns and installs PowerlinePrompts).
pub fn load_ipython_extension(has_prompt_manager: bool) -> LoadResult {
    // py:105-109  if has_prompt_manager: install pre-5 path
    if has_prompt_manager {
        LoadResult::PromptManager
    } else {
        // py:111-117  else: install since_7 path + DeprecationWarning
        LoadResult::Deprecated
    }
}

/// Result of `load_ipython_extension`. Caller wires the actual
/// IPython registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadResult {
    /// Pre-5 IPython path: install PowerlinePromptManager via
    /// ConfigurableIPythonPowerline.setup().
    PromptManager,
    /// IPython 5+ path: install PowerlinePrompts from since_7 and
    /// emit a DeprecationWarning.
    Deprecated,
}

/// Port of `unload_ipython_extension()` from
/// `powerline/bindings/ipython/post_0_11.py:121`.
///
/// Restores the snapshotted prompt manager into `ip.prompt_manager`.
pub fn unload_ipython_extension(ip: &mut Map<String, Value>) {
    // py:122-125  if old_prompt_manager is not None: ip.prompt_manager = ...
    let mut slot = old_prompt_manager()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(saved) = slot.take() {
        ip.insert("prompt_manager".to_string(), saved);
    }
    // py:125  old_prompt_manager = None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Module-scoped lock that serialises tests against the
    /// process-wide `old_prompt_manager` global. See since_7.rs for
    /// the macro/static-hoisting rationale.
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    macro_rules! lock_globals {
        () => {{
            TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    fn reset_old_prompt_manager() {
        let mut slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }

    #[test]
    fn powerline_magics_reload_succeeds() {
        // py:28-29  if line == 'reload': self._powerline.reload()
        let mut m = PowerlineMagics::new();
        assert!(m.powerline("reload").is_ok());
        assert!(m.last_reload_call);
    }

    #[test]
    fn powerline_magics_other_arg_errors() {
        // py:30-31  raise ValueError('Expected `reload`, but got {0}')
        let mut m = PowerlineMagics::new();
        let r = m.powerline("foo");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Expected `reload`"));
        assert!(!m.last_reload_call);
    }

    #[test]
    fn shutdown_hook_initial_state_no_call_count() {
        // py:41  self.powerline = lambda: None  → no live target
        let h = ShutdownHook::new();
        assert!(!h.powerline_live);
        assert_eq!(h.call_count, 0);
    }

    #[test]
    fn shutdown_hook_set_powerline_marks_live() {
        // py:97  shutdown_hook.powerline = ref(self)
        let mut h = ShutdownHook::new();
        h.set_powerline();
        assert!(h.powerline_live);
    }

    #[test]
    fn shutdown_hook_call_returns_try_next() {
        // py:49  raise TryNext()
        let mut h = ShutdownHook::new();
        h.set_powerline();
        let r = h.call();
        assert_eq!(r, "TryNext");
        assert_eq!(h.call_count, 1);
    }

    #[test]
    fn shutdown_hook_call_skips_shutdown_when_target_dead() {
        // py:46-48  if powerline is not None: shutdown()
        // When powerline_live=false (initial state), no shutdown call.
        let mut h = ShutdownHook::new();
        h.call();
        assert_eq!(h.call_count, 0);
    }

    #[test]
    fn powerline_prompt_manager_render_returns_plain_for_non_rewrite() {
        // py:71-72  return ret  (when name != 'rewrite')
        let mut pm = PowerlinePromptManager::new(IPythonInfo::new(0));
        let result = pm.render("in", true, |_is_prompt, _side, _ow, _or, _name| {
            ("colored".to_string(), "raw".to_string(), 5)
        });
        match result {
            PromptManagerResult::Plain(s) => assert_eq!(s, "colored"),
            _ => panic!("expected Plain"),
        }
        assert_eq!(pm.txtwidth, 5);
        assert_eq!(pm.width, 5);
    }

    #[test]
    fn powerline_prompt_manager_render_returns_rewrite_for_rewrite_name() {
        // py:73-74  if name == 'rewrite': return RewriteResult(ret)
        let mut pm = PowerlinePromptManager::new(IPythonInfo::new(0));
        let result = pm.render("rewrite", true, |_is_prompt, _side, _ow, _or, _name| {
            ("rewritten".to_string(), "raw".to_string(), 7)
        });
        match result {
            PromptManagerResult::Rewrite(r) => assert_eq!(r.prompt, "rewritten"),
            _ => panic!("expected Rewrite"),
        }
    }

    #[test]
    fn powerline_prompt_manager_render_returns_raw_when_color_false() {
        // py:69-70  ret = res[0] if color else res[1]
        let mut pm = PowerlinePromptManager::new(IPythonInfo::new(0));
        let result = pm.render("in", false, |_is_prompt, _side, _ow, _or, _name| {
            ("colored".to_string(), "raw_text".to_string(), 8)
        });
        match result {
            PromptManagerResult::Plain(s) => assert_eq!(s, "raw_text"),
            _ => panic!("expected Plain"),
        }
    }

    #[test]
    fn powerline_prompt_manager_render_passes_is_prompt_true_for_in_names() {
        // py:60  is_prompt=name.startswith('in')
        let mut pm = PowerlinePromptManager::new(IPythonInfo::new(0));
        let mut saw_is_prompt = false;
        let _ = pm.render("in", true, |is_prompt, _side, _ow, _or, _name| {
            saw_is_prompt = is_prompt;
            (String::new(), String::new(), 0)
        });
        assert!(saw_is_prompt);
    }

    #[test]
    fn powerline_prompt_manager_render_passes_is_prompt_false_for_other_names() {
        let mut pm = PowerlinePromptManager::new(IPythonInfo::new(0));
        let mut saw_is_prompt = true;
        let _ = pm.render("rewrite", true, |is_prompt, _side, _ow, _or, _name| {
            saw_is_prompt = is_prompt;
            (String::new(), String::new(), 0)
        });
        assert!(!saw_is_prompt);
    }

    #[test]
    fn configurable_init_returns_pre_5_when_has_prompt_manager() {
        // py:82-85  '.pre_5' if has_prompt_manager else '.since_7'
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        assert_eq!(c.init(&cfg, true), ".pre_5");
    }

    #[test]
    fn configurable_init_returns_since_7_without_prompt_manager() {
        let mut c = ConfigurableIPythonPowerline::new();
        let cfg = Map::new();
        assert_eq!(c.init(&cfg, false), ".since_7");
    }

    #[test]
    fn configurable_init_reads_overrides() {
        let mut c = ConfigurableIPythonPowerline::new();
        let mut overrides = Map::new();
        overrides.insert("k".to_string(), Value::from(1));
        let mut cfg = Map::new();
        cfg.insert("config_overrides".to_string(), Value::Object(overrides));
        c.init(&cfg, true);
        assert!(c.base.config_overrides.is_some());
    }

    #[test]
    fn configurable_do_setup_swaps_prompt_manager_and_wires_hook() {
        let _g = lock_globals!();
        reset_old_prompt_manager();
        let mut c = ConfigurableIPythonPowerline::new();
        let mut ip = Map::new();
        ip.insert("prompt_manager".to_string(), Value::String("<orig>".into()));
        let mut hook = ShutdownHook::new();
        c.do_setup(&mut ip, &mut hook);
        // ip.prompt_manager now holds the PowerlinePromptManager marker
        assert_eq!(
            ip.get("prompt_manager"),
            Some(&Value::String("<PowerlinePromptManager>".into()))
        );
        // The hook now has a live powerline ref
        assert!(hook.powerline_live);
        // setup_done flag flipped
        assert!(c.setup_done);
        // old_prompt_manager captured the original
        let slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert_eq!(*slot, Some(Value::String("<orig>".into())));
    }

    #[test]
    fn configurable_do_setup_does_not_overwrite_existing_snapshot() {
        // py:90-92  if old_prompt_manager is None: ...
        let _g = lock_globals!();
        let mut slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *slot = Some(Value::String("<pre-existing>".into()));
        drop(slot);

        let mut c = ConfigurableIPythonPowerline::new();
        let mut ip = Map::new();
        ip.insert("prompt_manager".to_string(), Value::String("<new>".into()));
        let mut hook = ShutdownHook::new();
        c.do_setup(&mut ip, &mut hook);
        let slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // Still the pre-existing snapshot, not <new>.
        assert_eq!(*slot, Some(Value::String("<pre-existing>".into())));
    }

    #[test]
    fn load_ipython_extension_with_prompt_manager_returns_prompt_manager() {
        // py:105-109  install pre-5 path
        assert_eq!(load_ipython_extension(true), LoadResult::PromptManager);
    }

    #[test]
    fn load_ipython_extension_without_prompt_manager_returns_deprecated() {
        // py:111-117  install since_7 + DeprecationWarning
        assert_eq!(load_ipython_extension(false), LoadResult::Deprecated);
    }

    #[test]
    fn unload_ipython_extension_restores_snapshot() {
        // py:122-125  if old_prompt_manager is not None: ip.prompt_manager = ...
        let _g = lock_globals!();
        reset_old_prompt_manager();
        let mut slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *slot = Some(Value::String("<saved>".into()));
        drop(slot);

        let mut ip = Map::new();
        ip.insert(
            "prompt_manager".to_string(),
            Value::String("<powerline>".into()),
        );
        unload_ipython_extension(&mut ip);
        assert_eq!(
            ip.get("prompt_manager"),
            Some(&Value::String("<saved>".into()))
        );
        // Slot reset to None
        let slot = old_prompt_manager()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        assert!(slot.is_none());
    }

    #[test]
    fn unload_ipython_extension_noop_when_no_snapshot() {
        let _g = lock_globals!();
        reset_old_prompt_manager();
        let mut ip = Map::new();
        ip.insert(
            "prompt_manager".to_string(),
            Value::String("<original>".into()),
        );
        unload_ipython_extension(&mut ip);
        // ip.prompt_manager untouched.
        assert_eq!(
            ip.get("prompt_manager"),
            Some(&Value::String("<original>".into()))
        );
    }
}
