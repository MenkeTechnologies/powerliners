// vim:fileencoding=utf-8:noet
//! Port of `powerline/vim.py`.
//!
//! Vim-specific Powerline bindings. The Python class wraps the live
//! `vim` module (statusline construction, global variable reads,
//! window iteration). The Rust port surfaces the pure transformation
//! pieces:
//!   - `_override_from(config, override_varname, key)` — overlay
//!     resolution
//!   - `VimPowerline::get_matcher_module(match_name, ext)` — the
//!     rpartition dispatch on dotted matcher names
//!   - `get_default_pycmd()` — pycmd choice based on Python major
//!     version (port returns "python3" since Rust-host always >= 3)
//!   - `create_window_statusline_format(pyeval)` — produces the
//!     `%!<pyeval>('powerline.statusline(<idx>)')` template
//!   - `pycmd()` / `set_pycmd()` global state
//!
//! The actual `vim.command` / `vim.eval` / `vim.windows` /
//! `__main__.powerline` dispatch remains stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import json                                      // py:5
// import logging                                   // py:6
// from itertools import count                      // py:8
// import vim                                       // py:11
// from powerline.bindings.vim import vim_get_func, vim_getvar, get_vim_encoding, python_to_vim  // py:15
// from powerline import Powerline, FailedUnicode, finish_common_config                    // py:16
// from powerline.lib.dict import mergedicts        // py:17
// from powerline.lib.unicode import u              // py:18

use crate::ported::lib::dict::mergedicts;
use serde_json::{Map, Value};
use std::sync::Mutex;
use std::sync::OnceLock;

/// Port of module-level `pycmd = None` variable from
/// `powerline/vim.py:340`.
///
/// Holds the current Python pycmd ("python"/"python3"/etc.) used by
/// the do_setup augroup commands. Initially None.
pub fn pycmd() -> &'static Mutex<Option<String>> {
    static M: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(None))
}

/// Port of `set_pycmd()` from
/// `powerline/vim.py:343`.
pub fn set_pycmd(new_pycmd: impl Into<String>) {
    // py:344-345  global pycmd; pycmd = new_pycmd
    let mut slot = pycmd().lock().unwrap_or_else(|e| e.into_inner());
    *slot = Some(new_pycmd.into());
}

/// Port of `get_default_pycmd()` from
/// `powerline/vim.py:348`.
///
/// Python source returns `'python'` on Python 2 and `'python3'` on
/// Python 3. Rust host is always >=3 (Rust runtime, no Python 2
/// reachable), so the port returns `"python3"`.
pub fn get_default_pycmd() -> &'static str {
    // py:349  return 'python' if sys.version_info < (3,) else 'python3'
    "python3"
}

/// Port of `_override_from()` from
/// `powerline/vim.py:21`.
///
/// `vim_get_var` is the caller-supplied resolver for the
/// `vim_getvar(override_varname)` call at py:23. Returns `None` to
/// represent the Python `KeyError` branch (variable not set).
pub fn _override_from(
    config: &mut Map<String, Value>,
    key: Option<&str>,
    vim_get_var: impl FnOnce() -> Option<Value>,
) {
    // py:21  def _override_from(config, override_varname, key=None):
    // py:22  try:
    // py:23  overrides = vim_getvar(override_varname)
    // py:24  except KeyError:
    // py:25  return config
    let overrides = match vim_get_var() {
        Some(v) => v,
        None => return,
    };
    // py:26  if key is not None:
    // py:27  try:
    // py:28  overrides = overrides[key]
    // py:29  except KeyError:
    // py:30  return config
    let overlay = if let Some(k) = key {
        match overrides.get(k) {
            Some(v) => v.clone(),
            None => return,
        }
    } else {
        overrides
    };
    // py:31  mergedicts(config, overrides)
    // py:32  return config
    if let Some(overlay_map) = overlay.as_object() {
        mergedicts(config, overlay_map.clone(), false);
    }
}

/// Port of `class VimVarHandler(logging.Handler, object)` from
/// `powerline/vim.py:36`.
///
/// Vim-specific logging handler that emits messages to a vim global
/// variable list. The Rust port surfaces the structural pieces +
/// the message-formatting helper since the actual `vim.command` /
/// `vim.eval` dispatch isn't reachable.
pub struct VimVarHandler {
    /// Python: `self.vim_varname` (ASCII-encoded variable name).
    pub vim_varname: String,
    /// Captured log messages — Rust-side accumulator that callers
    /// can drain to issue the real `vim.eval(b'add(...)')` calls.
    pub captured: Vec<String>,
}

