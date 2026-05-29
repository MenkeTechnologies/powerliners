// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/zsh/__init__.py`.
//!
//! Zsh shell bindings. The Python source wires the live `zsh` module
//! (`zsh.getvalue`/`zsh.setvalue`/`zsh.expand`/`zsh.pipestatus()`)
//! through a `Prompt` class that renders the statusline on each
//! prompt expansion. The Rust port surfaces:
//!   - `used_powerlines` weak registry (process-wide live set)
//!   - `get_var_config` parser (dict / str → mergeargs Map)
//!   - `Args` config-override / theme-override / config-path accessors
//!   - `Environment` getter contract
//!   - `string()` bytes-decode helper
//!   - `Prompt` segment_info builder + render-string post-processing
//!   - `set_prompt` zpyvar variable name builder
//!   - `setup` entry point (returns ZshPowerline; live atexit/zsh
//!     wiring stubbed)
//!
//! The live zsh module dispatch remains stubbed since adding a Rust
//! zsh binding crate is out of scope.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import atexit                                    // py:4
// from weakref import WeakValueDictionary, ref     // py:6
// import zsh                                       // py:8
// from powerline.shell import ShellPowerline       // py:10
// from powerline.lib.overrides import parsedotval, parse_override_var                       // py:11
// from powerline.lib.unicode import unicode, u     // py:12
// from powerline.lib.encoding import (get_preferred_output_encoding, get_preferred_environment_encoding)  // py:13
// from powerline.lib.dict import mergeargs          // py:15

use crate::ported::lib::dict::mergeargs;
use crate::ported::lib::overrides::{parse_override_var, parsedotval_tuple};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Process-wide registry of live ZshPowerline identifiers. Python
/// uses `WeakValueDictionary` so entries vanish when the powerline
/// instance is GC'd; Rust uses a `HashSet<u64>` of ids since std
/// has no weak hashmap.
pub fn used_powerlines() -> &'static Mutex<HashSet<u64>> {
    static M: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Port of `shutdown()` from
/// `powerline/bindings/zsh/__init__.py:22`.
///
/// `shutdown_each` is the caller-supplied closure that runs the
/// per-powerline shutdown (Python: `powerline.shutdown()` at py:23).
pub fn shutdown<F>(mut shutdown_each: F)
where
    F: FnMut(u64),
{
    // py:23-24  for powerline in used_powerlines.values(): powerline.shutdown()
    let ids: Vec<u64> = used_powerlines()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .copied()
        .collect();
    for id in ids {
        shutdown_each(id);
    }
}

/// Port of `get_var_config()` from
/// `powerline/bindings/zsh/__init__.py:27`.
///
/// `vim_get_var` resolves the named zsh variable. The result is
/// parsed via either `parsedotval` (for dict input) or
/// `parse_override_var` (for string input). Returns `None` on
/// missing variable or unsupported type.
pub fn get_var_config(value: Option<Value>) -> Option<Map<String, Value>> {
    // py:26  def get_var_config(var):
    // py:27  try:
    // py:28  val = zsh.getvalue(var)
    let v = value?;
    // py:29  if isinstance(val, dict):
    // py:30  return mergeargs([parsedotval((u(k), u(v))) for k, v in val.items()])
    if let Some(obj) = v.as_object() {
        let pairs: Vec<(String, Value)> = obj
            .iter()
            .map(|(k, v)| {
                let v_str = v.as_str().unwrap_or("").to_string();
                parsedotval_tuple(k, &v_str)
            })
            .collect();
        return mergeargs(pairs, false);
    }
    // py:31  elif isinstance(val, (unicode, str, bytes)):
    // py:32  return mergeargs(parse_override_var(u(val)))
    if let Some(s) = v.as_str() {
        return mergeargs(parse_override_var(s), false);
    }
    // py:33  else:
    // py:34  return None
    // py:35  except:
    // py:36  return None
    None
}

