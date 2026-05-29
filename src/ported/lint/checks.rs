// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/checks.py`.
//!
//! Validators used by `powerline-lint` to check the powerline
//! configuration files. The Python source defines ~30
//! `check_*` validation functions that emit `echoerr(...)`
//! diagnostics via the DelayedEchoErr accumulator, plus the static
//! key-set tables they reference.
//!
//! Rust port surfaces:
//!   - `generic_keys()` / `type_keys()` / `required_keys()` /
//!     `highlight_keys()` accessor functions for the Python
//!     module-level sets at py:24-44
//!   - `list_sep` constant for py:21 JStr(', ')
//!   - `get_function_strings(name, default_module)` rpartition
//!     helper for py:47
//!   - `common_names` registry + `register_common_name` for
//!     py:755-762
//!   - `LintResult` tuple shape mirroring (proceed, echo,
//!     hadproblem) and (proceed, hadproblem) returns
//!
//! The full validation closures (check_segment_function /
//! check_highlight_groups / check_args / etc.) are deferred —
//! they take the full echoerr+context+data dispatch chain and
//! walk the lint pipeline. Surface them here as type signatures
//! ready for the future port pass.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// import logging                                   // py:6
// from collections import defaultdict              // py:8
// from powerline.lib.threaded import ThreadedSegment                                       // py:10
// from powerline.lib.unicode import unicode         // py:11
// from powerline.lint.markedjson.markedvalue import MarkedUnicode                          // py:12
// from powerline.lint.markedjson.error import DelayedEchoErr, Mark                          // py:13
// from powerline.lint.selfcheck import havemarks    // py:14
// from powerline.lint.context import JStr, list_themes                                     // py:15
// from powerline.lint.imp import WithPath, import_function, import_segment                 // py:16
// from powerline.lint.spec import Spec              // py:17
// from powerline.lint.inspect import getconfigargspec                                       // py:18

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// Port of `list_sep = JStr(', ')` from
/// `powerline/lint/checks.py:21`.
pub const LIST_SEP: &str = ", ";

/// Port of `generic_keys` from
/// `powerline/lint/checks.py:24-32`.
pub fn generic_keys() -> &'static HashSet<&'static str> {
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| {
        // py:25-31  generic_keys set
        let mut s = HashSet::new();
        s.insert("exclude_modes");
        s.insert("include_modes");
        s.insert("exclude_function");
        s.insert("include_function");
        s.insert("width");
        s.insert("align");
        s.insert("name");
        s.insert("draw_soft_divider");
        s.insert("draw_hard_divider");
        s.insert("priority");
        s.insert("after");
        s.insert("before");
        s.insert("display");
        s
    })
}

/// Port of `type_keys` from
/// `powerline/lint/checks.py:33-37`.
///
/// Returns the per-segment-type allowed keys table. The Rust port
/// uses fixed slices since the table is read-only.
pub fn type_keys() -> &'static std::collections::HashMap<&'static str, HashSet<&'static str>> {
    static M: OnceLock<std::collections::HashMap<&'static str, HashSet<&'static str>>> =
        OnceLock::new();
    M.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        // py:34  'function' → {function, args, draw_inner_divider}
        let mut function_keys = HashSet::new();
        function_keys.insert("function");
        function_keys.insert("args");
        function_keys.insert("draw_inner_divider");
        m.insert("function", function_keys);
        // py:35  'string' → {contents, type, highlight_groups, divider_highlight_group}
        let mut string_keys = HashSet::new();
        string_keys.insert("contents");
        string_keys.insert("type");
        string_keys.insert("highlight_groups");
        string_keys.insert("divider_highlight_group");
        m.insert("string", string_keys);
        // py:36  'segment_list' → {function, segments, args, type}
        let mut segment_list_keys = HashSet::new();
        segment_list_keys.insert("function");
        segment_list_keys.insert("segments");
        segment_list_keys.insert("args");
        segment_list_keys.insert("type");
        m.insert("segment_list", segment_list_keys);
        m
    })
}

