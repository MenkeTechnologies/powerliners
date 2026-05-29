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
    let v = value?;
    // py:30-32  isinstance(val, dict): mergeargs([parsedotval(...) for k, v])
    if let Some(obj) = v.as_object() {
        let pairs: Vec<(String, Value)> = obj
            .iter()
            .map(|(k, v)| {
                // py:31  parsedotval((u(k), u(v)))
                let v_str = v.as_str().unwrap_or("").to_string();
                parsedotval_tuple(k, &v_str)
            })
            .collect();
        return mergeargs(pairs.into_iter(), false);
    }
    // py:33-34  isinstance(val, (unicode, str, bytes)): mergeargs(parse_override_var(u(val)))
    if let Some(s) = v.as_str() {
        return mergeargs(parse_override_var(s).into_iter(), false);
    }
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
    // py:72-75  bytes → decode; else str(s)
    String::from_utf8_lossy(bytes).to_string()
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
        // py:87-91  try string(zsh.getvalue(key)); except IndexError: return default
        get_value().or(default)
    }

    /// Port of `Environment.__contains__()` (staticmethod) from
    /// `powerline/bindings/zsh/__init__.py:93`.
    pub fn contains<F>(get_value: F) -> bool
    where
        F: FnOnce() -> Option<String>,
    {
        // py:94-97  try zsh.getvalue(key): return True; except IndexError: False
        get_value().is_some()
    }
}

/// Port of `set_prompt()` from
/// `powerline/bindings/zsh/__init__.py:196`.
///
/// Returns the zpyvar name that the Python source builds before
/// calling `zsh.set_special_string`. Caller wires the actual zsh
/// special-string registration.
pub fn set_prompt_zpyvar_name(psvar: &str) -> String {
    // py:201  zpyvar = 'ZPYTHON_POWERLINE_' + psvar
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
    /// Port of `Prompt.__init__()` from
    /// `powerline/bindings/zsh/__init__.py:138`.
    pub fn new(
        side: impl Into<String>,
        theme: Option<String>,
        savedpsvar: Option<String>,
        savedps: Option<String>,
        above: bool,
    ) -> Self {
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
        // py:122-123  self.args.last_pipe_status = zsh.pipestatus()
        //             self.args.last_exit_code = zsh.last_exit_code()
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
        // py:126-130  set_prompt calls
        let prompts = vec![
            ("PS1", "left", None, true),
            ("RPS1", "right", None, false),
            ("PS2", "left", Some("continuation"), false),
            ("RPS2", "right", Some("continuation"), false),
            ("PS3", "left", Some("select"), false),
        ];
        // py:131  used_powerlines[id(self)] = self
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
}