/// Port of `class Args(object)` from
/// `powerline/bindings/zsh/__init__.py:39`.
///
/// Holds the runtime args read at each prompt expansion. The
/// Python class exposes properties that re-read zsh globals on every
/// access; the Rust port takes them as method calls + caller-
/// supplied accessor closures.
#[derive(Debug, Clone, Default)]
pub struct Args {
    /// Python: `__slots__ = ('last_pipe_status', 'last_exit_code')`.
    pub last_pipe_status: Vec<i32>,
    pub last_exit_code: i32,
}

impl Args {
    /// Python class attribute: `ext = ['shell']` (py:41).
    pub fn ext() -> Vec<&'static str> {
        vec!["shell"]
    }

    /// Python class attribute: `renderer_module = '.zsh'` (py:42).
    pub const RENDERER_MODULE: &'static str = ".zsh";

    /// Constructs a fresh Args with defaults.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Port of `Args.config_path` (property) from
/// `powerline/bindings/zsh/__init__.py:52`.
///
/// Splits the `POWERLINE_CONFIG_PATHS` value on `:`, filters empty
/// entries. Returns the original list when the zsh value is itself
/// a list.
pub fn config_path_from_var(value: Option<Value>) -> Option<Vec<String>> {
    let v = value?;
    // py:58  isinstance(ret, (unicode, str, bytes)): split on ':' filter empty
    if let Some(s) = v.as_str() {
        return Some(
            s.split(':')
                .filter(|p| !p.is_empty())
                .map(|p| p.to_string())
                .collect(),
        );
    }
    // py:64-65  else: return ret (list form)
    if let Some(arr) = v.as_array() {
        return Some(
            arr.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect(),
        );
    }
    None
}

/// Port of `string()` from
/// `powerline/bindings/zsh/__init__.py:71`.
///
/// Python: `s.decode(get_preferred_environment_encoding(),
/// 'replace')` for bytes, `str(s)` otherwise. Rust always returns a
/// String; the bytes-decode branch uses UTF-8 with replace.
pub fn string(bytes: &[u8]) -> String {
    // py:73  def string(s):
    // py:74  if type(s) is bytes:
    // py:75  return s.decode(get_preferred_environment_encoding(), 'replace')
    // py:76  else:
    // py:77  return str(s)
    String::from_utf8_lossy(bytes).to_string()
}

/// Port of `Args.config_override` property at
/// `powerline/bindings/zsh/__init__.py:43-44`.
///
/// Reads `POWERLINE_CONFIG_OVERRIDES` and parses it via
/// `get_var_config`. The Rust port takes the resolved zsh value
/// directly since `zsh.getvalue` isn't reachable.
pub fn args_config_override(value: Option<Value>) -> Option<Map<String, Value>> {
    // py:44  return get_var_config('POWERLINE_CONFIG_OVERRIDES')
    get_var_config(value)
}

/// Port of `Args.theme_override` property at
/// `powerline/bindings/zsh/__init__.py:47-48`.
pub fn args_theme_override(value: Option<Value>) -> Option<Map<String, Value>> {
    // py:48  return get_var_config('POWERLINE_THEME_OVERRIDES')
    get_var_config(value)
}