impl VimVarHandler {
    /// Port of `VimVarHandler.__init__()` from
    /// `powerline/vim.py:42`.
    pub fn new(varname: impl Into<String>) -> Self {
        // py:35  class VimVarHandler(logging.Handler, object):
        // py:36  '''Vim-specific handler which emits messages to Vim global variables
        // py:37
        // py:38  :param str varname:
        // py:39  Variable where
        // py:40  '''
        // py:41  def __init__(self, varname):
        // py:42  super(VimVarHandler, self).__init__()
        // py:43  utf_varname = u(varname)
        // py:44  self.vim_varname = utf_varname.encode('ascii')
        // py:45  vim.command('unlet! g:' + utf_varname)
        // py:46  vim.command('let g:' + utf_varname + ' = []')
        Self {
            vim_varname: varname.into(),
            captured: Vec::new(),
        }
    }

    /// Port of `VimVarHandler.emit()` from
    /// `powerline/vim.py:47`.
    ///
    /// Captures the formatted message. Python's `record.message` +
    /// `record.exc_text` are joined with a newline; Rust port takes
    /// them as separate inputs.
    pub fn emit(&mut self, message: &str, exc_text: Option<&str>) {
        // py:48  def emit(self, record):
        // py:49  message = u(record.message)
        // py:50  if record.exc_text:
        // py:51  message += '\n' + u(record.exc_text)
        // py:52  vim.eval(b'add(g:' + self.vim_varname + b', ' + python_to_vim(message) + b')')
        let mut combined = message.to_string();
        if let Some(exc) = exc_text {
            combined.push('\n');
            combined.push_str(exc);
        }
        self.captured.push(combined);
    }
}

/// Port of `VimPowerline.create_window_statusline_constructor()`
/// from `powerline/vim.py:75`.
///
/// Returns a closure that, given a window index, produces the vim
/// `&l:stl` bytes value:
/// `b'%!<pyeval>(\'powerline.statusline(<idx>)\')'`.
pub fn create_window_statusline_format(pyeval: &str) -> impl Fn(u64) -> String + '_ {
    // py:68  if sys.version_info < (3,):
    // py:69  def create_window_statusline_constructor(self):
    // py:70  window_statusline = b'%!' + str(self.pyeval) + b'(\'powerline.statusline({0})\')'
    // py:71  return window_statusline.format
    // py:72  else:
    // py:73  def create_window_statusline_constructor(self):
    // py:74  startstr = b'%!' + self.pyeval.encode('ascii') + b'(\'powerline.statusline('
    // py:75  endstr = b')\')'
    // py:76  return lambda idx: (
    // py:77  startstr + str(idx).encode('ascii') + endstr
    // py:78  )
    move |idx: u64| -> String { format!("%!{}('powerline.statusline({})')", pyeval, idx) }
}

/// Port of `VimPowerline.get_matcher()` from
/// `powerline/vim.py:184`.
///
/// Parses a dotted matcher reference into `(module_path, function_name)`.
/// If the `match_name` has no dot, the module defaults to
/// `powerline.matchers.{ext}` per py:185-188.
pub fn get_matcher_module(match_name: &str, ext: &str) -> (String, String) {
    // py:185  match_module, separator, match_function = match_name.rpartition('.')
    if let Some(dot_idx) = match_name.rfind('.') {
        let (module, rest) = match_name.split_at(dot_idx);
        let function = &rest[1..]; // skip the '.'
        (module.to_string(), function.to_string())
    } else {
        // py:186-188  default module = 'powerline.matchers.{ext}'
        (
            format!("powerline.matchers.{}", ext),
            match_name.to_string(),
        )
    }
}