/// Port of `required_keys` from
/// `powerline/lint/checks.py:38-42`.
pub fn required_keys() -> &'static std::collections::HashMap<&'static str, HashSet<&'static str>> {
    static M: OnceLock<std::collections::HashMap<&'static str, HashSet<&'static str>>> =
        OnceLock::new();
    M.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        // py:39  'function' → {'function'}
        let mut function_req = HashSet::new();
        function_req.insert("function");
        m.insert("function", function_req);
        // py:40  'string' → {}
        m.insert("string", HashSet::new());
        // py:41  'segment_list' → {'function', 'segments'}
        let mut segment_list_req = HashSet::new();
        segment_list_req.insert("function");
        segment_list_req.insert("segments");
        m.insert("segment_list", segment_list_req);
        m
    })
}

/// Port of `highlight_keys` from
/// `powerline/lint/checks.py:43`.
pub fn highlight_keys() -> &'static HashSet<&'static str> {
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| {
        // py:43  {'highlight_groups', 'name'}
        let mut s = HashSet::new();
        s.insert("highlight_groups");
        s.insert("name");
        s
    })
}

/// Port of `get_function_strings()` from
/// `powerline/lint/checks.py:47`.
///
/// `function_name` is the raw config string; `default_module` is the
/// fallback module (Python: `context[0][1].get('default_module',
/// 'powerline.segments.' + ext)`).
///
/// Returns the resolved `(module, function_name)` pair.
pub fn get_function_strings(function_name: &str, default_module: &str) -> (String, String) {
    // py:48-54  rpartition on '.'
    if let Some(dot_idx) = function_name.rfind('.') {
        let (module, rest) = function_name.split_at(dot_idx);
        let function = &rest[1..];
        (module.to_string(), function.to_string())
    } else {
        (default_module.to_string(), function_name.to_string())
    }
}

/// Tuple returned by `check_func` lint helpers per
/// `powerline/lint/checks.py:684`.
///
/// `proceed` controls whether the caller continues; `echo` controls
/// whether check_func echoes the diagnostic itself; `hadproblem`
/// reports whether the check found errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LintResult {
    pub proceed: bool,
    pub echo: bool,
    pub hadproblem: bool,
}

impl LintResult {
    pub fn ok() -> Self {
        Self {
            proceed: true,
            echo: false,
            hadproblem: false,
        }
    }

    pub fn failed() -> Self {
        Self {
            proceed: false,
            echo: true,
            hadproblem: true,
        }
    }

    pub fn warned() -> Self {
        Self {
            proceed: true,
            echo: true,
            hadproblem: true,
        }
    }
}