/// Port of `Args.config_path` property at
/// `powerline/bindings/zsh/__init__.py:51-66`.
///
/// `value` is the resolved `POWERLINE_CONFIG_PATHS` zsh variable
/// value (None when py:55 raises IndexError). Strings/bytes are
/// split on `:` per py:59-65; lists pass through. Empty path
/// segments are filtered out per py:63.
pub fn args_config_path(value: Option<Value>) -> Option<Vec<String>> {
    // py:52  @property
    // py:53  def config_path(self):
    // py:54  try:
    // py:55  ret = zsh.getvalue('POWERLINE_CONFIG_PATHS')
    // py:56  except IndexError:
    // py:57  return None
    let v = value?;
    match v {
        // py:58  else:
        // py:59  if isinstance(ret, (unicode, str, bytes)):
        // py:60  return [
        // py:61  path
        // py:62  for path in ret.split((b':' if isinstance(ret, bytes) else ':'))
        // py:63  if path
        // py:64  ]
        Value::String(s) => Some(
            s.split(':')
                .filter(|p| !p.is_empty())
                .map(String::from)
                .collect(),
        ),
        // py:65  else:
        // py:66  return ret
        Value::Array(arr) => Some(
            arr.into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    }
}

/// Port of `Args.jobnum` property at
/// `powerline/bindings/zsh/__init__.py:68-72`.
///
/// Returns the `_POWERLINE_JOBNUM` zsh variable value. The Rust port
/// takes the resolved integer directly.
pub fn args_jobnum(value: Option<i32>) -> Option<i32> {
    // py:70  return zsh.getvalue('_POWERLINE_JOBNUM')
    value
}

/// Port of `class Environment(object)` from
/// `powerline/bindings/zsh/__init__.py:78`.
///
/// Surfaces `get(key, default)` + `__contains__(key)` over a
/// caller-supplied zsh-var resolver closure. The actual `zsh.getvalue`
/// call lives outside the Rust port.
pub struct Environment;

impl Environment {
    /// Port of `Environment.get()` (staticmethod) from
    /// `powerline/bindings/zsh/__init__.py:86`.
    pub fn get<F>(get_value: F, default: Option<String>) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        // py:88  @staticmethod
        // py:89  def get(key, default=None):
        // py:90  try:
        // py:91  return string(zsh.getvalue(key))
        // py:92  except IndexError:
        // py:93  return default
        get_value().or(default)
    }

    /// Port of `Environment.__contains__()` (staticmethod) from
    /// `powerline/bindings/zsh/__init__.py:93`.
    pub fn contains<F>(get_value: F) -> bool
    where
        F: FnOnce() -> Option<String>,
    {
        // py:95  @staticmethod
        // py:96  def __contains__(key):
        // py:97  try:
        // py:98  zsh.getvalue(key)
        // py:99  return True
        // py:100  except IndexError:
        // py:101  return False
        get_value().is_some()
    }
}

/// Port of `zsh_expand()` fallback at
/// `powerline/bindings/zsh/__init__.py:109-114`.
///
/// Python lazily checks `hasattr(zsh, 'expand')` per py:107; when
/// absent, falls back to:
///   1. `zsh.eval('local _POWERLINE_REPLY="' + s + '"')` (py:111)
///   2. `ret = zsh.getvalue('_POWERLINE_REPLY')` (py:112)
///   3. `zsh.setvalue('_POWERLINE_REPLY', None)` (py:113)
///   4. `return ret` (py:114)
///
/// The Rust port returns the (eval_command, getvalue_key,
/// setvalue_key, setvalue_value) tuple the caller dispatches
/// through its zsh-RPC binding.
pub fn zsh_expand_fallback_steps(s: &str) -> (String, &'static str, &'static str, Option<String>) {
    // py:107  if hasattr(zsh, 'expand') and zsh.expand('${:-}') == '':
    // py:108  zsh_expand = zsh.expand
    // py:109  else:
    // py:110  def zsh_expand(s):
    // py:111  zsh.eval('local _POWERLINE_REPLY="' + s + '"')
    let eval_cmd = format!("local _POWERLINE_REPLY=\"{}\"", s);
    // py:112  ret = zsh.getvalue('_POWERLINE_REPLY')
    // py:113  zsh.setvalue('_POWERLINE_REPLY', None)
    // py:114  return ret
    (eval_cmd, "_POWERLINE_REPLY", "_POWERLINE_REPLY", None)
}

/// Port of `set_prompt()` from
/// `powerline/bindings/zsh/__init__.py:196`.
///
/// Returns the zpyvar name that the Python source builds before
/// calling `zsh.set_special_string`. Caller wires the actual zsh
/// special-string registration.
pub fn set_prompt_zpyvar_name(psvar: &str) -> String {
    // py:199  def set_prompt(powerline, psvar, side, theme, above=False):
    // py:200  try:
    // py:201  savedps = zsh.getvalue(psvar)
    // py:202  except IndexError:
    // py:203  savedps = None
    // py:204  zpyvar = 'ZPYTHON_POWERLINE_' + psvar
    // py:205  prompt = Prompt(powerline, side, theme, psvar, savedps, above)
    // py:206  zsh.setvalue(zpyvar, None)
    // py:207  zsh.set_special_string(zpyvar, prompt)
    // py:208  zsh.setvalue(psvar, '${' + zpyvar + '}')
    // py:209  return ref(prompt)
    format!("ZPYTHON_POWERLINE_{}", psvar)
}