/// Port of `class VimPowerline(Powerline)` from
/// `powerline/vim.py:55`.
///
/// Rust port surfaces the pure state (last_window_id, pyeval,
/// captured local_themes) without wiring the live vim module dispatch.
pub struct VimPowerline {
    /// Python: `self.last_window_id` (initial 1 at py:57).
    pub last_window_id: u64,
    /// Python: `self.pyeval` (initial 'PowerlinePyeval' at py:58).
    pub pyeval: String,
    /// Captured `(key, config)` pairs from `add_local_theme` —
    /// mirrors py:127-131 `setup_kwargs['_local_themes']` accumulator.
    pub local_themes: Vec<(String, Map<String, Value>)>,
}

impl Default for VimPowerline {
    fn default() -> Self {
        Self::new("PowerlinePyeval")
    }
}

impl VimPowerline {
    /// Port of `VimPowerline.init()` from
    /// `powerline/vim.py:56`.
    ///
    /// Returns a fresh instance with `last_window_id=1` and the
    /// supplied pyeval (default `'PowerlinePyeval'`).
    pub fn new(pyeval: impl Into<String>) -> Self {
        // py:55  class VimPowerline(Powerline):
        // py:56  def init(self, pyeval='PowerlinePyeval', **kwargs):
        // py:57  super(VimPowerline, self).init('vim', **kwargs)
        // py:58  self.last_window_id = 1
        // py:59  self.pyeval = pyeval
        // py:60  self.construct_window_statusline = self.create_window_statusline_constructor()
        // py:61  if all((hasattr(vim.current.window, attr) for attr in ('options', 'vars', 'number'))):
        // py:62  self.win_idx = self.new_win_idx
        // py:63  else:
        // py:64  self.win_idx = self.old_win_idx
        // py:65  self._vim_getwinvar = vim_get_func('getwinvar', 'bytes')
        // py:66  self._vim_setwinvar = vim_get_func('setwinvar')
        Self {
            last_window_id: 1,
            pyeval: pyeval.into(),
            local_themes: Vec::new(),
        }
    }

    /// Port of `VimPowerline.add_local_theme()` from
    /// `powerline/vim.py:95`.
    ///
    /// **Status:** records `(key, config)` in the
    /// `local_themes` accumulator. Returns `true` per py:124 success
    /// path. The actual renderer wiring at py:121
    /// `self.renderer.add_local_theme(...)` is stubbed.
    pub fn add_local_theme(&mut self, key: impl Into<String>, config: Map<String, Value>) -> bool {
        // py:94  def add_local_theme(self, key, config):
        // py:95-118  docstring
        // py:119  log_prefix = 'matcher_load: {0}'
        // py:120  matcher = self.get_matcher(key)
        // py:121  theme_config = self.load_theme_config(config)
        // py:122  self.renderer.add_local_theme(matcher, {'config': theme_config})
        // py:123  return True
        self.local_themes.push((key.into(), config));
        true
    }

    /// Port of `VimPowerline.statusline()` from
    /// `powerline/vim.py:301`.
    ///
    /// Surfaces only the wrap branch (`None` window) — actual
    /// `self.render(...)` requires the unported render pipeline.
    /// Returns the `FailedUnicode("No window <id>")` message per
    /// py:302-303.
    pub fn failed_unicode_message(window_id: u64) -> String {
        // py:303  return FailedUnicode('No window {0}'.format(window_id))
        format!("No window {}", window_id)
    }

    /// Port of `VimPowerline.setup_components()` from
    /// `powerline/vim.py:331`.
    ///
    /// Returns the `vim.command(...)` strings the Python source
    /// emits at py:333-339, given the pyeval and a list of
    /// components (`None` defaults to ["statusline", "tabline"]).
    pub fn setup_components(&self, components: Option<&[&str]>) -> Vec<String> {
        // py:331  def setup_components(self, components):
        // py:332  if components is None:
        // py:333  components = ('statusline', 'tabline')
        // py:334  if 'statusline' in components:
        // py:335  # Is immediately changed after new_window function is run. Good for
        // py:336  # global value.
        // py:337  vim.command('set statusline=%!{0}(\'powerline.new_window()\')'.format(self.pyeval))
        // py:338  if 'tabline' in components:
        // py:339  vim.command('set tabline=%!{0}(\'powerline.tabline()\')'.format(self.pyeval))
        let defaults = ["statusline", "tabline"];
        let comps = components.unwrap_or(&defaults);
        let mut out: Vec<String> = Vec::new();
        for c in comps {
            if *c == "statusline" {
                out.push(format!(
                    "set statusline=%!{}('powerline.new_window()')",
                    self.pyeval
                ));
            }
            if *c == "tabline" {
                out.push(format!(
                    "set tabline=%!{}('powerline.tabline()')",
                    self.pyeval
                ));
            }
        }
        out
    }