/// Port of `common_names = defaultdict(set)` from
/// `powerline/lint/checks.py:755`.
///
/// Maps a logical name to the set of `(module, name)` qualified
/// pairs that resolve to it.
pub fn common_names() -> &'static Mutex<std::collections::HashMap<String, HashSet<(String, String)>>>
{
    static M: OnceLock<Mutex<std::collections::HashMap<String, HashSet<(String, String)>>>> =
        OnceLock::new();
    M.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

/// Port of `register_common_name()` from
/// `powerline/lint/checks.py:758`.
pub fn register_common_name(
    name: impl Into<String>,
    cmodule: impl Into<String>,
    cname: impl Into<String>,
) {
    // py:759-762  common_names[name].add((cmodule, cname))
    let mut map = common_names().lock().unwrap_or_else(|e| e.into_inner());
    map.entry(name.into())
        .or_default()
        .insert((cmodule.into(), cname.into()));
}

/// Port of `check_log_file_level()` from
/// `powerline/lint/checks.py:805`.
///
/// Validates that the supplied logging level is at least as
/// critical as the top_level. Both are case-sensitive Python
/// logging level names (`DEBUG`, `INFO`, `WARNING`, `ERROR`,
/// `CRITICAL`).
///
/// Returns `(proceed, echo, hadproblem)` per py:822-836.
pub fn check_log_file_level(this_level: &str, top_level: &str) -> LintResult {
    // py:812-815  both levels must be valid logging level names
    let log_levels: std::collections::HashMap<&str, i32> = [
        ("CRITICAL", 50),
        ("ERROR", 40),
        ("WARNING", 30),
        ("INFO", 20),
        ("DEBUG", 10),
        ("NOTSET", 0),
    ]
    .into_iter()
    .collect();
    let top_val = match log_levels.get(top_level) {
        Some(v) => *v,
        None => return LintResult::ok(),
    };
    let this_val = match log_levels.get(this_level) {
        Some(v) => *v,
        None => return LintResult::ok(),
    };
    // py:827  if this_level < top_level: emit problem
    if this_val < top_val {
        LintResult::warned()
    } else {
        LintResult::ok()
    }
}

/// Port of `check_logging_handler()` from
/// `powerline/lint/checks.py:838`.
///
/// Validates that the given handler name resolves to a real
/// `logging.Handler` subclass. Rust port maintains a known-handler
/// set covering the upstream logging.handlers module names.
pub fn check_logging_handler(handler_name: &str) -> LintResult {
    // py:839-864  handler must exist in logging.handlers
    let known_handlers: HashSet<&str> = [
        "StreamHandler",
        "FileHandler",
        "NullHandler",
        "WatchedFileHandler",
        "BaseRotatingHandler",
        "RotatingFileHandler",
        "TimedRotatingFileHandler",
        "SocketHandler",
        "DatagramHandler",
        "SysLogHandler",
        "SMTPHandler",
        "NTEventLogHandler",
        "HTTPHandler",
        "BufferingHandler",
        "MemoryHandler",
        "QueueHandler",
        "QueueListener",
    ]
    .into_iter()
    .collect();
    if known_handlers.contains(handler_name) {
        LintResult::ok()
    } else {
        LintResult::failed()
    }
}

/// Port of `check_color()` from
/// `powerline/lint/checks.py:152`.
///
/// Validates that `color` looks like a colorscheme reference. The
/// Python source resolves it against the colors config; the Rust
/// port checks the structural shape (must be a string with no
/// whitespace).
pub fn check_color(color: &str) -> LintResult {
    // py:153-164  color must be a non-empty string without spaces
    if color.is_empty() || color.contains(char::is_whitespace) {
        LintResult::failed()
    } else {
        LintResult::ok()
    }
}

/// Port of `check_hl_group_name()` from
/// `powerline/lint/checks.py:354`.
///
/// Validates a highlight-group name. The Python source enforces an
/// identifier-like pattern.
pub fn check_hl_group_name(hl_group: &str) -> LintResult {
    // py:354-369  must be a valid identifier
    if hl_group.is_empty() {
        return LintResult::failed();
    }
    let first = hl_group.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return LintResult::failed();
    }
    for c in hl_group.chars().skip(1) {
        if !c.is_ascii_alphanumeric() && c != '_' && c != ':' {
            return LintResult::failed();
        }
    }
    LintResult::ok()
}

/// Port of `check_ext()` from
/// `powerline/lint/checks.py:98-116`.
///
/// Validates that `ext` is in the available extensions list AND that
/// at least one of `themes` / `colorschemes` is configured for it.
///
/// Returns `(hadsomedirs, hadproblem)` per py:116 — note this is the
/// only check_* in the file that returns a 2-tuple rather than the
/// 3-tuple LintResult.
pub fn check_ext(
    ext: &str,
    available_exts: &HashSet<&str>,
    has_themes_for_ext: bool,
    has_colorschemes_for_ext: bool,
    has_top_themes: bool,
    has_top_colorschemes: bool,
) -> (bool, bool) {
    // py:102-106  if ext not in data['lists']['exts']
    if !available_exts.contains(ext) {
        return (false, true);
    }
    // py:108-115  walk ('themes', 'colorschemes')
    let mut hadsomedirs = false;
    let mut hadproblem = false;
    // themes
    if !has_themes_for_ext && !has_top_themes {
        hadproblem = true;
    } else {
        hadsomedirs = true;
    }
    // colorschemes
    if !has_colorschemes_for_ext && !has_top_colorschemes {
        hadproblem = true;
    } else {
        hadsomedirs = true;
    }
    (hadsomedirs, hadproblem)
}

/// Port of `check_config()` from
/// `powerline/lint/checks.py:119-138`.
///
/// Validates that the given `theme` is configured for `ext` either
/// in the per-ext config (`configs[d][ext]`) or in the top-level
/// (`configs['top_' + d]`).
///
/// `d` is the config-kind ("themes" or "colorschemes"). Returns the
/// LintResult triple per py:138.
pub fn check_config(
    d: &str,
    theme: &str,
    ext: &str,
    available_exts: &HashSet<&str>,
    has_theme_for_ext: bool,
    has_top_theme: bool,
) -> LintResult {
    // py:125-129
    if !available_exts.contains(ext) {
        return LintResult::warned();
    }
    // py:130-137
    let _ = (d, theme);
    if has_theme_for_ext || has_top_theme {
        LintResult::ok()
    } else {
        // py:136  echoerr + return True, False, True
        LintResult {
            proceed: true,
            echo: false,
            hadproblem: true,
        }
    }
}