/// Builds the segment_info Map passed to `Powerline.render` per
/// `powerline/bindings/zsh/__init__.py:158-167`.
///
/// The Python source reads the parser_state / shortened_path / mode /
/// default_mode via zsh.expand and zsh.getvalue; the Rust port takes
/// them as explicit args.
pub fn build_segment_info(
    parser_state: &str,
    shortened_path: &str,
    mode: Option<&str>,
    default_mode: Option<&str>,
    client_id: u64,
    local_theme: Option<&str>,
) -> Map<String, Value> {
    // py:158  segment_info = {
    // py:159  'args': self.args,
    // py:160  'environ': environ,
    // py:161  'client_id': 1,
    // py:162  'local_theme': self.theme,
    // py:163  'parser_state': parser_state,
    // py:164  'shortened_path': shortened_path,
    // py:165  'mode': mode,
    // py:166  'default_mode': default_mode,
    // py:167  }
    let mut info = Map::new();
    info.insert("client_id".to_string(), Value::from(client_id));
    info.insert(
        "parser_state".to_string(),
        Value::String(parser_state.into()),
    );
    info.insert(
        "shortened_path".to_string(),
        Value::String(shortened_path.into()),
    );
    info.insert(
        "mode".to_string(),
        mode.map(|s| Value::String(s.into())).unwrap_or(Value::Null),
    );
    info.insert(
        "default_mode".to_string(),
        default_mode
            .map(|s| Value::String(s.into()))
            .unwrap_or(Value::Null),
    );
    info.insert(
        "local_theme".to_string(),
        local_theme
            .map(|s| Value::String(s.into()))
            .unwrap_or(Value::Null),
    );
    info
}

/// Port of `class Prompt(object)` from
/// `powerline/bindings/zsh/__init__.py:135`.
///
/// State accumulator for one prompt position. The Python source
/// uses `__slots__` to constrain attribute names; the Rust port
/// uses direct fields.
pub struct Prompt {
    /// Python: `self.side`.
    pub side: String,
    /// Python: `self.above` (used for the above-lines render).
    pub above: bool,
    /// Python: `self.savedpsvar` — variable name being shadowed.
    pub savedpsvar: Option<String>,
    /// Python: `self.savedps` — original PS value to restore on
    /// `__del__`.
    pub savedps: Option<String>,
    /// Python: `self.theme`.
    pub theme: Option<String>,
}

impl Prompt {
    /// Port of `Prompt.__del__()` value-restore step at
    /// `powerline/bindings/zsh/__init__.py:193-196`.
    ///
    /// Returns `Some((var_name, value))` to restore when both
    /// `savedps` and `savedpsvar` are set per py:194-195. The caller
    /// dispatches the zsh setvalue + powerline shutdown.
    pub fn del_restore(&self) -> Option<(String, String)> {
        // py:193  def __del__(self):
        // py:194  if self.savedps:
        // py:195  zsh.setvalue(self.savedpsvar, self.savedps)
        // py:196  self.powerline.shutdown()
        match (&self.savedpsvar, &self.savedps) {
            (Some(var), Some(ps)) if !ps.is_empty() => Some((var.clone(), ps.clone())),
            _ => None,
        }
    }

    /// Port of `Prompt.__init__()` from
    /// `powerline/bindings/zsh/__init__.py:138`.
    pub fn new(
        side: impl Into<String>,
        theme: Option<String>,
        savedpsvar: Option<String>,
        savedps: Option<String>,
        above: bool,
    ) -> Self {
        // py:138  def __init__(self, powerline, side, theme, savedpsvar=None, savedps=None, above=False):
        // py:139  self.powerline = powerline
        // py:140  self.side = side
        // py:141  self.above = above
        // py:142  self.savedpsvar = savedpsvar
        // py:143  self.savedps = savedps
        // py:144  self.args = powerline.args
        // py:145  self.theme = theme
        Self {
            side: side.into(),
            above,
            savedpsvar,
            savedps,
            theme,
        }
    }
}