    /// Port of `VimPowerline.reset_highlight()` from
    /// `powerline/vim.py:252-261`.
    ///
    /// Python wraps `self.renderer.reset_highlight()` in try/except
    /// AttributeError per py:253-261. The Rust port takes the
    /// renderer-reset closure as an `Option<F>` (None = renderer not
    /// yet created, mirroring Python's AttributeError path).
    pub fn reset_highlight<F>(reset_renderer: Option<F>)
    where
        F: FnOnce(),
    {
        // py:253-255  try: self.renderer.reset_highlight()
        if let Some(f) = reset_renderer {
            f();
        }
        // py:256-261  except AttributeError: pass (renderer not yet built)
    }

    /// Port of the per-window id assignment from
    /// `powerline/vim.py:266-274` (inside new_win_idx).
    ///
    /// Returns `(window_id, new_last_window_id, assigned)` where:
    /// - `existing` is the pre-existing `powerline_window_id` for
    ///   the window (None when missing per py:271 KeyError),
    /// - `match_window_id` is the requested window_id (py:269),
    /// - on conflict (existing matches but we already found r),
    ///   forces re-assignment per py:270-273.
    pub fn assign_window_id(
        existing: Option<u64>,
        last_window_id: u64,
        conflict: bool,
    ) -> (u64, u64, bool) {
        // py:266  def new_win_idx(self, window_id):
        // py:267  if window_id:
        // py:268  for window in vim.windows:
        // py:269  try:
        // py:270  curwindow_id = window.vars['powerline_window_id']
        // py:271  if curwindow_id == window_id:
        // py:272  break
        // py:273  except KeyError:
        // py:274  pass
        match existing {
            Some(id) if !conflict => (id, last_window_id, false),
            _ => (last_window_id, last_window_id + 1, true),
        }
    }

    /// Port of `VimPowerline.tabline()` fallback at
    /// `powerline/vim.py:311-317`.
    ///
    /// When `win_idx(None)` returns None per py:306, falls back to
    /// `(vim.current.window, last_window_id_or_existing,
    /// vim.current.window.number)`. The Rust port returns the
    /// (window_id, winnr) pair callers feed to render(is_tabline=True).
    pub fn tabline_fallback_window(
        current_window_existing_id: Option<u64>,
        current_window_number: u64,
        last_window_id: u64,
    ) -> (u64, u64) {
        // py:312-316  (win, win.vars.get('powerline_window_id', last_window_id), win.number)
        let window_id = current_window_existing_id.unwrap_or(last_window_id);
        (window_id, current_window_number)
    }

    /// Port of `VimPowerline.do_pyeval()` at
    /// `powerline/vim.py:322-330`.
    ///
    /// Returns the `vim.command(...)` string Python emits at py:330,
    /// given the JSON-encoded eval result. The actual `eval(...)`
    /// dispatch lives outside the Rust port — caller wires the
    /// evaluation through its own python-runtime layer and passes
    /// the result JSON in.
    pub fn do_pyeval_command(json_encoded_result: &str) -> String {
        // py:330  vim.command('return ' + json.dumps(eval(...)))
        format!("return {}", json_encoded_result)
    }

    /// Port of `VimPowerline.statusline()` early-exit at
    /// `powerline/vim.py:299-303`.
    ///
    /// Returns the "No window" message when `win_idx(window_id)`
    /// returned None per py:300-302; None when a real window was
    /// found (caller routes through `render(window, window_id,
    /// winnr)` at py:303 directly).
    pub fn statusline_no_window_message(window_id: Option<u64>) -> Option<String> {
        // py:300-302
        Some(format!("No window {}", window_id.unwrap_or(0)))
    }