/// Port of `check_top_theme()` from
/// `powerline/lint/checks.py:141-149`.
///
/// Validates that `theme` is in the available top_themes set.
pub fn check_top_theme(theme: &str, top_themes: &HashSet<&str>) -> LintResult {
    // py:143  if theme not in data['configs']['top_themes']
    if top_themes.contains(theme) {
        LintResult::ok()
    } else {
        // py:144-148  echoerr + return True, False, True
        LintResult {
            proceed: true,
            echo: false,
            hadproblem: true,
        }
    }
}

/// Port of `check_translated_group_name()` from
/// `powerline/lint/checks.py:166-167`.
///
/// Python: `return check_group(group, data, context, echoerr)` —
/// pass-through. The Rust port surfaces the call shape; the full
/// `check_group` cascade (py:170-243) walks the colorscheme config
/// tree and is deferred.
pub fn check_translated_group_name(group: &str, defined_groups: &HashSet<&str>) -> LintResult {
    // py:167  return check_group(...)
    check_group(group, defined_groups)
}

/// Port of `check_group()` from
/// `powerline/lint/checks.py:170-243`.
///
/// Validates that the group name resolves in the colorscheme config.
/// The Rust port takes the resolved set of available group names
/// directly (Python walks `data['ext_colorscheme_configs']` etc.).
pub fn check_group(group: &str, defined_groups: &HashSet<&str>) -> LintResult {
    // py:172-173  if not isinstance(group, unicode): return True, False, False
    // (string is always valid as &str)
    // py:212-242  check if group exists in any of the configs
    if defined_groups.contains(group) {
        LintResult::ok()
    } else {
        // py:233-240  echoerr + return True, False, True
        LintResult {
            proceed: true,
            echo: false,
            hadproblem: true,
        }
    }
}

/// Port of `check_key_compatibility()` from
/// `powerline/lint/checks.py:245-291`.
///
/// Validates the keys of a segment dict against the allowed key
/// sets for its `type`. `segment_keys` is the set of keys the
/// segment config dict carries; `segment_type` is the resolved
/// type ("function", "string", or "segment_list").
pub fn check_key_compatibility(segment_keys: &HashSet<&str>, segment_type: &str) -> LintResult {
    // py:250-254  if segment_type not in type_keys: fail
    let tk = match type_keys().get(segment_type) {
        Some(s) => s,
        None => {
            return LintResult {
                proceed: false,
                echo: false,
                hadproblem: true,
            }
        }
    };

    let mut hadproblem = false;
    // py:259-268  unknown keys not in (generic_keys | type_keys[t])
    let gk = generic_keys();
    for k in segment_keys.iter() {
        if !gk.contains(k) && !tk.contains(k) {
            hadproblem = true;
            break;
        }
    }
    // py:270-278  required keys must all be present
    if let Some(rk) = required_keys().get(segment_type) {
        for k in rk.iter() {
            if !segment_keys.contains(k) {
                hadproblem = true;
                break;
            }
        }
    }
    // py:280-289  type != 'function' and (keys & highlight_keys) must be non-empty
    if segment_type != "function" {
        let hk = highlight_keys();
        let mut has_hl = false;
        for k in segment_keys.iter() {
            if hk.contains(k) {
                has_hl = true;
                break;
            }
        }
        if !has_hl {
            hadproblem = true;
        }
    }

    LintResult {
        proceed: true,
        echo: false,
        hadproblem,
    }
}

/// Port of `check_segment_module()` from
/// `powerline/lint/checks.py:294-306`.
///
/// Validates that the module name is importable. The Rust port can't
/// actually call Python's `__import__`; it accepts a closure
/// `is_importable` so callers (test harness or runtime importer)
/// can supply the lookup.
pub fn check_segment_module(module: &str, is_importable: impl Fn(&str) -> bool) -> LintResult {
    // py:297-298  __import__(str(module))
    if is_importable(module) {
        LintResult::ok()
    } else {
        // py:299-305  ImportError → echoerr + return True, False, True
        LintResult {
            proceed: true,
            echo: false,
            hadproblem: true,
        }
    }
}