/// Port of `class ZshPowerline(ShellPowerline)` from
/// `powerline/bindings/zsh/__init__.py:117`.
pub struct ZshPowerline {
    /// Python: `self.args` — the Args instance.
    pub args: Args,
    /// Process-wide identity used by used_powerlines registry.
    pub id: u64,
}

impl Default for ZshPowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl ZshPowerline {
    /// Process-wide id counter.
    fn next_id() -> u64 {
        static C: OnceLock<Mutex<u64>> = OnceLock::new();
        let mut slot = C
            .get_or_init(|| Mutex::new(0))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *slot += 1;
        *slot
    }

    /// Port of `ZshPowerline.init()` from
    /// `powerline/bindings/zsh/__init__.py:118`.
    pub fn new() -> Self {
        Self {
            args: Args::new(),
            id: Self::next_id(),
        }
    }

    /// Port of `ZshPowerline.precmd()` from
    /// `powerline/bindings/zsh/__init__.py:121`.
    pub fn precmd(&mut self, pipe_status: Vec<i32>, last_exit_code: i32) {
        // py:121  def precmd(self):
        // py:122  self.args.last_pipe_status = zsh.pipestatus()
        // py:123  self.args.last_exit_code = zsh.last_exit_code()
        self.args.last_pipe_status = pipe_status;
        self.args.last_exit_code = last_exit_code;
    }

    /// Port of `ZshPowerline.do_setup()` from
    /// `powerline/bindings/zsh/__init__.py:125`.
    ///
    /// Registers the powerline in the used_powerlines set. Returns
    /// the list of `(psvar, side, theme, above)` set_prompt arg
    /// tuples the caller wires through the zsh binding.
    pub fn do_setup(&self) -> Vec<(&'static str, &'static str, Option<&'static str>, bool)> {
        // py:125  def do_setup(self, zsh_globals):
        // py:126  set_prompt(self, 'PS1', 'left', None, above=True)
        // py:127  set_prompt(self, 'RPS1', 'right', None)
        // py:128  set_prompt(self, 'PS2', 'left', 'continuation')
        // py:129  set_prompt(self, 'RPS2', 'right', 'continuation')
        // py:130  set_prompt(self, 'PS3', 'left', 'select')
        let prompts = vec![
            ("PS1", "left", None, true),
            ("RPS1", "right", None, false),
            ("PS2", "left", Some("continuation"), false),
            ("RPS2", "right", Some("continuation"), false),
            ("PS3", "left", Some("select"), false),
        ];
        // py:131  used_powerlines[id(self)] = self
        // py:132  zsh_globals['_powerline'] = self
        used_powerlines()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(self.id);
        prompts
    }
}

/// Port of `setup()` from
/// `powerline/bindings/zsh/__init__.py:217`.
///
/// Constructs the ZshPowerline and returns it; caller wires the
/// atexit hook (Python: `atexit.register(shutdown)` at py:220).
pub fn setup_entry() -> ZshPowerline {
    // py:218  powerline = ZshPowerline()
    // py:219  powerline.setup(zsh_globals)  (stubbed)
    // py:220  atexit.register(shutdown)     (caller-wired)
    ZshPowerline::new()
}

/// Port of `reload()` from
/// `powerline/bindings/zsh/__init__.py:206`.
///
/// `reload_each` is the caller-supplied per-instance reload closure
/// (Python: `powerline.reload()`).
pub fn reload<F>(mut reload_each: F)
where
    F: FnMut(u64),
{
    // py:207-208  for powerline in used_powerlines.values(): powerline.reload()
    let ids: Vec<u64> = used_powerlines()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .copied()
        .collect();
    for id in ids {
        reload_each(id);
    }
}

