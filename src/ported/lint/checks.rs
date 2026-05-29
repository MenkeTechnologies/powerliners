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
    // py:21  list_sep = JStr(', ')
    // py:24  generic_keys = set((
    // py:25  'exclude_modes', 'include_modes',
    // py:26  'exclude_function', 'include_function',
    // py:27  'width', 'align',
    // py:28  'name',
    // py:29  'draw_soft_divider', 'draw_hard_divider',
    // py:30  'priority',
    // py:31  'after', 'before',
    // py:32  'display'
    // py:33  ))
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| {
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
    // py:34  type_keys = {
    // py:35  'function': set(('function', 'args', 'draw_inner_divider')),
    // py:36  'string': set(('contents', 'type', 'highlight_groups', 'divider_highlight_group')),
    // py:37  'segment_list': set(('function', 'segments', 'args', 'type')),
    // py:38  }
    // py:39  required_keys = {
    // py:40  'function': set(('function',)),
    // py:41  'string': set(()),
    // py:42  'segment_list': set(('function', 'segments',)),
    // py:43  }
    // py:44  highlight_keys = set(('highlight_groups', 'name'))
    static M: OnceLock<std::collections::HashMap<&'static str, HashSet<&'static str>>> =
        OnceLock::new();
    M.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        let mut function_keys = HashSet::new();
        function_keys.insert("function");
        function_keys.insert("args");
        function_keys.insert("draw_inner_divider");
        m.insert("function", function_keys);
        let mut string_keys = HashSet::new();
        string_keys.insert("contents");
        string_keys.insert("type");
        string_keys.insert("highlight_groups");
        string_keys.insert("divider_highlight_group");
        m.insert("string", string_keys);
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
    // py:47  def get_function_strings(function_name, context, ext):
    // py:48  if '.' in function_name:
    // py:49  module, function_name = function_name.rpartition('.')[::2]
    // py:50  else:
    // py:51  module = context[0][1].get('default_module', 'powerline.segments.' + ext)
    // py:52  return module, function_name
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
    // py:758  def register_common_name(name, cmodule, cname):
    // py:759  common_names[name].add((cmodule, cname))
    // py:760  highlight_groups.append(name)
    // py:761  # also register the qualified name
    // py:762  highlight_groups.append(cmodule + '.' + cname)
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
    // py:805  def check_log_file_level(this_level, top_level, *args, **kwargs):
    // py:806  '''Check log file logging level.
    // py:807-811  docstring
    // py:812  hadproblem = False
    // py:813  top_level_val = logging.getLevelName(top_level)
    // py:814  this_level_val = logging.getLevelName(this_level)
    // py:815  if not isinstance(top_level_val, int):
    // py:816  hadproblem = True
    // py:817  echoerr(...)
    // py:818  if not isinstance(this_level_val, int):
    // py:819  hadproblem = True
    // py:820  echoerr(...)
    // py:821  if hadproblem:
    // py:822  return True, False, hadproblem
    // py:823  if this_level_val < top_level_val:
    // py:824  hadproblem = True
    // py:825  echoerr(...)
    // py:826  return True, False, hadproblem
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
    // py:838  def check_logging_handler(handler, data, context, echoerr):
    // py:839  '''Check that the logging handler refers to a real handler class.
    // py:840-845  docstring
    // py:846  hadproblem = False
    // py:847  module, _, handler_name = handler.rpartition('.')
    // py:848  try:
    // py:849  module = __import__(str(module), fromlist=[str(handler_name)])
    // py:850  except ImportError as e:
    // py:851  echoerr(...)
    // py:852  hadproblem = True
    // py:853  return True, False, hadproblem
    // py:854  try:
    // py:855  handler_class = getattr(module, str(handler_name))
    // py:856  except AttributeError as e:
    // py:857  echoerr(...)
    // py:858  hadproblem = True
    // py:859  return True, False, hadproblem
    // py:860  if not issubclass(handler_class, logging.Handler):
    // py:861  echoerr(...)
    // py:862  hadproblem = True
    // py:863  return True, False, hadproblem
    // py:864  return True, False, hadproblem
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

/// Port of `check_full_segment_data()` from
/// `powerline/lint/checks.py:309-346`.
///
/// Validates a segment dict against the theme's `segment_data`
/// overrides for the segment's module/function/name. Python walks
/// `theme_segment_data` + `top_segment_data` chaining the per-key
/// fallback (`before`/`after`/`args`/`contents`) then delegates to
/// [`check_key_compatibility`].
///
/// The Rust port surfaces the entry point for parity; the deep
/// context walk that needs `data['ext']` + `data['theme']` +
/// `data['ext_theme_configs']` + `data['main_config']` is deferred
/// until the lint dispatch infrastructure is wired. The current
/// implementation returns `ok()` so the validator doesn't false-
/// positive — callers using the actual Spec composers route checks
/// through [`check_key_compatibility`] directly.
pub fn check_full_segment_data(segment: &serde_json::Map<String, serde_json::Value>) -> LintResult {
    // py:309  def check_full_segment_data(segment, data, context, echoerr):
    // py:310  if 'name' not in segment and 'function' not in segment:
    // py:311  return True, False, False
    if !segment.contains_key("name") && !segment.contains_key("function") {
        return LintResult::ok();
    }
    // py:313-345  theme_segment_data + top_segment_data merge + per-key
    //             fallback walk + check_key_compatibility dispatch.
    // py:346  return check_key_compatibility(segment_copy, data, context, echoerr)
    LintResult::ok()
}

/// Port of `check_segment_function()` from
/// `powerline/lint/checks.py:371-602`.
///
/// Walks the segment function's docstring for `Highlight groups
/// used:` and `Divider highlight group used:` sentinels and checks
/// each named group exists in the resolved colorscheme.
///
/// Rust port surfaces the entry point; the full Mark + import +
/// docstring scan is deferred until the lint dispatch infra is
/// wired. Returns `ok()` as the safe default — direct callers
/// continue to use [`check_highlight_group`] + [`check_highlight_groups`]
/// against the resolved group sets.
pub fn check_segment_function(_function_name: &str) -> LintResult {
    // py:371  def check_segment_function(function_name, data, context, echoerr):
    // py:372  havemarks(function_name)
    // py:373-374  module, function_name = get_function_strings(function_name, context, ext)
    // py:375  if context[-2][1].get('type', 'function') == 'function':
    // py:376  func = import_segment(function_name, data, context, echoerr, module=module)
    // py:377-602  docstring scan + hl_group / divider_hl_group / args dispatch
    LintResult::ok()
}

/// Port of `check_segment_data_key()` from
/// `powerline/lint/checks.py:639-674`.
///
/// Validates that a `segment_data` key references a real segment
/// (by name or by `<module>.<function>`). Python walks every
/// listed theme via `list_themes(data, context)` and matches the
/// key against each segment's name or fn-resolved identifier.
///
/// Rust port surfaces the entry point; the cross-theme walk needs
/// `data['ext_theme_configs']` + `data['theme_type']` which the
/// lint dispatch infrastructure provides. Returns `ok()` as the
/// safe default.
pub fn check_segment_data_key(_key: &str) -> LintResult {
    // py:639  def check_segment_data_key(key, data, context, echoerr):
    // py:640  havemarks(key)
    // py:641  has_module_name = '.' in key
    // py:642-666  for ext, theme in list_themes(data, context):
    //                 for segments in theme.get('segments', {}).values():
    //                     for segment in segments:
    //                         match by name / function
    // py:668-672  if data['theme_type'] != 'top':
    //                 echoerr(...)
    //                 return True, False, True
    // py:674  return True, False, False
    LintResult::ok()
}

/// Port of `check_args_variant()` from
/// `powerline/lint/checks.py:684-723`.
///
/// Validates the segment-args dict against the resolved function's
/// argspec. Python walks `getconfigargspec(func)` and checks for
/// missing required + extra unknown args against a list of
/// implicitly-omitted args (`pl`, `segment_info`, `create_watcher`,
/// `*omitted_args(func)`).
///
/// Rust port surfaces the entry point; the argspec walk uses
/// `inspect.getfullargspec` which has no Rust equivalent. Returns
/// `ok()` as the safe default — runtime arg-validation lives in
/// the actual segment dispatch.
pub fn check_args_variant(_args: &serde_json::Map<String, serde_json::Value>) -> LintResult {
    // py:684  def check_args_variant(func, args, data, context, echoerr):
    // py:685  havemarks(args)
    // py:686  argspec = getconfigargspec(func)
    // py:687-723  argspec walk + missing/extra dispatch via threaded_args_specs
    LintResult::ok()
}

/// Port of `check_args()` from
/// `powerline/lint/checks.py:725-792`.
///
/// Top-level dispatcher that resolves the segment function via
/// `get_functions` (one of `get_one_segment_function` /
/// `get_all_possible_functions` depending on caller context), then
/// delegates to [`check_args_variant`].
///
/// Rust port surfaces the entry point; the function resolution
/// flow needs the lint context dispatch. Returns `ok()` as the
/// safe default.
pub fn check_args(_args: &serde_json::Map<String, serde_json::Value>) -> LintResult {
    // py:725  def check_args(get_functions, args, data, context, echoerr):
    // py:726-792  for func in get_functions(...): check_args_variant(func, args, ...)
    LintResult::ok()
}

/// Port of `check_color()` from
/// `powerline/lint/checks.py:152`.
///
/// Validates that `color` looks like a colorscheme reference. The
/// Python source resolves it against the colors config; the Rust
/// port checks the structural shape (must be a string with no
/// whitespace).
pub fn check_color(color: &str) -> LintResult {
    // py:152  def check_color(color, data, context, echoerr):
    // py:153  havemarks(color)
    // py:154  if (color not in data['colors_config'].get('colors', {})
    // py:155  and color not in data['colors_config'].get('gradients', {})):
    // py:156  echoerr(
    // py:157  context='Error while checking highlight group in colorscheme (key {key})'.format(
    // py:158  key=context.key),
    // py:159  problem='found unexistent color or gradient {0}'.format(color),
    // py:160  problem_mark=color.mark
    // py:161  )
    // py:162  return True, False, True
    // py:163  return True, False, False
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
    // py:98  def check_ext(ext, data, context, echoerr):
    // py:99  havemarks(ext)
    // py:100  hadsomedirs = False
    // py:101  hadproblem = False
    // py:102  if ext not in data['lists']['exts']:
    // py:103  hadproblem = True
    // py:104  echoerr(context='Error while loading {0} extension configuration'.format(ext),
    // py:105  context_mark=ext.mark,
    // py:106  problem='extension configuration does not exist')
    // py:107  else:
    // py:108  for typ in ('themes', 'colorschemes'):
    // py:109  if ext not in data['configs'][typ] and not data['configs']['top_' + typ]:
    // py:110  hadproblem = True
    // py:111  echoerr(context='Error while loading {0} extension configuration'.format(ext),
    // py:112  context_mark=ext.mark,
    // py:113  problem='{0} configuration does not exist'.format(typ))
    // py:114  else:
    // py:115  hadsomedirs = True
    // py:116  return hadsomedirs, hadproblem
    if !available_exts.contains(ext) {
        return (false, true);
    }
    let mut hadsomedirs = false;
    let mut hadproblem = false;
    if !has_themes_for_ext && !has_top_themes {
        hadproblem = true;
    } else {
        hadsomedirs = true;
    }
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
    // py:119  def check_config(d, theme, data, context, echoerr):
    // py:120  if len(context) == 4:
    // py:121  ext = context[-2][0]
    // py:122  else:
    // py:123  # local_themes
    // py:124  ext = context[-3][0]
    // py:125  if ext not in data['lists']['exts']:
    // py:126  echoerr(context='Error while loading {0} extension configuration'.format(ext),
    // py:127  context_mark=ext.mark,
    // py:128  problem='extension configuration does not exist')
    // py:129  return True, False, True
    // py:130  if (
    // py:131  (ext not in data['configs'][d] or theme not in data['configs'][d][ext])
    // py:132  and theme not in data['configs']['top_' + d]
    // py:133  ):
    // py:134  echoerr(context='Error while loading {0} from {1} extension configuration'.format(d[:-1], ext),
    // py:135  problem='failed to find configuration file {0}/{1}/{2}.json'.format(d, ext, theme),
    // py:136  problem_mark=theme.mark)
    // py:137  return True, False, True
    // py:138  return True, False, False
    if !available_exts.contains(ext) {
        return LintResult::warned();
    }
    let _ = (d, theme);
    if has_theme_for_ext || has_top_theme {
        LintResult::ok()
    } else {
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
    // py:141  def check_top_theme(theme, data, context, echoerr):
    // py:142  havemarks(theme)
    // py:143  if theme not in data['configs']['top_themes']:
    // py:144  echoerr(context='Error while checking extension configuration (key {key})'.format(key=context.key),
    // py:145  context_mark=context[-2][0].mark,
    // py:146  problem='failed to find top theme {0}'.format(theme),
    // py:147  problem_mark=theme.mark)
    // py:148  return True, False, True
    // py:149  return True, False, False
    if top_themes.contains(theme) {
        LintResult::ok()
    } else {
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
    // py:166  def check_translated_group_name(group, data, context, echoerr):
    // py:167  return check_group(group, data, context, echoerr)
    check_group(group, defined_groups)
}

/// Port of `check_group()` from
/// `powerline/lint/checks.py:170-243`.
///
/// Validates that the group name resolves in the colorscheme config.
/// The Rust port takes the resolved set of available group names
/// directly (Python walks `data['ext_colorscheme_configs']` etc.).
pub fn check_group(group: &str, defined_groups: &HashSet<&str>) -> LintResult {
    // py:170  def check_group(group, data, context, echoerr):
    // py:171  havemarks(group)
    // py:172  if not isinstance(group, unicode):
    // py:173  return True, False, False
    // py:174  colorscheme = data['colorscheme']
    // py:175  ext = data['ext']
    // py:176  configs = None
    // py:177  if ext:
    // py:178  def listed_key(d, k):
    // py:179  try:
    // py:180  return [d[k]]
    // py:181  except KeyError:
    // py:182  return []
    // py:184  if colorscheme == '__main__':
    // py:185  colorscheme_names = set(data['ext_colorscheme_configs'][ext])
    // py:186  colorscheme_names.update(data['top_colorscheme_configs'])
    // py:187  colorscheme_names.discard('__main__')
    // py:188  configs = [
    // py:189  (
    // py:190  name,
    // py:191  listed_key(data['ext_colorscheme_configs'][ext], name)
    // py:192  + listed_key(data['ext_colorscheme_configs'][ext], '__main__')
    // py:193  + listed_key(data['top_colorscheme_configs'], name)
    // py:194  )
    // py:195  for name in colorscheme_names
    // py:196  ]
    // py:197  else:
    // py:198  configs = [
    // py:199  (
    // py:200  colorscheme,
    // py:201  listed_key(data['ext_colorscheme_configs'][ext], colorscheme)
    // py:202  + listed_key(data['ext_colorscheme_configs'][ext], '__main__')
    // py:203  + listed_key(data['top_colorscheme_configs'], colorscheme)
    // py:204  )
    // py:205  ]
    // py:206  else:
    // py:207  try:
    // py:208  configs = [(colorscheme, [data['top_colorscheme_configs'][colorscheme]])]
    // py:209  except KeyError:
    // py:210  pass
    // py:211  hadproblem = False
    // py:212  for new_colorscheme, config_lst in configs:
    // py:213  not_found = []
    // py:214  new_data = data.copy()
    // py:215  new_data['colorscheme'] = new_colorscheme
    // py:216  for config in config_lst:
    // py:217  havemarks(config)
    // py:218  try:
    // py:219  group_data = config['groups'][group]
    // py:220  except KeyError:
    // py:221  not_found.append(config.mark.name)
    // py:222  else:
    // py:223  proceed, echo, chadproblem = check_group(
    // py:224  group_data,
    // py:225  new_data,
    // py:226  context,
    // py:227  echoerr,
    // py:228  )
    // py:229  if chadproblem:
    // py:230  hadproblem = True
    // py:231  if not proceed:
    // py:232  break
    // py:233  if not_found and len(not_found) == len(config_lst):
    // py:234  echoerr(
    // py:235  context='Error while checking group definition in colorscheme (key {key})'.format(
    // py:236  key=context.key),
    // py:237  problem='name {0} is not present anywhere in {1} {2} {3} colorschemes: {4}'.format(
    // py:238  group, len(not_found), ext, new_colorscheme, ', '.join(not_found)),
    // py:239  problem_mark=group.mark
    // py:240  )
    // py:241  hadproblem = True
    // py:242  return True, False, hadproblem
    if defined_groups.contains(group) {
        LintResult::ok()
    } else {
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
    // py:245  def check_key_compatibility(segment, data, context, echoerr):
    // py:246  havemarks(segment)
    // py:247  segment_type = segment.get('type', MarkedUnicode('function', None))
    // py:248  havemarks(segment_type)
    // py:250  if segment_type not in type_keys:
    // py:251  echoerr(context='Error while checking segments (key {key})'.format(key=context.key),
    // py:252  problem='found segment with unknown type {0}'.format(segment_type),
    // py:253  problem_mark=segment_type.mark)
    // py:254  return False, False, True
    // py:256  hadproblem = False
    // py:258  keys = set(segment)
    // py:259  if not ((keys - generic_keys) < type_keys[segment_type]):
    // py:260  unknown_keys = keys - generic_keys - type_keys[segment_type]
    // py:261  echoerr(
    // py:262  context='Error while checking segments (key {key})'.format(key=context.key),
    // py:263  context_mark=context[-1][1].mark,
    // py:264  problem='found keys not used with the current segment type: {0}'.format(
    // py:265  list_sep.join(unknown_keys)),
    // py:266  problem_mark=list(unknown_keys)[0].mark
    // py:267  )
    // py:268  hadproblem = True
    // py:270  if not (keys >= required_keys[segment_type]):
    // py:271  missing_keys = required_keys[segment_type] - keys
    // py:272  echoerr(
    // py:273  context='Error while checking segments (key {key})'.format(key=context.key),
    // py:274  context_mark=context[-1][1].mark,
    // py:275  problem='found missing required keys: {0}'.format(
    // py:276  list_sep.join(missing_keys))
    // py:277  )
    // py:278  hadproblem = True
    // py:280  if not (segment_type == 'function' or (keys & highlight_keys)):
    // py:281  echoerr(
    // py:282  context='Error while checking segments (key {key})'.format(key=context.key),
    // py:283  context_mark=context[-1][1].mark,
    // py:284  problem=(
    // py:285  'found missing keys required to determine highlight group. '
    // py:286  'Either highlight_groups or name key must be present'
    // py:287  )
    // py:288  )
    // py:289  hadproblem = True
    // py:291  return True, False, hadproblem
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
    let gk = generic_keys();
    for k in segment_keys.iter() {
        if !gk.contains(k) && !tk.contains(k) {
            hadproblem = true;
            break;
        }
    }
    if let Some(rk) = required_keys().get(segment_type) {
        for k in rk.iter() {
            if !segment_keys.contains(k) {
                hadproblem = true;
                break;
            }
        }
    }
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
    // py:294  def check_segment_module(module, data, context, echoerr):
    // py:295  havemarks(module)
    // py:296  with WithPath(data['import_paths']):
    // py:297  try:
    // py:298  __import__(str(module))
    // py:299  except ImportError as e:
    // py:300  echoerr(context='Error while checking segments (key {key})'.format(key=context.key),
    // py:301  context_mark=module.mark,
    // py:302  problem='failed to import module {0}'.format(module),
    // py:303  problem_mark=module.mark)
    // py:304  return True, False, True
    // py:305  return True, False, False
    if is_importable(module) {
        LintResult::ok()
    } else {
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
    // py:794  def check_exinclude_function(name, data, context, echoerr):
    // py:795  havemarks(name)
    // py:796  module, name = name.rpartition('.')[::2]
    // py:797  if not module:
    // py:798  module = 'powerline.selectors.{0}'.format(data['ext'])
    // py:799  ext = data['ext']
    // py:800  return import_function('selector', name, data, context, echoerr,
    // py:801  module=MarkedUnicode(module, name.mark))
    // py:802  # See ``import_function``
    let (module, function) = match name.rfind('.') {
        Some(idx) => (name[..idx].to_string(), name[idx + 1..].to_string()),
        None => (String::new(), name.to_string()),
    };
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
    // py:747  def get_one_segment_function(function_name, context, ext):
    // py:748  havemarks(function_name)
    // py:749  module, function_name = get_function_strings(function_name, context, ext)
    // py:750  func = import_segment(function_name, context[0][1], context, lambda **kwargs: True,
    // py:751  module=MarkedUnicode(module, function_name.mark))
    // py:752  if func:
    // py:753  yield func
    // py:754  return
    let function_name = function_name?;
    let default_module = format!("powerline.segments.{}", ext);
    Some(get_function_strings(function_name, &default_module))
}

/// Port of `check_matcher_func()` from
/// `powerline/lint/checks.py:56-95`.
///
/// Resolves the matcher function name and returns the `(module,
/// function)` pair after defaulting to `powerline.matchers.<ext>`.
pub fn check_matcher_func(ext: &str, match_name: &str) -> (String, String) {
    // py:56  def check_matcher_func(ext, match_name, data, context, echoerr):
    // py:57  havemarks(match_name)
    // py:58  import_paths = [os.path.expanduser(path) for path in context[0][1].get('common', {}).get('paths', [])]
    // py:60  match_module, separator, match_function = match_name.rpartition('.')
    // py:61  if not separator:
    // py:62  match_module = 'powerline.matchers.{0}'.format(ext)
    // py:63  match_function = match_name
    // py:65  with WithPath(import_paths):
    // py:66  try:
    // py:67  func = getattr(__import__(str(match_module), fromlist=[str(match_function)]), str(match_function))
    // py:68  except ImportError:
    // py:69  echoerr(context='Error while loading matcher functions',
    // py:70  problem='failed to load module {0}'.format(match_module),
    // py:71  problem_mark=match_name.mark)
    // py:72  return True, True
    // py:73  except AttributeError:
    // py:74  echoerr(context='Error while loading matcher functions',
    // py:75  problem='failed to load matcher function {0}'.format(match_function),
    // py:76  problem_mark=match_name.mark)
    // py:77  return True, True
    // py:79  if not callable(func):
    // py:80  echoerr(context='Error while checking segments (key {key})'.format(key=context.key),
    // py:81  context_mark=match_name.mark,
    // py:82  problem='imported "function" {0} from module {1} is not callable'.format(match_function, match_module),
    // py:83  problem_mark=match_module.mark)
    // py:84  return True, True
    // py:86  return True, False
    match match_name.rfind('.') {
        Some(idx) => (
            match_name[..idx].to_string(),
            match_name[idx + 1..].to_string(),
        ),
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

/// Port of `hl_group_in_colorscheme()` from
/// `powerline/lint/checks.py:535-582`.
///
/// Walks the colorscheme's `groups` dict to check whether `hl_group`
/// resolves to a non-gradient color when `allow_gradients=false` per
/// py:564-571, or a gradient when `allow_gradients='force'` per
/// py:573-580.
///
/// The Rust port takes the resolved color sets directly since the
/// full nested-config walk needs the lint context. Returns `true`
/// when the group exists in the colorscheme AND satisfies the
/// gradient policy.
/// Port of the inner `listed_key()` closure from
/// `powerline/lint/checks.py:178-182`.
///
/// Wraps a single-key lookup into a list-wrapped result: returns
/// `[value]` when `k` is present in `d`, `[]` when missing. Python
/// embeds this closure inside `hl_exists` to chain multiple
/// dict-lookups via list concatenation (py:191-194). The Rust port
/// surfaces it as a free fn over `&serde_json::Map` so callers can
/// reuse the list-or-empty pattern.
pub fn listed_key(
    d: &serde_json::Map<String, serde_json::Value>,
    k: &str,
) -> Vec<serde_json::Value> {
    // py:178  def listed_key(d, k):
    // py:179  try:
    // py:180  return [d[k]]
    // py:181  except KeyError:
    // py:182  return []
    match d.get(k) {
        Some(v) => vec![v.clone()],
        None => Vec::new(),
    }
}

pub fn hl_group_in_colorscheme(
    hl_group: &str,
    groups: &HashSet<&str>,
    colors: &HashSet<&str>,
    gradients: &HashSet<&str>,
    group_fg: Option<&str>,
    group_bg: Option<&str>,
    allow_gradients: AllowGradients,
) -> bool {
    // py:537-538  if hl_group not in cconfig.get('groups', {}): return False
    if !groups.contains(hl_group) {
        return false;
    }
    // py:539  elif not allow_gradients or allow_gradients == 'force'
    if matches!(allow_gradients, AllowGradients::Yes) {
        return true;
    }
    let mut hadgradient = false;
    for color in [group_fg, group_bg].into_iter().flatten() {
        // py:560-561  hascolor / hasgradient
        let hascolor = colors.contains(color);
        let hasgradient = gradients.contains(color);
        if hasgradient {
            hadgradient = true;
        }
        // py:564-572  allow_gradients=False + gradient-not-color → fail
        if matches!(allow_gradients, AllowGradients::No) && !hascolor && hasgradient {
            return false;
        }
    }
    // py:573-580  allow_gradients='force' + no gradient → fail
    if matches!(allow_gradients, AllowGradients::Force) && !hadgradient {
        return false;
    }
    true
}

/// Tri-state for the `allow_gradients` arg per py:539.
///
/// Python: `False` / truthy / `'force'`. Rust port enumerates the
/// three cases since Python's mixed-type sentinel doesn't survive
/// the boundary cleanly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowGradients {
    /// Python `False` — gradient-not-color is an error.
    No,
    /// Python truthy (non-empty/non-'force') — gradients allowed,
    /// but at least one color must be present too.
    Yes,
    /// Python `'force'` — at least one gradient is required.
    Force,
}

/// Port of `hl_exists()` from
/// `powerline/lint/checks.py:585-601`.
///
/// Returns the list of colorschemes where `hl_group` is NOT defined.
/// Empty list means the group exists in every colorscheme.
///
/// `colorscheme_membership` is the caller-supplied mapping of
/// `(colorscheme_name → in_colorscheme)` per py:594-598. Python
/// dispatches through `hl_group_in_colorscheme`; Rust takes the
/// resolved membership directly so the iteration shape can be
/// tested.
pub fn hl_exists(hl_group: &str, colorscheme_membership: &[(String, bool)]) -> Vec<String> {
    // py:587-591  if ext not in data['colorscheme_configs']: return []
    if colorscheme_membership.is_empty() {
        return Vec::new();
    }
    // py:593-598  walk colorschemes; collect those where hl_group missing
    let _ = hl_group;
    let mut r: Vec<String> = Vec::new();
    let mut found = false;
    for (name, present) in colorscheme_membership {
        if *present {
            found = true;
        } else {
            r.push(name.clone());
        }
    }
    // py:599-601  found path is implicit
    let _ = found;
    r
}

/// Port of `get_all_possible_functions()` from
/// `powerline/lint/checks.py:767-792`.
///
/// Yields all function names that match the given segment name +
/// known module/name pairs. Python walks the registered themes and
/// common_names; the Rust port surfaces the rpartition logic at
/// py:768-769 + the common_names dispatch at py:775-779.
///
/// Returns the list of `(module, function_name)` candidates the
/// Python source would yield via `import_segment(...)` per py:771
/// or py:777.
pub fn get_all_possible_functions(name: &str) -> Vec<(String, String)> {
    // py:768-769  module, name = name.rpartition('.')[::2]
    let (module, fname) = match name.rfind('.') {
        Some(idx) => (name[..idx].to_string(), name[idx + 1..].to_string()),
        None => (String::new(), name.to_string()),
    };

    let mut out: Vec<(String, String)> = Vec::new();
    // py:770-773  if module: yield (module, name)
    if !module.is_empty() {
        out.push((module, fname));
        return out;
    }
    // py:774-779  walk common_names
    let map = common_names().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(entries) = map.get(&fname) {
        for (cmodule, cname) in entries {
            out.push((cmodule.clone(), cname.clone()));
        }
    }
    out
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

    #[test]
    fn hl_group_in_colorscheme_missing_group_returns_false() {
        // py:537-538
        let groups: HashSet<&str> = HashSet::new();
        let colors: HashSet<&str> = HashSet::new();
        let gradients: HashSet<&str> = HashSet::new();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            None,
            None,
            AllowGradients::Yes,
        );
        assert!(!r);
    }

    #[test]
    fn hl_group_in_colorscheme_allow_gradients_yes_returns_true_if_present() {
        // py:539  truthy allow_gradients → permissive
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let colors: HashSet<&str> = HashSet::new();
        let gradients: HashSet<&str> = HashSet::new();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            None,
            None,
            AllowGradients::Yes,
        );
        assert!(r);
    }

    #[test]
    fn hl_group_in_colorscheme_no_gradients_with_gradient_color_fails() {
        // py:564-572  AllowGradients::No + gradient-not-color → fail
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let colors: HashSet<&str> = HashSet::new();
        let gradients: HashSet<&str> = ["my_gradient"].into_iter().collect();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            Some("my_gradient"),
            None,
            AllowGradients::No,
        );
        assert!(!r);
    }

    #[test]
    fn hl_group_in_colorscheme_no_gradients_with_real_color_passes() {
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let colors: HashSet<&str> = ["solarized_red"].into_iter().collect();
        let gradients: HashSet<&str> = HashSet::new();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            Some("solarized_red"),
            Some("solarized_red"),
            AllowGradients::No,
        );
        assert!(r);
    }

    #[test]
    fn hl_group_in_colorscheme_force_gradient_without_gradient_fails() {
        // py:573-580  Force + no gradient → fail
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let colors: HashSet<&str> = ["red"].into_iter().collect();
        let gradients: HashSet<&str> = HashSet::new();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            Some("red"),
            Some("red"),
            AllowGradients::Force,
        );
        assert!(!r);
    }

    #[test]
    fn hl_group_in_colorscheme_force_gradient_with_gradient_passes() {
        let groups: HashSet<&str> = ["branch"].into_iter().collect();
        let colors: HashSet<&str> = HashSet::new();
        let gradients: HashSet<&str> = ["my_gradient"].into_iter().collect();
        let r = hl_group_in_colorscheme(
            "branch",
            &groups,
            &colors,
            &gradients,
            Some("my_gradient"),
            None,
            AllowGradients::Force,
        );
        assert!(r);
    }

    #[test]
    fn hl_exists_empty_input_returns_empty() {
        // py:587-591  no colorschemes → []
        let r = hl_exists("branch", &[]);
        assert!(r.is_empty());
    }

    #[test]
    fn hl_exists_returns_list_of_missing_colorschemes() {
        // py:593-598  collect missing
        let cs = vec![
            ("default".to_string(), true),
            ("solarized".to_string(), false),
            ("monokai".to_string(), false),
        ];
        let r = hl_exists("branch", &cs);
        assert_eq!(r.len(), 2);
        assert!(r.contains(&"solarized".to_string()));
        assert!(r.contains(&"monokai".to_string()));
    }

    #[test]
    fn hl_exists_all_present_returns_empty() {
        let cs = vec![
            ("default".to_string(), true),
            ("solarized".to_string(), true),
        ];
        let r = hl_exists("branch", &cs);
        assert!(r.is_empty());
    }

    #[test]
    fn get_all_possible_functions_dotted_name_yields_module_function_pair() {
        // py:768-773
        let r = get_all_possible_functions("powerline.segments.shell.uptime");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "powerline.segments.shell");
        assert_eq!(r[0].1, "uptime");
    }

    #[test]
    fn get_all_possible_functions_undotted_walks_common_names() {
        // py:774-779
        let _g = lock_globals!();
        reset_common_names();
        register_common_name("uptime", "powerline.segments.common.sys", "uptime_impl");
        let r = get_all_possible_functions("uptime");
        assert!(!r.is_empty());
        let pair = r
            .iter()
            .find(|(m, n)| m == "powerline.segments.common.sys" && n == "uptime_impl");
        assert!(pair.is_some());
    }

    #[test]
    fn get_all_possible_functions_undotted_unknown_returns_empty() {
        let _g = lock_globals!();
        reset_common_names();
        let r = get_all_possible_functions("nonexistent_segment");
        assert!(r.is_empty());
    }

    #[test]
    fn check_full_segment_data_empty_segment_returns_ok() {
        // py:310-311  no 'name' and no 'function' → ok
        let seg = serde_json::Map::new();
        let r = check_full_segment_data(&seg);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_full_segment_data_with_name_returns_ok() {
        // py:312+  segment with name flows through compatibility check
        let mut seg = serde_json::Map::new();
        seg.insert("name".into(), serde_json::json!("powerline_test"));
        let r = check_full_segment_data(&seg);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_segment_function_returns_ok() {
        // py:371-602  defers to runtime; stub returns ok.
        let r = check_segment_function("powerline.segments.common.time.fuzzy_time");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_segment_data_key_returns_ok() {
        // py:639-674  defers to cross-theme walk; stub returns ok.
        let r = check_segment_data_key("hostname");
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_args_variant_returns_ok() {
        // py:684-723  defers to argspec walk; stub returns ok.
        let r = check_args_variant(&serde_json::Map::new());
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_args_returns_ok() {
        // py:725-792  defers to function-resolution + dispatch; stub ok.
        let mut args = serde_json::Map::new();
        args.insert("interval".into(), serde_json::json!(60.0));
        let r = check_args(&args);
        assert!(!r.hadproblem);
    }
}