/// Port of `check_exinclude_function()` from
/// `powerline/lint/checks.py:794-802`.
///
/// Validates that `name` resolves as a selector function. Python
/// uses `rpartition` then defaults the module to
/// `powerline.selectors.<ext>` if absent.
///
/// Returns the resolved `(module, name)` pair via the out parameter
/// `resolved` plus the lint result. Callers use the pair to look up
/// the actual function via `import_function`.
pub fn check_exinclude_function(name: &str, ext: &str) -> (String, String) {
    // py:796  module, name = name.rpartition('.')[::2]
    let (module, function) = match name.rfind('.') {
        Some(idx) => (name[..idx].to_string(), name[idx + 1..].to_string()),
        None => (String::new(), name.to_string()),
    };
    // py:797-798  if not module: module = 'powerline.selectors.' + ext
    let module = if module.is_empty() {
        format!("powerline.selectors.{}", ext)
    } else {
        module
    };
    (module, function)
}

/// Port of `get_one_segment_function()` from
/// `powerline/lint/checks.py:747-754`.
///
/// Yields the segment's function name resolved via
/// `get_function_strings`. Rust port returns the resolved
/// `(module, function)` pair when the function key is set.
pub fn get_one_segment_function(
    function_name: Option<&str>,
    ext: &str,
) -> Option<(String, String)> {
    // py:749  function_name = context[-2][1].get('function')
    let function_name = function_name?;
    // py:751  module, function_name = get_function_strings(function_name, context, ext)
    let default_module = format!("powerline.segments.{}", ext);
    Some(get_function_strings(function_name, &default_module))
}

/// Port of `check_matcher_func()` from
/// `powerline/lint/checks.py:56-95`.
///
/// Resolves the matcher function name and returns the `(module,
/// function)` pair after defaulting to `powerline.matchers.<ext>`.
pub fn check_matcher_func(ext: &str, match_name: &str) -> (String, String) {
    // py:60  match_module, separator, match_function = match_name.rpartition('.')
    match match_name.rfind('.') {
        Some(idx) => (
            match_name[..idx].to_string(),
            match_name[idx + 1..].to_string(),
        ),
        // py:61-63  if not separator: match_module = 'powerline.matchers.<ext>'
        None => (
            format!("powerline.matchers.{}", ext),
            match_name.to_string(),
        ),
    }
}

/// Port of `check_highlight_group()` from
/// `powerline/lint/checks.py:604-616`.
///
/// Wrapper that validates a single highlight group exists in the
/// colorscheme. The full `hl_exists` chain (py:585-602) walks the
/// colorscheme + gradients; the Rust port takes the resolved
/// available-group set.
pub fn check_highlight_group(hl_group: &str, available_groups: &HashSet<&str>) -> LintResult {
    // py:614  hl_exists(hl_group, ...)
    if available_groups.contains(hl_group) {
        LintResult::ok()
    } else {
        LintResult {
            proceed: true,
            echo: false,
            hadproblem: true,
        }
    }
}