/// Port of `reload_config()` from
/// `powerline/bindings/zsh/__init__.py:211`.
///
/// `reload_each` is the caller-supplied per-instance renderer-create
/// closure (Python: `powerline.create_renderer(load_main=True,
/// load_colors=True, load_colorscheme=True, load_theme=True)`).
pub fn reload_config<F>(mut reload_each: F)
where
    F: FnMut(u64),
{
    // py:212-213
    let ids: Vec<u64> = used_powerlines()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .copied()
        .collect();
    for id in ids {
        reload_each(id);
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

    fn reset_used() {
        used_powerlines()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    #[test]
    fn args_ext_matches_upstream() {
        // py:41  ext = ['shell']
        assert_eq!(Args::ext(), vec!["shell"]);
    }

    #[test]
    fn args_renderer_module_matches_upstream() {
        // py:42  renderer_module = '.zsh'
        assert_eq!(Args::RENDERER_MODULE, ".zsh");
    }

    #[test]
    fn args_new_defaults_to_empty_pipe_status_and_zero_exit_code() {
        let a = Args::new();
        assert!(a.last_pipe_status.is_empty());
        assert_eq!(a.last_exit_code, 0);
    }

    #[test]
    fn get_var_config_none_returns_none() {
        assert!(get_var_config(None).is_none());
    }

    #[test]
    fn get_var_config_dict_parses_via_parsedotval() {
        // py:30-32  isinstance dict → mergeargs([parsedotval...])
        let mut inner = Map::new();
        inner.insert("foo".to_string(), Value::String("bar".into()));
        let r = get_var_config(Some(Value::Object(inner)));
        assert!(r.is_some());
        // foo=bar produces {"foo": "bar"}-shaped overlay
        let m = r.unwrap();
        assert_eq!(m.get("foo"), Some(&Value::String("bar".into())));
    }

    #[test]
    fn get_var_config_string_parses_via_parse_override_var() {
        // py:33-34  isinstance str → mergeargs(parse_override_var)
        let r = get_var_config(Some(Value::String("common.term_truecolor=true".into())));
        assert!(r.is_some());
        let m = r.unwrap();
        // overlay should contain "common.term_truecolor=true" nested form
        let common = m.get("common").and_then(|v| v.as_object()).unwrap();
        assert_eq!(common.get("term_truecolor"), Some(&Value::Bool(true)));
    }

    #[test]
    fn get_var_config_unknown_type_returns_none() {
        // py:35-36  else: return None
        assert!(get_var_config(Some(Value::from(42))).is_none());
        assert!(get_var_config(Some(Value::Null)).is_none());
    }

    #[test]
    fn config_path_from_var_none_returns_none() {
        assert!(config_path_from_var(None).is_none());
    }

    #[test]
    fn config_path_from_var_string_splits_on_colon() {
        // py:60-62  ret.split(':')
        let r = config_path_from_var(Some(Value::String("/etc/p:/usr/share".into())));
        assert_eq!(
            r,
            Some(vec!["/etc/p".to_string(), "/usr/share".to_string()])
        );
    }

    #[test]
    fn config_path_from_var_string_filters_empty_entries() {
        let r = config_path_from_var(Some(Value::String("::/a::/b:".into())));
        assert_eq!(r, Some(vec!["/a".to_string(), "/b".to_string()]));
    }

    #[test]
    fn config_path_from_var_list_preserves_entries() {
        // py:64-65  else: return ret (list form)
        let arr = Value::Array(vec![
            Value::String("/p1".into()),
            Value::String("/p2".into()),
        ]);
        let r = config_path_from_var(Some(arr));
        assert_eq!(r, Some(vec!["/p1".to_string(), "/p2".to_string()]));
    }

    #[test]
    fn string_decodes_bytes_to_utf8() {
        // py:72-75  bytes → decode; replace errors
        assert_eq!(string(b"hello"), "hello");
        let r = string(&[b'a', 0xff, b'b']);
        assert!(r.contains('a'));
        assert!(r.contains('b'));
    }

    #[test]
    fn environment_get_returns_value_when_present() {
        let r = Environment::get(|| Some("alice".to_string()), None);
        assert_eq!(r, Some("alice".to_string()));
    }

    #[test]
    fn environment_get_returns_default_when_absent() {
        let r = Environment::get(|| None, Some("default".to_string()));
        assert_eq!(r, Some("default".to_string()));
    }

    #[test]
    fn environment_contains_true_when_present() {
        assert!(Environment::contains(|| Some("x".to_string())));
    }

    #[test]
    fn environment_contains_false_when_absent() {
        assert!(!Environment::contains(|| None));
    }

    #[test]
    fn set_prompt_zpyvar_name_prepends_zpython_powerline() {
        // py:201  zpyvar = 'ZPYTHON_POWERLINE_' + psvar
        assert_eq!(set_prompt_zpyvar_name("PS1"), "ZPYTHON_POWERLINE_PS1");
        assert_eq!(set_prompt_zpyvar_name("RPS2"), "ZPYTHON_POWERLINE_RPS2");
    }

    #[test]
    fn build_segment_info_contains_all_expected_keys() {
        let info = build_segment_info(
            "%_state",
            "~/p",
            Some("normal"),
            Some("normal"),
            1,
            Some("continuation"),
        );
        assert!(info.contains_key("parser_state"));
        assert!(info.contains_key("shortened_path"));
        assert!(info.contains_key("mode"));
        assert!(info.contains_key("default_mode"));
        assert!(info.contains_key("client_id"));
        assert!(info.contains_key("local_theme"));
        assert_eq!(info["parser_state"], "%_state");
        assert_eq!(info["client_id"], 1);
    }

    #[test]
    fn build_segment_info_none_mode_becomes_null() {
        let info = build_segment_info("", "", None, None, 1, None);
        assert_eq!(info["mode"], Value::Null);
        assert_eq!(info["default_mode"], Value::Null);
        assert_eq!(info["local_theme"], Value::Null);
    }

    #[test]
    fn prompt_new_stores_state() {
        let p = Prompt::new(
            "left",
            Some("theme1".to_string()),
            Some("PS1".to_string()),
            None,
            true,
        );
        assert_eq!(p.side, "left");
        assert!(p.above);
        assert_eq!(p.savedpsvar, Some("PS1".to_string()));
        assert!(p.savedps.is_none());
        assert_eq!(p.theme, Some("theme1".to_string()));
    }

    #[test]
    fn zsh_powerline_new_assigns_unique_id() {
        let _g = lock_globals!();
        let a = ZshPowerline::new();
        let b = ZshPowerline::new();
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn zsh_powerline_precmd_sets_args() {
        let mut p = ZshPowerline::new();
        p.precmd(vec![0, 1, 2], 42);
        assert_eq!(p.args.last_pipe_status, vec![0, 1, 2]);
        assert_eq!(p.args.last_exit_code, 42);
    }

    #[test]
    fn zsh_powerline_do_setup_emits_five_prompt_pairs() {
        // py:126-130  PS1, RPS1, PS2, RPS2, PS3
        let p = ZshPowerline::new();
        let prompts = p.do_setup();
        assert_eq!(prompts.len(), 5);
        let psvars: Vec<&str> = prompts.iter().map(|t| t.0).collect();
        assert_eq!(psvars, vec!["PS1", "RPS1", "PS2", "RPS2", "PS3"]);
    }

    #[test]
    fn zsh_powerline_do_setup_marks_above_only_for_ps1() {
        // py:126  set_prompt(self, 'PS1', 'left', None, above=True)
        let p = ZshPowerline::new();
        let prompts = p.do_setup();
        assert!(prompts[0].3); // PS1 above
        for t in &prompts[1..] {
            assert!(!t.3);
        }
    }

    #[test]
    fn zsh_powerline_do_setup_registers_in_used_powerlines() {
        let _g = lock_globals!();
        reset_used();
        let p = ZshPowerline::new();
        let _ = p.do_setup();
        let set = used_powerlines().lock().unwrap_or_else(|e| e.into_inner());
        assert!(set.contains(&p.id));
    }

    #[test]
    fn shutdown_iterates_registered_powerlines() {
        let _g = lock_globals!();
        reset_used();
        let p = ZshPowerline::new();
        let id = p.id;
        let _ = p.do_setup();
        let mut shutdown_calls: Vec<u64> = Vec::new();
        shutdown(|ident| shutdown_calls.push(ident));
        assert!(shutdown_calls.contains(&id));
    }

    #[test]
    fn reload_iterates_registered_powerlines() {
        let _g = lock_globals!();
        reset_used();
        let p = ZshPowerline::new();
        let id = p.id;
        let _ = p.do_setup();
        let mut reload_calls: Vec<u64> = Vec::new();
        reload(|ident| reload_calls.push(ident));
        assert!(reload_calls.contains(&id));
    }

    #[test]
    fn reload_config_iterates_registered_powerlines() {
        let _g = lock_globals!();
        reset_used();
        let p = ZshPowerline::new();
        let id = p.id;
        let _ = p.do_setup();
        let mut reload_calls: Vec<u64> = Vec::new();
        reload_config(|ident| reload_calls.push(ident));
        assert!(reload_calls.contains(&id));
    }

    #[test]
    fn setup_entry_returns_fresh_powerline() {
        let p = setup_entry();
        assert!(p.id > 0);
    }

    #[test]
    fn args_config_override_delegates_to_get_var_config() {
        // py:44
        let cfg = serde_json::json!({"foo": "bar"});
        let r = args_config_override(Some(cfg)).unwrap();
        assert_eq!(r.get("foo"), Some(&Value::String("bar".into())));
    }

    #[test]
    fn args_config_override_none_input_returns_none() {
        assert!(args_config_override(None).is_none());
    }

    #[test]
    fn args_theme_override_delegates_to_get_var_config() {
        // py:48
        let cfg = serde_json::json!({"key": "value"});
        let r = args_theme_override(Some(cfg)).unwrap();
        assert_eq!(r.get("key"), Some(&Value::String("value".into())));
    }

    #[test]
    fn args_config_path_splits_string_on_colon() {
        // py:59-65
        let v = Value::String("/etc/powerline:/home/user/.config/powerline".into());
        let r = args_config_path(Some(v)).unwrap();
        assert_eq!(
            r,
            vec![
                "/etc/powerline".to_string(),
                "/home/user/.config/powerline".to_string()
            ]
        );
    }

    #[test]
    fn args_config_path_filters_empty_segments() {
        // py:62-63  if path
        let v = Value::String(":/etc/powerline::/home/user:".into());
        let r = args_config_path(Some(v)).unwrap();
        assert_eq!(
            r,
            vec!["/etc/powerline".to_string(), "/home/user".to_string()]
        );
    }

    #[test]
    fn args_config_path_passes_array_through() {
        // py:65-66
        let v = Value::Array(vec![Value::String("/a".into()), Value::String("/b".into())]);
        let r = args_config_path(Some(v)).unwrap();
        assert_eq!(r, vec!["/a".to_string(), "/b".to_string()]);
    }

    #[test]
    fn args_config_path_none_input_returns_none() {
        // py:55-57  IndexError → None
        assert!(args_config_path(None).is_none());
    }

    #[test]
    fn args_jobnum_passes_through() {
        // py:70
        assert_eq!(args_jobnum(Some(3)), Some(3));
        assert!(args_jobnum(None).is_none());
    }

    #[test]
    fn zsh_expand_fallback_builds_local_var_command() {
        // py:109-114
        let (eval_cmd, get_key, set_key, set_val) = zsh_expand_fallback_steps("${(%):-%~}");
        assert_eq!(eval_cmd, "local _POWERLINE_REPLY=\"${(%):-%~}\"");
        assert_eq!(get_key, "_POWERLINE_REPLY");
        assert_eq!(set_key, "_POWERLINE_REPLY");
        assert!(set_val.is_none());
    }

    #[test]
    fn prompt_del_restore_returns_var_value_pair() {
        // py:194-195
        let p = Prompt::new(
            "left",
            None,
            Some("PS1".to_string()),
            Some("$ ".to_string()),
            false,
        );
        let r = p.del_restore().unwrap();
        assert_eq!(r.0, "PS1");
        assert_eq!(r.1, "$ ");
    }

    #[test]
    fn prompt_del_restore_returns_none_when_no_savedps() {
        let p = Prompt::new("left", None, Some("PS1".to_string()), None, false);
        assert!(p.del_restore().is_none());
    }

    #[test]
    fn prompt_del_restore_returns_none_when_savedps_empty() {
        // py:194  if self.savedps — empty string is falsy in Python
        let p = Prompt::new(
            "left",
            None,
            Some("PS1".to_string()),
            Some("".to_string()),
            false,
        );
        assert!(p.del_restore().is_none());
    }
}