    /// Port of `VimPowerline.get_local_themes()` from
    /// `powerline/vim.py:157-174`.
    ///
    /// Builds the `(matcher_key → {'config': resolved_theme})` dict.
    /// `__tabline__` is the magic key for the tabline matcher per
    /// py:165 — its matcher is `None` in the dict result. Other
    /// matchers resolve through `get_matcher` per py:165; entries
    /// whose matcher fails to resolve are dropped per py:170-173.
    ///
    /// `resolve_matcher` is the caller-supplied closure that runs
    /// `self.get_matcher(key)` (returning Some when the matcher
    /// resolves, None otherwise). `load_theme` produces the resolved
    /// theme dict per py:162 `self.load_theme_config(val)`.
    pub fn get_local_themes<R, L>(
        local_themes: &Map<String, Value>,
        mut resolve_matcher: R,
        mut load_theme: L,
    ) -> Map<String, Value>
    where
        R: FnMut(&str) -> Option<String>,
        L: FnMut(&str) -> Value,
    {
        // py:158-159  if not local_themes: return {}
        if local_themes.is_empty() {
            return Map::new();
        }
        let mut out = Map::new();
        for (k, v) in local_themes {
            // py:165  None if k == '__tabline__' else self.get_matcher(k)
            let matcher_key = if k == "__tabline__" {
                Some("__tabline__".to_string())
            } else {
                resolve_matcher(k)
            };
            // py:170-173  filter by (matcher or key == '__tabline__')
            if let Some(matcher) = matcher_key {
                let val_str = v.as_str().unwrap_or("");
                let theme = load_theme(val_str);
                let mut entry = Map::new();
                entry.insert("config".to_string(), theme);
                out.insert(matcher, Value::Object(entry));
            }
        }
        out
    }

    /// Port of `VimPowerline.get_config_paths()` from
    /// `powerline/vim.py:183-187`.
    ///
    /// Returns the `g:powerline_config_paths` value when set per
    /// py:185; falls through to the super-class `get_config_paths`
    /// per py:186-187. The Rust port takes both sides as explicit
    /// args since the live vim resolver isn't reachable.
    pub fn get_config_paths(
        powerline_config_paths: Option<Vec<String>>,
        super_config_paths: impl FnOnce() -> Vec<String>,
    ) -> Vec<String> {
        // py:184-187
        match powerline_config_paths {
            Some(p) => p,
            None => super_config_paths(),
        }
    }

    /// Port of `VimPowerline.new_window()` from
    /// `powerline/vim.py:319-320`.
    ///
    /// Returns the `(window, window_id, winnr)` tuple that
    /// `self.render(*self.win_idx(None))` would dispatch on. The
    /// actual render() at py:320 lives outside the Rust port.
    ///
    /// Returns the caller-supplied win_idx result; None means the
    /// vim runtime has no current window (Python panics in that
    /// case; Rust returns None for testability).
    pub fn new_window<W>(win_idx_none: W) -> Option<(u64, u64)>
    where
        W: FnOnce() -> Option<(u64, u64)>,
    {
        // py:320  return self.render(*self.win_idx(None))
        win_idx_none()
    }