/// Port of `check_highlight_groups()` from
/// `powerline/lint/checks.py:618-637`.
///
/// Validates a list of highlight groups; all must exist OR at
/// least one must exist with hadproblem=true per py:631-636.
pub fn check_highlight_groups(hl_groups: &[&str], available_groups: &HashSet<&str>) -> LintResult {
    // py:621-632  any group missing → hadproblem
    let mut hadproblem = false;
    for hl in hl_groups {
        if !available_groups.contains(hl) {
            hadproblem = true;
            break;
        }
    }
    LintResult {
        proceed: true,
        echo: false,
        hadproblem,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Module-scoped lock that serializes tests against the
    /// process-wide common_names registry.
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    macro_rules! lock_globals {
        () => {{
            TEST_LOCK
                .get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    fn reset_common_names() {
        let mut map = common_names().lock().unwrap_or_else(|e| e.into_inner());
        map.clear();
    }

    #[test]
    fn list_sep_matches_upstream() {
        // py:21  JStr(', ')
        assert_eq!(LIST_SEP, ", ");
    }

    #[test]
    fn generic_keys_has_13_entries() {
        // py:25-31  13 keys
        let g = generic_keys();
        assert_eq!(g.len(), 13);
        assert!(g.contains("exclude_modes"));
        assert!(g.contains("priority"));
        assert!(g.contains("display"));
    }

    #[test]
    fn type_keys_function_includes_args() {
        let t = type_keys();
        let function_keys = t.get("function").unwrap();
        assert!(function_keys.contains("function"));
        assert!(function_keys.contains("args"));
        assert!(function_keys.contains("draw_inner_divider"));
    }

    #[test]
    fn type_keys_string_includes_contents() {
        let t = type_keys();
        let string_keys = t.get("string").unwrap();
        assert!(string_keys.contains("contents"));
        assert!(string_keys.contains("highlight_groups"));
        assert!(string_keys.contains("divider_highlight_group"));
    }

    #[test]
    fn type_keys_segment_list_includes_segments() {
        let t = type_keys();
        let sl_keys = t.get("segment_list").unwrap();
        assert!(sl_keys.contains("function"));
        assert!(sl_keys.contains("segments"));
    }

    #[test]
    fn required_keys_function_requires_function() {
        // py:39
        let r = required_keys();
        let fn_req = r.get("function").unwrap();
        assert!(fn_req.contains("function"));
        assert_eq!(fn_req.len(), 1);
    }

    #[test]
    fn required_keys_string_has_no_required() {
        // py:40
        let r = required_keys();
        let str_req = r.get("string").unwrap();
        assert!(str_req.is_empty());
    }

    #[test]
    fn required_keys_segment_list_requires_function_and_segments() {
        // py:41
        let r = required_keys();
        let sl_req = r.get("segment_list").unwrap();
        assert!(sl_req.contains("function"));
        assert!(sl_req.contains("segments"));
        assert_eq!(sl_req.len(), 2);
    }

    #[test]
    fn highlight_keys_contains_highlight_groups_and_name() {
        // py:43
        let h = highlight_keys();
        assert!(h.contains("highlight_groups"));
        assert!(h.contains("name"));
    }

    #[test]
    fn get_function_strings_dotted_name_splits() {
        // py:48-51  rpartition on '.'
        let (m, f) = get_function_strings("foo.bar.baz", "powerline.segments.shell");
        assert_eq!(m, "foo.bar");
        assert_eq!(f, "baz");
    }

    #[test]
    fn get_function_strings_undotted_uses_default_module() {
        // py:51-54  default to 'powerline.segments.' + ext
        let (m, f) = get_function_strings("plain", "powerline.segments.shell");
        assert_eq!(m, "powerline.segments.shell");
        assert_eq!(f, "plain");
    }

    #[test]
    fn lint_result_ok_is_proceed_true_no_echo_no_problem() {
        let r = LintResult::ok();
        assert!(r.proceed);
        assert!(!r.echo);
        assert!(!r.hadproblem);
    }

    #[test]
    fn lint_result_failed_is_proceed_false_echo_true_problem() {
        let r = LintResult::failed();
        assert!(!r.proceed);
        assert!(r.echo);
        assert!(r.hadproblem);
    }

    #[test]
    fn lint_result_warned_proceeds_but_echoes_problem() {
        let r = LintResult::warned();
        assert!(r.proceed);
        assert!(r.echo);
        assert!(r.hadproblem);
    }

    #[test]
    fn register_common_name_inserts_into_registry() {
        let _g = lock_globals!();
        reset_common_names();
        register_common_name("uptime", "powerline.segments.common.sys", "uptime");
        let map = common_names().lock().unwrap_or_else(|e| e.into_inner());
        assert!(map.contains_key("uptime"));
        let entries = map.get("uptime").unwrap();
        assert_eq!(entries.len(), 1);
        let pair = entries.iter().next().unwrap();
        assert_eq!(pair.0, "powerline.segments.common.sys");
        assert_eq!(pair.1, "uptime");
    }

    #[test]
    fn register_common_name_dedupes_duplicates() {
        let _g = lock_globals!();
        reset_common_names();
        register_common_name("x", "a", "b");
        register_common_name("x", "a", "b");
        let map = common_names().lock().unwrap_or_else(|e| e.into_inner());
        let entries = map.get("x").unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn register_common_name_supports_multiple_aliases() {
        let _g = lock_globals!();
        reset_common_names();
        register_common_name("x", "mod1", "fn_a");
        register_common_name("x", "mod2", "fn_b");
        let map = common_names().lock().unwrap_or_else(|e| e.into_inner());
        let entries = map.get("x").unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn check_log_file_level_below_top_warns() {
        // py:827  this_level < top_level → emit problem
        let r = check_log_file_level("DEBUG", "WARNING");
        assert!(r.hadproblem);
        assert!(r.proceed);
    }

    #[test]
    fn check_log_file_level_at_or_above_top_is_ok() {
        let r = check_log_file_level("ERROR", "WARNING");
        assert!(!r.hadproblem);
        let r = check_log_file_level("WARNING", "WARNING");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_log_file_level_invalid_level_is_ok() {
        // py:817-819  fall-through when level name not in logging module
        let r = check_log_file_level("BOGUS", "WARNING");
        assert!(!r.hadproblem);
        let r = check_log_file_level("DEBUG", "BOGUS");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_logging_handler_known_handler_is_ok() {
        // py:839-862  standard logging.handlers
        let r = check_logging_handler("StreamHandler");
        assert!(r.proceed);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_logging_handler_unknown_handler_fails() {
        let r = check_logging_handler("BogusHandler");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_color_accepts_simple_name() {
        let r = check_color("solarized_red");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_color_rejects_empty() {
        let r = check_color("");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_color_rejects_whitespace() {
        let r = check_color("red blue");
        assert!(r.hadproblem);
        let r = check_color("red\t");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_hl_group_name_accepts_identifier() {
        let r = check_hl_group_name("branch_clean");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_hl_group_name_accepts_colon() {
        // py: highlight groups support ':' for sub-classification
        let r = check_hl_group_name("workspace:focused");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_hl_group_name_rejects_empty() {
        let r = check_hl_group_name("");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_hl_group_name_rejects_leading_digit() {
        let r = check_hl_group_name("123foo");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_hl_group_name_accepts_underscore_prefix() {
        let r = check_hl_group_name("_private");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_ext_known_ext_with_themes_is_ok() {
        // py:102-115
        let exts: HashSet<&str> = ["shell", "tmux", "vim"].into_iter().collect();
        let (had_dirs, had_problem) = check_ext("shell", &exts, true, true, false, false);
        assert!(had_dirs);
        assert!(!had_problem);
    }

    #[test]
    fn check_ext_unknown_ext_returns_problem() {
        // py:102-106
        let exts: HashSet<&str> = ["shell"].into_iter().collect();
        let (had_dirs, had_problem) = check_ext("bogus", &exts, false, false, false, false);
        assert!(!had_dirs);
        assert!(had_problem);
    }

    #[test]
    fn check_ext_falls_back_to_top_themes() {
        // py:109  if ext not in configs[typ] AND not configs['top_' + typ]
        let exts: HashSet<&str> = ["shell"].into_iter().collect();
        let (had_dirs, _) = check_ext("shell", &exts, false, false, true, true);
        assert!(had_dirs);
    }

    #[test]
    fn check_config_known_ext_with_theme_ok() {
        // py:130-138
        let exts: HashSet<&str> = ["shell"].into_iter().collect();
        let r = check_config("themes", "default", "shell", &exts, true, false);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_config_unknown_ext_warns() {
        let exts: HashSet<&str> = ["shell"].into_iter().collect();
        let r = check_config("themes", "default", "bogus", &exts, false, false);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_config_missing_theme_emits_problem() {
        let exts: HashSet<&str> = ["shell"].into_iter().collect();
        let r = check_config("themes", "missing", "shell", &exts, false, false);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_top_theme_known_theme_is_ok() {
        // py:143-149
        let themes: HashSet<&str> = ["default", "tmux"].into_iter().collect();
        let r = check_top_theme("default", &themes);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_top_theme_unknown_theme_has_problem() {
        let themes: HashSet<&str> = ["default"].into_iter().collect();
        let r = check_top_theme("bogus", &themes);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_group_known_group_is_ok() {
        // py:212-242
        let groups: HashSet<&str> = ["branch", "background"].into_iter().collect();
        let r = check_group("branch", &groups);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_group_unknown_group_has_problem() {
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let r = check_group("nonexistent", &groups);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_translated_group_name_delegates_to_check_group() {
        // py:167
        let groups: HashSet<&str> = ["foo"].into_iter().collect();
        let r1 = check_translated_group_name("foo", &groups);
        let r2 = check_group("foo", &groups);
        assert_eq!(r1, r2);
    }

    #[test]
    fn check_key_compatibility_function_segment_with_valid_keys() {
        // py:259-289
        let keys: HashSet<&str> = ["function", "args", "name", "priority"]
            .into_iter()
            .collect();
        let r = check_key_compatibility(&keys, "function");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_key_compatibility_string_segment_requires_highlight() {
        // py:280-289  type != 'function' requires highlight_groups or name
        let keys: HashSet<&str> = ["contents"].into_iter().collect();
        let r = check_key_compatibility(&keys, "string");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_key_compatibility_string_segment_with_name_ok() {
        let keys: HashSet<&str> = ["contents", "name"].into_iter().collect();
        let r = check_key_compatibility(&keys, "string");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_key_compatibility_function_missing_function_key() {
        // py:270-278  required keys check
        let keys: HashSet<&str> = ["args"].into_iter().collect();
        let r = check_key_compatibility(&keys, "function");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_key_compatibility_unknown_segment_type_fails() {
        // py:250-254
        let keys: HashSet<&str> = ["function"].into_iter().collect();
        let r = check_key_compatibility(&keys, "bogus_type");
        assert!(!r.proceed);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_key_compatibility_unknown_key_emits_problem() {
        // py:259-268
        let keys: HashSet<&str> = ["function", "bogus_key"].into_iter().collect();
        let r = check_key_compatibility(&keys, "function");
        assert!(r.hadproblem);
    }

    #[test]
    fn check_segment_module_importable_is_ok() {
        // py:297-306
        let r = check_segment_module("powerline.segments.shell", |_| true);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_segment_module_unimportable_has_problem() {
        let r = check_segment_module("definitely.not.a.real.module", |_| false);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_exinclude_function_dotted_name_splits() {
        // py:796-798
        let (m, n) = check_exinclude_function("powerline.selectors.vim.in_help", "vim");
        assert_eq!(m, "powerline.selectors.vim");
        assert_eq!(n, "in_help");
    }

    #[test]
    fn check_exinclude_function_undotted_defaults_module() {
        // py:797-798
        let (m, n) = check_exinclude_function("in_help", "vim");
        assert_eq!(m, "powerline.selectors.vim");
        assert_eq!(n, "in_help");
    }

    #[test]
    fn get_one_segment_function_returns_pair_when_set() {
        // py:747-754
        let r = get_one_segment_function(Some("powerline.segments.shell.uptime"), "shell");
        assert_eq!(
            r,
            Some(("powerline.segments.shell".to_string(), "uptime".to_string()))
        );
    }

    #[test]
    fn get_one_segment_function_returns_none_when_unset() {
        // py:749-754  no function_name → no yield
        assert_eq!(get_one_segment_function(None, "shell"), None);
    }

    #[test]
    fn get_one_segment_function_undotted_defaults_module() {
        let r = get_one_segment_function(Some("uptime"), "shell");
        assert_eq!(
            r,
            Some(("powerline.segments.shell".to_string(), "uptime".to_string()))
        );
    }

    #[test]
    fn check_matcher_func_dotted_name_splits() {
        // py:60
        let (m, n) = check_matcher_func("vim", "powerline.matchers.vim.help");
        assert_eq!(m, "powerline.matchers.vim");
        assert_eq!(n, "help");
    }

    #[test]
    fn check_matcher_func_undotted_defaults_module() {
        // py:61-63
        let (m, n) = check_matcher_func("vim", "help");
        assert_eq!(m, "powerline.matchers.vim");
        assert_eq!(n, "help");
    }

    #[test]
    fn check_highlight_group_known_group_ok() {
        // py:604-616
        let groups: HashSet<&str> = ["background", "branch"].into_iter().collect();
        let r = check_highlight_group("background", &groups);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_highlight_group_unknown_group_has_problem() {
        let groups: HashSet<&str> = ["background"].into_iter().collect();
        let r = check_highlight_group("nonexistent", &groups);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_highlight_groups_all_known_is_ok() {
        // py:621-636
        let groups: HashSet<&str> = ["a", "b", "c"].into_iter().collect();
        let r = check_highlight_groups(&["a", "b"], &groups);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_highlight_groups_any_missing_has_problem() {
        let groups: HashSet<&str> = ["a"].into_iter().collect();
        let r = check_highlight_groups(&["a", "b"], &groups);
        assert!(r.hadproblem);
    }
}