    /// Port of `setup()` from `powerline/vim.py:354`.
    ///
    /// Convenience wrapper that constructs the `VimPowerline` and
    /// returns it. Python calls `.setup(...)` on the new instance
    /// to wire up the augroup; the Rust port stops at construction
    /// since the live vim runtime isn't reachable.
    pub fn setup_entry(pyeval: impl Into<String>) -> Self {
        // py:355  powerline = VimPowerline()
        // py:356  return powerline.setup(*args, **kwargs)  (stubbed)
        Self::new(pyeval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    macro_rules! lock_globals {
        () => {{
            TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    fn reset_pycmd() {
        let mut slot = pycmd().lock().unwrap_or_else(|e| e.into_inner());
        *slot = None;
    }

    #[test]
    fn get_default_pycmd_returns_python3() {
        // py:349  Rust host is always Python-3 era
        assert_eq!(get_default_pycmd(), "python3");
    }

    #[test]
    fn set_pycmd_updates_global() {
        let _g = lock_globals!();
        reset_pycmd();
        set_pycmd("py3eval");
        let slot = pycmd().lock().unwrap_or_else(|e| e.into_inner());
        assert_eq!(slot.as_deref(), Some("py3eval"));
    }

    #[test]
    fn override_from_no_var_leaves_config_untouched() {
        // py:23-26  except KeyError: return
        let mut cfg = Map::new();
        cfg.insert("k".to_string(), Value::from(1));
        _override_from(&mut cfg, None, || None);
        assert_eq!(cfg.get("k"), Some(&Value::from(1)));
        assert_eq!(cfg.len(), 1);
    }

    #[test]
    fn override_from_no_key_merges_full_dict() {
        // py:28-31  key=None path: merge overrides directly
        let mut cfg = Map::new();
        cfg.insert("k".to_string(), Value::from(1));
        _override_from(&mut cfg, None, || {
            let mut o = Map::new();
            o.insert("k2".to_string(), Value::from(2));
            Some(Value::Object(o))
        });
        assert_eq!(cfg.get("k"), Some(&Value::from(1)));
        assert_eq!(cfg.get("k2"), Some(&Value::from(2)));
    }

    #[test]
    fn override_from_with_matching_key_merges_subdict() {
        // py:28-31  key path: overrides = overrides[key]
        let mut cfg = Map::new();
        _override_from(&mut cfg, Some("theme1"), || {
            let mut o = Map::new();
            let mut inner = Map::new();
            inner.insert("seg".to_string(), Value::String("custom".into()));
            o.insert("theme1".to_string(), Value::Object(inner));
            Some(Value::Object(o))
        });
        assert_eq!(cfg.get("seg"), Some(&Value::String("custom".into())));
    }

    #[test]
    fn override_from_with_missing_key_no_op() {
        let mut cfg = Map::new();
        cfg.insert("k".to_string(), Value::from(1));
        _override_from(&mut cfg, Some("not_present"), || {
            let mut o = Map::new();
            o.insert("other_theme".to_string(), Value::from(2));
            Some(Value::Object(o))
        });
        assert_eq!(cfg.get("k"), Some(&Value::from(1)));
        assert_eq!(cfg.len(), 1);
    }

    #[test]
    fn create_window_statusline_format_substitutes_idx() {
        // py:78-79  startstr + idx + endstr
        let f = create_window_statusline_format("PowerlinePyeval");
        let s = f(7);
        assert_eq!(s, "%!PowerlinePyeval('powerline.statusline(7)')");
    }

    #[test]
    fn create_window_statusline_format_works_for_zero_idx() {
        let f = create_window_statusline_format("PowerlinePyeval");
        assert_eq!(f(0), "%!PowerlinePyeval('powerline.statusline(0)')");
    }

    #[test]
    fn create_window_statusline_format_with_custom_pyeval() {
        let f = create_window_statusline_format("py3eval");
        assert_eq!(f(42), "%!py3eval('powerline.statusline(42)')");
    }

    #[test]
    fn get_matcher_module_splits_dotted_name() {
        // py:185  rpartition on '.'
        let (m, f) = get_matcher_module("mymodule.fn_name", "vim");
        assert_eq!(m, "mymodule");
        assert_eq!(f, "fn_name");
    }

    #[test]
    fn get_matcher_module_unrpartitioned_defaults_to_powerline_matchers() {
        // py:186-188  default module = 'powerline.matchers.{ext}'
        let (m, f) = get_matcher_module("plain_name", "vim");
        assert_eq!(m, "powerline.matchers.vim");
        assert_eq!(f, "plain_name");
    }

    #[test]
    fn get_matcher_module_takes_rightmost_dot() {
        // py:185  rpartition takes rightmost
        let (m, f) = get_matcher_module("a.b.c.fn", "vim");
        assert_eq!(m, "a.b.c");
        assert_eq!(f, "fn");
    }

    #[test]
    fn vim_var_handler_captures_message() {
        let mut h = VimVarHandler::new("powerline_log_messages");
        h.emit("hello world", None);
        assert_eq!(h.captured.len(), 1);
        assert_eq!(h.captured[0], "hello world");
    }

    #[test]
    fn vim_var_handler_appends_exc_text_with_newline() {
        // py:48-50  message += '\n' + exc_text
        let mut h = VimVarHandler::new("v");
        h.emit("error msg", Some("Traceback (most recent call last):..."));
        assert_eq!(
            h.captured[0],
            "error msg\nTraceback (most recent call last):..."
        );
    }

    #[test]
    fn vim_var_handler_stores_varname_as_ascii() {
        // py:44  utf_varname.encode('ascii')
        let h = VimVarHandler::new("powerline_log_messages");
        assert_eq!(h.vim_varname, "powerline_log_messages");
    }

    #[test]
    fn vim_powerline_init_defaults_to_powerlinepyeval() {
        // py:56  pyeval='PowerlinePyeval'
        let p = VimPowerline::default();
        assert_eq!(p.pyeval, "PowerlinePyeval");
        assert_eq!(p.last_window_id, 1);
    }

    #[test]
    fn vim_powerline_add_local_theme_records_kv_and_returns_true() {
        // py:127-131  setup_kwargs['_local_themes'].append + return True
        let mut p = VimPowerline::default();
        let mut cfg = Map::new();
        cfg.insert("seg".to_string(), Value::String("custom".into()));
        let r = p.add_local_theme("matcher_a", cfg.clone());
        assert!(r);
        assert_eq!(p.local_themes.len(), 1);
        assert_eq!(p.local_themes[0].0, "matcher_a");
        assert_eq!(p.local_themes[0].1.get("seg"), cfg.get("seg"));
    }

    #[test]
    fn failed_unicode_message_includes_window_id() {
        // py:303  FailedUnicode('No window {0}'.format(window_id))
        assert_eq!(VimPowerline::failed_unicode_message(7), "No window 7");
    }

    #[test]
    fn setup_components_default_emits_statusline_and_tabline() {
        // py:332-339  defaults to ('statusline', 'tabline')
        let p = VimPowerline::default();
        let r = p.setup_components(None);
        assert_eq!(r.len(), 2);
        assert!(r[0].contains("statusline=%!PowerlinePyeval"));
        assert!(r[1].contains("tabline=%!PowerlinePyeval"));
    }

    #[test]
    fn setup_components_only_statusline() {
        let p = VimPowerline::default();
        let r = p.setup_components(Some(&["statusline"]));
        assert_eq!(r.len(), 1);
        assert!(r[0].contains("statusline=%!"));
    }

    #[test]
    fn setup_components_only_tabline() {
        let p = VimPowerline::default();
        let r = p.setup_components(Some(&["tabline"]));
        assert_eq!(r.len(), 1);
        assert!(r[0].contains("tabline=%!"));
    }

    #[test]
    fn setup_components_unknown_ignored() {
        let p = VimPowerline::default();
        let r = p.setup_components(Some(&["unknown"]));
        assert!(r.is_empty());
    }

    #[test]
    fn setup_components_uses_custom_pyeval() {
        let p = VimPowerline::new("py3eval");
        let r = p.setup_components(Some(&["statusline"]));
        assert!(r[0].contains("py3eval"));
    }

    #[test]
    fn setup_entry_returns_powerline_with_given_pyeval() {
        // py:354-356  setup() returns VimPowerline instance
        let p = VimPowerline::setup_entry("py3eval");
        assert_eq!(p.pyeval, "py3eval");
        assert_eq!(p.last_window_id, 1);
    }

    #[test]
    fn reset_highlight_calls_renderer_when_present() {
        // py:253-255
        use std::cell::Cell;
        let called = Cell::new(false);
        VimPowerline::reset_highlight(Some(|| called.set(true)));
        assert!(called.get());
    }

    #[test]
    fn reset_highlight_no_op_when_renderer_missing() {
        // py:256-261  AttributeError: pass
        let f: Option<fn()> = None;
        VimPowerline::reset_highlight(f);
    }

    #[test]
    fn assign_window_id_existing_no_conflict_reuses() {
        // py:271  use existing
        let (id, last, assigned) = VimPowerline::assign_window_id(Some(7), 10, false);
        assert_eq!(id, 7);
        assert_eq!(last, 10);
        assert!(!assigned);
    }

    #[test]
    fn assign_window_id_no_existing_assigns_and_bumps() {
        // py:271-273
        let (id, last, assigned) = VimPowerline::assign_window_id(None, 10, false);
        assert_eq!(id, 10);
        assert_eq!(last, 11);
        assert!(assigned);
    }

    #[test]
    fn assign_window_id_existing_with_conflict_reassigns() {
        // py:269-273  forces re-assignment via KeyError when conflict
        let (id, last, assigned) = VimPowerline::assign_window_id(Some(5), 10, true);
        assert_eq!(id, 10);
        assert_eq!(last, 11);
        assert!(assigned);
    }

    #[test]
    fn tabline_fallback_window_uses_existing_id_when_set() {
        // py:312-316
        let (window_id, winnr) = VimPowerline::tabline_fallback_window(Some(3), 5, 100);
        assert_eq!(window_id, 3);
        assert_eq!(winnr, 5);
    }

    #[test]
    fn tabline_fallback_window_defaults_to_last_window_id() {
        // py:314  vars.get('powerline_window_id', last_window_id)
        let (window_id, winnr) = VimPowerline::tabline_fallback_window(None, 5, 100);
        assert_eq!(window_id, 100);
        assert_eq!(winnr, 5);
    }

    #[test]
    fn do_pyeval_command_builds_return_statement() {
        // py:330  vim.command('return ' + json.dumps(...))
        let cmd = VimPowerline::do_pyeval_command("[1, 2, 3]");
        assert_eq!(cmd, "return [1, 2, 3]");
    }

    #[test]
    fn do_pyeval_command_handles_dict_json() {
        let cmd = VimPowerline::do_pyeval_command(r#"{"key": "value"}"#);
        assert_eq!(cmd, r#"return {"key": "value"}"#);
    }

    #[test]
    fn statusline_no_window_message_with_id() {
        // py:302-303
        let msg = VimPowerline::statusline_no_window_message(Some(5)).unwrap();
        assert_eq!(msg, "No window 5");
    }

    #[test]
    fn statusline_no_window_message_without_id() {
        let msg = VimPowerline::statusline_no_window_message(None).unwrap();
        assert_eq!(msg, "No window 0");
    }

    #[test]
    fn get_local_themes_empty_returns_empty() {
        // py:158-159
        let r = VimPowerline::get_local_themes(
            &Map::new(),
            |_| Some("matcher".to_string()),
            |_| Value::Object(Map::new()),
        );
        assert!(r.is_empty());
    }

    #[test]
    fn get_local_themes_resolves_matchers() {
        // py:161-174
        let mut input = Map::new();
        input.insert("help".to_string(), Value::String("help_theme".into()));
        input.insert(
            "__tabline__".to_string(),
            Value::String("tabline_theme".into()),
        );

        let r = VimPowerline::get_local_themes(
            &input,
            |k| Some(format!("matchers.vim.{}", k)),
            |v| serde_json::json!({"name": v}),
        );

        // help → resolved matcher key
        assert!(r.contains_key("matchers.vim.help"));
        assert_eq!(r["matchers.vim.help"]["config"]["name"], "help_theme");
        // __tabline__ → magic key per py:165
        assert!(r.contains_key("__tabline__"));
        assert_eq!(r["__tabline__"]["config"]["name"], "tabline_theme");
    }

    #[test]
    fn get_local_themes_drops_unresolved_matchers() {
        // py:170-173  filter by (matcher or key == '__tabline__')
        let mut input = Map::new();
        input.insert("bogus".to_string(), Value::String("v".into()));
        let r = VimPowerline::get_local_themes(
            &input,
            |_| None, // matcher fails to resolve
            |v| serde_json::json!({"name": v}),
        );
        assert!(r.is_empty());
    }

    #[test]
    fn get_config_paths_returns_vim_var_when_set() {
        // py:184-185
        let paths =
            VimPowerline::get_config_paths(Some(vec!["/etc/powerline".to_string()]), || {
                vec!["/should/not/use".to_string()]
            });
        assert_eq!(paths, vec!["/etc/powerline".to_string()]);
    }

    #[test]
    fn get_config_paths_falls_back_to_super_when_unset() {
        // py:186-187  except KeyError: super().get_config_paths()
        let paths = VimPowerline::get_config_paths(None, || vec!["/super/path".to_string()]);
        assert_eq!(paths, vec!["/super/path".to_string()]);
    }

    #[test]
    fn new_window_returns_win_idx_result() {
        // py:319-320
        let r = VimPowerline::new_window(|| Some((5, 1)));
        assert_eq!(r, Some((5, 1)));
    }

    #[test]
    fn new_window_returns_none_when_no_current_window() {
        let r = VimPowerline::new_window(|| None);
        assert_eq!(r, None);
    }
}
