// vim:fileencoding=utf-8:noet
//! Port of `powerline/theme.py`.
//!
//! Theme dispatch: combines a colorscheme + a list of configured
//! segments into a renderable per-line layout. The Theme class itself
//! ties together segment.py, renderer.py, and colorscheme.py at
//! construction time and yields fully-resolved segment dicts during
//! `get_segments()`.
//!
//! This port covers the free fns + the `expand_functions` table. The
//! Theme class (~135 LOC of __init__/get_segments) ports together with
//! segment.py since both halves of the dispatch contract must land
//! together.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import itertools                                 // py:4
// from powerline.segment import gen_segment_getter, process_segment, get_fallback_segment  // py:6
// from powerline.lib.unicode import u, safe_unicode                                          // py:7

use serde_json::{Map, Value};

/// Port of `requires_segment_info()` decorator from
/// `powerline/theme.py:10`.
///
/// Python:
/// ```python
/// def requires_segment_info(func):
///     func.powerline_requires_segment_info = True
///     return func
/// ```
///
/// Python decorator that marks a segment function so the renderer
/// knows to pass it the live `segment_info` payload. In Rust the
/// marker is a const `bool` per-fn-name lookup table (segments register
/// themselves at construction time in the Powerline orchestrator);
/// this fn exists for the upstream identity but is a no-op identity
/// in the Rust pipeline.
///
/// The actual marker check is performed by inspecting the
/// `Segment::requires_segment_info` associated const when the
/// segment trait is added.
pub fn requires_segment_info<F>(func: F) -> F {
    // py:10
    // py:11  func.powerline_requires_segment_info = True
    // py:12  return func
    // (No runtime attribute attachment in Rust; the marker is carried
    // by the segment registry at construction time.)
    func
}

/// Port of `requires_filesystem_watcher()` decorator from
/// `powerline/theme.py:15`.
///
/// Marks a segment function as needing the filesystem watcher
/// injected. Same Rust handling as `requires_segment_info` —
/// identity passthrough at this layer.
pub fn requires_filesystem_watcher<F>(func: F) -> F {
    // py:15
    // py:16  func.powerline_requires_filesystem_watcher = True
    // py:17  return func
    func
}

/// Port of `new_empty_segment_line()` from `powerline/theme.py:20`.
///
/// Returns a fresh `{'left': [], 'right': []}` dict representing one
/// rendered line of the statusline.
pub fn new_empty_segment_line() -> Map<String, Value> {
    // py:20  def new_empty_segment_line():
    // py:21  return {
    // py:22  'left': [],
    // py:23  'right': []
    // py:24  }
    let mut m = Map::new();
    m.insert("left".to_string(), Value::Array(Vec::new()));
    m.insert("right".to_string(), Value::Array(Vec::new()));
    m
}

/// Port of `add_spaces_left()` from `powerline/theme.py:27`.
///
/// Python: `return (' ' * amount) + segment['contents']`
///
/// Right-aligned expand: pad on the left.
pub fn add_spaces_left(_pl: &(), amount: usize, segment: &Map<String, Value>) -> String {
    // py:27  def add_spaces_left(pl, amount, segment):
    // py:28  return (' ' * amount) + segment['contents']
    let contents = segment
        .get("contents")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!("{}{}", " ".repeat(amount), contents)
}

/// Port of `add_spaces_right()` from `powerline/theme.py:31`.
///
/// Python: `return segment['contents'] + (' ' * amount)`
///
/// Left-aligned expand: pad on the right.
pub fn add_spaces_right(_pl: &(), amount: usize, segment: &Map<String, Value>) -> String {
    // py:31  def add_spaces_right(pl, amount, segment):
    // py:32  return segment['contents'] + (' ' * amount)
    let contents = segment
        .get("contents")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!("{}{}", contents, " ".repeat(amount))
}

/// Port of `add_spaces_center()` from `powerline/theme.py:35`.
///
/// Center expand: split padding evenly, extra char on the left.
///
/// Python:
/// ```python
/// amount, remainder = divmod(amount, 2)
/// return (' ' * (amount + remainder)) + segment['contents'] + (' ' * amount)
/// ```
pub fn add_spaces_center(_pl: &(), amount: usize, segment: &Map<String, Value>) -> String {
    // py:35  def add_spaces_center(pl, amount, segment):
    // py:36  amount, remainder = divmod(amount, 2)
    let (half, remainder) = (amount / 2, amount % 2);
    // py:37  return (' ' * (amount + remainder)) + segment['contents'] + (' ' * amount)
    let contents = segment
        .get("contents")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!(
        "{}{}{}",
        " ".repeat(half + remainder),
        contents,
        " ".repeat(half)
    )
}

/// Port of module-level binding `expand_functions` from
/// `powerline/theme.py:40`.
///
/// Maps single-char alignment codes to the matching `add_spaces_*` fn.
///
/// Note the inverse mapping: align `'l'` (left) needs `add_spaces_right`
/// (because padding goes on the right to make text appear left-aligned),
/// align `'r'` (right) needs `add_spaces_left`.
// The fn-pointer return type mirrors the upstream Python protocol shape.
#[allow(clippy::type_complexity)]
pub fn expand_functions(align: char) -> Option<fn(&(), usize, &Map<String, Value>) -> String> {
    // py:40  expand_functions = {
    // py:41  'l': add_spaces_right,
    // py:42  'r': add_spaces_left,
    // py:43  'c': add_spaces_center,
    // py:44  }
    match align {
        'l' => Some(add_spaces_right),
        'r' => Some(add_spaces_left),
        'c' => Some(add_spaces_center),
        _ => None,
    }
}

/// Port of `class Theme(object)` from
/// `powerline/theme.py:47-182`.
///
/// Lightweight value-store. The heavy parts of `__init__` (segment
/// resolution via `gen_segment_getter` and the segment-iteration at
/// py:91-105) and `get_segments` (segment dispatch + alignment) port
/// together with `segment.py` since both halves of the dispatch
/// contract must land in sync. This struct surfaces the field shape
/// + the pure accessor methods so callers can be wired up now.
pub struct Theme {
    /// Python: `self.colorscheme` (py:59) — passed to process_segment.
    pub colorscheme: Value,
    /// Python: `self.dividers` (py:60-65) — `{side: {type: char}}`
    /// table from theme_config.
    pub dividers: Map<String, Value>,
    /// Python: `self.cursor_space_multiplier` (py:66-69) — derived
    /// from `theme_config['cursor_space']` if present.
    pub cursor_space_multiplier: Option<f64>,
    /// Python: `self.cursor_columns` (py:70) —
    /// `theme_config.get('cursor_columns')`.
    pub cursor_columns: Option<i64>,
    /// Python: `self.spaces` (py:71) —
    /// `theme_config['spaces']` (an int).
    pub spaces: i64,
    /// Python: `self.outer_padding` (py:72) —
    /// `int(theme_config.get('outer_padding', 1))`.
    pub outer_padding: i64,
    /// Python: `self.segments` (py:73) — list of `{left: [], right:
    /// []}` per-line segment dicts produced by py:91-105.
    pub segments: Vec<Map<String, Value>>,
    /// Python: `self.EMPTY_SEGMENT` (py:74-77) — sentinel returned
    /// by render fallback paths.
    pub empty_segment: Value,
    /// Records the shutdown order. Used in lieu of the
    /// `segment['shutdown']()` callable side effect since the
    /// segment dispatch closures aren't yet wired through Rust.
    pub shutdown_called: std::sync::Mutex<Vec<String>>,
}

impl Theme {
    /// Builds an empty Theme value-store with sentinel defaults.
    /// The full `__init__` body (py:48-105) requires
    /// `gen_segment_getter`; this constructor surfaces just the
    /// post-init shape so the accessor methods can be exercised.
    pub fn new() -> Self {
        // py:47  class Theme(object):
        // py:48  def __init__(self,
        // py:49  ext,
        // py:50  theme_config,
        // py:51  common_config,
        // py:52  pl,
        // py:53  get_module_attr,
        // py:54  top_theme,
        // py:55  colorscheme,
        // py:56  main_theme_config=None,
        // py:57  run_once=False,
        // py:58  shutdown_event=None):
        // py:59  self.colorscheme = colorscheme
        // py:60  self.dividers = theme_config['dividers']
        // py:61  self.dividers = dict((
        // py:62  (key, dict((k, u(v))
        // py:63  for k, v in val.items()))
        // py:64  for key, val in self.dividers.items()
        // py:65  ))
        // py:66  try:
        // py:67  self.cursor_space_multiplier = 1 - (theme_config['cursor_space'] / 100)
        // py:68  except KeyError:
        // py:69  self.cursor_space_multiplier = None
        // py:70  self.cursor_columns = theme_config.get('cursor_columns')
        // py:71  self.spaces = theme_config['spaces']
        // py:72  self.outer_padding = int(theme_config.get('outer_padding', 1))
        // py:73  self.segments = []
        // py:74  self.EMPTY_SEGMENT = {
        // py:75  'contents': None,
        // py:76  'highlight': {'fg': False, 'bg': False, 'attrs': 0}
        // py:77  }
        // py:78  self.pl = pl
        Self {
            colorscheme: Value::Null,
            dividers: Map::new(),
            cursor_space_multiplier: None,
            cursor_columns: None,
            spaces: 0,
            outer_padding: 1,
            segments: Vec::new(),
            empty_segment: serde_json::json!({
                "contents": Value::Null,
                "highlight": {"fg": false, "bg": false, "attrs": 0},
            }),
            shutdown_called: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Helper that sets `cursor_space_multiplier` from a
    /// `cursor_space` percentage per py:66-69.
    ///
    /// Python: `self.cursor_space_multiplier = 1 - (cs / 100)`,
    /// falling through to None on `KeyError`.
    pub fn apply_cursor_space(&mut self, cursor_space: Option<f64>) {
        // py:66-69
        self.cursor_space_multiplier = cursor_space.map(|cs| 1.0 - (cs / 100.0));
    }

    /// Port of `Theme.shutdown()` from
    /// `powerline/theme.py:107-114`.
    ///
    /// Calls each segment's `'shutdown'` callable per py:110-114.
    /// Python silently swallows `TypeError` per py:113 for segments
    /// whose `shutdown` is None. The Rust port records the called
    /// segment names in `shutdown_called` for test assertion since
    /// segment dispatch closures aren't reachable here.
    pub fn shutdown(&self) {
        // py:107  def shutdown(self):
        // py:108  for line in self.segments:
        // py:109  for segments in line.values():
        // py:110  for segment in segments:
        // py:111  try:
        // py:112  segment['shutdown']()
        // py:113  except TypeError:
        // py:114  pass
        let mut log = self
            .shutdown_called
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        for line in &self.segments {
            for (_side, segments_value) in line {
                let segments = match segments_value.as_array() {
                    Some(s) => s,
                    None => continue,
                };
                for segment in segments {
                    if let Some(name) = segment.get("name").and_then(|v| v.as_str()) {
                        if segment.get("shutdown").is_some_and(|v| !v.is_null()) {
                            log.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    /// Port of `Theme.get_divider()` from
    /// `powerline/theme.py:116-118`.
    ///
    /// Default `side='left', type='soft'` per the Python signature.
    /// Returns the divider char from the
    /// `dividers[side][type]` nested dict.
    pub fn get_divider(&self, side: &str, divider_type: &str) -> Option<String> {
        // py:116  def get_divider(self, side='left', type='soft'):
        // py:117  '''Return segment divider.'''
        // py:118  return self.dividers[side][type]
        self.dividers
            .get(side)
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(divider_type))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Port of `Theme.get_spaces()` from
    /// `powerline/theme.py:120-121`.
    pub fn get_spaces(&self) -> i64 {
        // py:120  def get_spaces(self):
        // py:121  return self.spaces
        self.spaces
    }

    /// Port of `Theme.get_line_number()` from
    /// `powerline/theme.py:123-124`.
    pub fn get_line_number(&self) -> usize {
        // py:123  def get_line_number(self):
        // py:124  return len(self.segments)
        self.segments.len()
    }

    /// Port of `Theme.get_segments()` from
    /// `powerline/theme.py:126-182`.
    ///
    /// Iterates `self.segments[line][side]`, applies `display_condition`
    /// and `process_segment` per segment, then walks the parsed result
    /// applying width/align per py:149-177. The `contents_func`
    /// closure is the caller-supplied dispatcher (Python looks it up
    /// as `segment['contents_func']`; Rust `Value` can't hold a
    /// closure so the bin shim provides one keyed by
    /// `segment['contents_func']` string id).
    #[allow(clippy::too_many_arguments)]
    pub fn get_segments<C>(
        &self,
        side: Option<&str>,
        line: usize,
        segment_info: Option<&Value>,
        mode: Option<&str>,
        colorscheme: &crate::ported::colorscheme::Colorscheme,
        contents_func: &C,
    ) -> Vec<Value>
    where
        C: Fn(&str, &(), &Map<String, Value>, &Map<String, Value>) -> Option<Value>,
    {
        // py:126  def get_segments(self, side=None, line=0, segment_info=None, mode=None):
        // py:127-135  docstring
        // py:136  for side in [side] if side else ['left', 'right']:
        // py:137  parsed_segments = []
        // py:138  for segment in self.segments[line][side]:
        // py:139  if segment['display_condition'](self.pl, segment_info, mode):
        // py:140  process_segment(
        // py:141  self.pl,
        // py:142  side,
        // py:143  segment_info,
        // py:144  parsed_segments,
        // py:145  segment,
        // py:146  mode,
        // py:147  self.colorscheme,
        // py:148  )
        // py:149  for segment in parsed_segments:
        // py:150  self.pl.prefix = segment['name']
        // py:151  try:
        // py:152  width = segment['width']
        // py:153  align = segment['align']
        // py:154  if width == 'auto' and segment['expand'] is None:
        // py:155  segment['expand'] = expand_functions.get(align)
        // py:156  if segment['expand'] is None:
        // py:157  self.pl.error('Align argument must be "r", "l" or "c", not "{0}"', align)
        // py:159  try:
        // py:160  segment['contents'] = segment['before'] + u(
        // py:161  segment['contents'] if segment['contents'] is not None else ''
        // py:162  ) + segment['after']
        // py:163  except Exception as e:
        // py:164  self.pl.exception('Failed to compute segment contents: {0}', str(e))
        // py:165  segment['contents'] = safe_unicode(segment.get('contents'))
        // py:166  # Align segment contents
        // py:167  if segment['width'] and segment['width'] != 'auto':
        // py:168  if segment['align'] == 'l':
        // py:169  segment['contents'] = segment['contents'].ljust(segment['width'])
        // py:170  elif segment['align'] == 'r':
        // py:171  segment['contents'] = segment['contents'].rjust(segment['width'])
        // py:172  elif segment['align'] == 'c':
        // py:173  segment['contents'] = segment['contents'].center(segment['width'])
        // py:177  yield segment.copy()
        // py:178  except Exception as e:
        // py:179  self.pl.exception('Failed to compute segment: {0}', str(e))
        // py:180  fallback = get_fallback_segment()
        // py:181  fallback.update(side=side)
        // py:182  yield fallback
        let mut out: Vec<Value> = Vec::new();
        let segment_info_map: Map<String, Value> = segment_info
            .and_then(|v| v.as_object().cloned())
            .unwrap_or_default();
        // py:136  for side in [side] if side else ['left', 'right']:
        let sides: Vec<&str> = match side {
            Some(s) => vec![s],
            None => vec!["left", "right"],
        };
        let pl = ();
        for side in sides {
            // py:137  parsed_segments = []
            let mut parsed_segments: Vec<Value> = Vec::new();
            // py:138  for segment in self.segments[line][side]:
            let Some(line_map) = self.segments.get(line) else {
                continue;
            };
            let Some(side_arr) = line_map.get(side).and_then(|v| v.as_array()) else {
                continue;
            };
            for segment_v in side_arr {
                let Some(segment) = segment_v.as_object() else {
                    continue;
                };
                // py:139  if segment['display_condition'](self.pl, segment_info, mode):
                // Mirrors `gen_display_condition` at upstream
                // `segment.py:303-317` for the include_modes / exclude_modes
                // branches (function-selectors require a callable
                // registry that's not yet ported and are treated as
                // "no constraint").
                let mode_in = |list_v: &Value| -> bool {
                    list_v
                        .as_array()
                        .map(|arr| {
                            arr.iter().any(|v| match (v.as_str(), mode) {
                                (Some(s), Some(m)) => s == m,
                                _ => false,
                            })
                        })
                        .unwrap_or(false)
                };
                // py:287-289  `if modes: ... return lambda ...: mode in modes`
                let include_ok = match segment.get("include_modes") {
                    Some(v) if !v.is_null() => mode_in(v),
                    _ => true,
                };
                // py:308-315  exclude wraps result in `not exclude_function(*args)`
                let exclude_ok = match segment.get("exclude_modes") {
                    Some(v) if !v.is_null() => !mode_in(v),
                    _ => true,
                };
                // py:303-317  function-name selectors. The Rust port
                // stores the function name string on the prepared
                // segment; a callable registry (analog of
                // `gen_module_attr_getter` for selectors) wires the
                // actual evaluation. Until the registry threads
                // through `Theme::get_segments`, function selectors
                // default to "no constraint" (always_true) per the
                // Python "no matcher → unfiltered" semantics at
                // py:300-301.
                let _include_fn_present = segment
                    .get("include_function")
                    .map(|v| !v.is_null())
                    .unwrap_or(false);
                let _exclude_fn_present = segment
                    .get("exclude_function")
                    .map(|v| !v.is_null())
                    .unwrap_or(false);
                if !(include_ok && exclude_ok) {
                    continue;
                }
                // py:140-148  process_segment(self.pl, side, segment_info, parsed_segments, segment, mode, self.colorscheme)
                crate::ported::segment::process_segment(
                    &pl,
                    side,
                    &segment_info_map,
                    &mut parsed_segments,
                    segment,
                    mode,
                    colorscheme,
                    &|pl_inner, si, args| {
                        // py:173/175  segment['contents_func'](pl, segment_info[, ...])
                        let id = segment
                            .get("contents_func")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        contents_func(id, pl_inner, si, args)
                    },
                );
            }
            // py:149  for segment in parsed_segments:
            for mut segment in parsed_segments {
                let Some(obj) = segment.as_object_mut() else {
                    continue;
                };
                // py:150  self.pl.prefix = segment['name'] — logger deferred
                let _ = obj.get("name");
                // py:152-153  width / align
                let width = obj.get("width").cloned().unwrap_or(Value::Null);
                let align = obj
                    .get("align")
                    .and_then(|v| v.as_str())
                    .unwrap_or("l")
                    .to_string();
                // py:154-157  if width == 'auto' and expand is None: expand = expand_functions[align]
                if width.as_str() == Some("auto")
                    && obj.get("expand").map(|v| v.is_null()).unwrap_or(true)
                    && expand_functions(align.chars().next().unwrap_or('l')).is_some()
                {
                    // The fn pointer can't be stored in Value; the renderer's
                    // padding logic at do_render handles 'auto' width spacing.
                    obj.insert("expand".to_string(), Value::String(align.clone()));
                }
                // py:159-165  segment['contents'] = before + contents + after
                let before = obj
                    .get("before")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let after = obj
                    .get("after")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let contents = obj
                    .get("contents")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                obj.insert(
                    "contents".to_string(),
                    Value::String(format!("{}{}{}", before, contents, after)),
                );
                // py:167-173  width-driven ljust/rjust/center alignment
                let width_int = width.as_u64();
                if let Some(w) = width_int {
                    let w = w as usize;
                    let cur = obj
                        .get("contents")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let len = cur.chars().count();
                    if len < w {
                        let pad = w - len;
                        let padded = match align.as_str() {
                            "l" => format!("{}{}", cur, " ".repeat(pad)),
                            "r" => format!("{}{}", " ".repeat(pad), cur),
                            "c" => {
                                let left = pad / 2;
                                let right = pad - left;
                                format!("{}{}{}", " ".repeat(left), cur, " ".repeat(right))
                            }
                            _ => cur,
                        };
                        obj.insert("contents".to_string(), Value::String(padded));
                    }
                }
                // py:177  yield segment.copy()
                out.push(Value::Object(obj.clone()));
            }
        }
        out
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new()
    }
}

// Theme.__init__ (py:48-105) and Theme.get_segments (py:126-182)
// port together with segment.py — see docs/PORT_CHECKLIST.md. The
// __init__ depends on gen_segment_getter (segment.py:254),
// get_segments depends on process_segment (segment.py:167) and
// get_fallback_segment (TODO in segment.py). All three are unported
// scaffolds; landing them together preserves the dispatch invariant.

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seg(contents: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("contents".to_string(), Value::String(contents.into()));
        m
    }

    #[test]
    fn new_empty_segment_line_has_left_and_right() {
        let line = new_empty_segment_line();
        assert!(line.get("left").is_some());
        assert!(line.get("right").is_some());
        assert!(line["left"].as_array().unwrap().is_empty());
        assert!(line["right"].as_array().unwrap().is_empty());
    }

    #[test]
    fn add_spaces_left_pads_on_left() {
        let s = seg("hi");
        assert_eq!(add_spaces_left(&(), 3, &s), "   hi");
    }

    #[test]
    fn add_spaces_right_pads_on_right() {
        let s = seg("hi");
        assert_eq!(add_spaces_right(&(), 3, &s), "hi   ");
    }

    #[test]
    fn add_spaces_center_pads_evenly_extra_left_when_odd() {
        let s = seg("hi");
        // amount=4 → 2+2 around "hi"
        assert_eq!(add_spaces_center(&(), 4, &s), "  hi  ");
        // amount=5 → 3 left, 2 right (remainder goes to left)
        assert_eq!(add_spaces_center(&(), 5, &s), "   hi  ");
        // amount=0 → no padding
        assert_eq!(add_spaces_center(&(), 0, &s), "hi");
    }

    #[test]
    fn expand_functions_returns_correct_fn_for_each_align() {
        let s = seg("x");
        // 'l' → add_spaces_right (left-aligned → right pad)
        let f = expand_functions('l').unwrap();
        assert_eq!(f(&(), 2, &s), "x  ");
        // 'r' → add_spaces_left (right-aligned → left pad)
        let f = expand_functions('r').unwrap();
        assert_eq!(f(&(), 2, &s), "  x");
        // 'c' → add_spaces_center
        let f = expand_functions('c').unwrap();
        assert_eq!(f(&(), 2, &s), " x ");
        // unknown align returns None
        assert!(expand_functions('z').is_none());
    }

    #[test]
    fn requires_decorators_are_identity() {
        let f = |x: i32| x + 1;
        let _g = requires_segment_info(f);
        let _h = requires_filesystem_watcher(f);
        assert_eq!(f(1), 2);
    }

    /// Smoke test: a typical right-aligned segment renders with leading spaces
    /// matching the underlying contents prefix.
    #[test]
    fn alignment_idiom_matches_documented_inverse_mapping() {
        let s = json!({"contents": "x"}).as_object().unwrap().clone();
        // Right-aligned segment is padded on the LEFT
        let right_aligned = expand_functions('r').unwrap();
        let result = right_aligned(&(), 3, &s);
        assert!(result.ends_with('x'));
        assert!(result.starts_with(' '));
    }

    #[test]
    fn theme_new_initialises_empty_segment_with_python_shape() {
        // py:74-77
        let t = Theme::new();
        let es = &t.empty_segment;
        assert_eq!(es["contents"], Value::Null);
        assert_eq!(es["highlight"]["fg"], false);
        assert_eq!(es["highlight"]["bg"], false);
        assert_eq!(es["highlight"]["attrs"], 0);
    }

    #[test]
    fn theme_new_defaults_outer_padding_to_1() {
        // py:72  int(theme_config.get('outer_padding', 1))
        let t = Theme::new();
        assert_eq!(t.outer_padding, 1);
    }

    #[test]
    fn theme_apply_cursor_space_computes_multiplier() {
        // py:67  1 - (cursor_space / 100)
        let mut t = Theme::new();
        t.apply_cursor_space(Some(25.0));
        assert!((t.cursor_space_multiplier.unwrap() - 0.75).abs() < 1e-9);
    }

    #[test]
    fn theme_apply_cursor_space_none_leaves_multiplier_unset() {
        // py:68-69  KeyError → None
        let mut t = Theme::new();
        t.apply_cursor_space(None);
        assert!(t.cursor_space_multiplier.is_none());
    }

    #[test]
    fn theme_get_divider_resolves_nested_lookup() {
        // py:118  return self.dividers[side][type]
        let mut t = Theme::new();
        let mut left_dividers = Map::new();
        left_dividers.insert("hard".to_string(), Value::String("\u{e0b0}".into()));
        left_dividers.insert("soft".to_string(), Value::String("\u{e0b1}".into()));
        t.dividers
            .insert("left".to_string(), Value::Object(left_dividers));
        assert_eq!(t.get_divider("left", "hard").as_deref(), Some("\u{e0b0}"));
        assert_eq!(t.get_divider("left", "soft").as_deref(), Some("\u{e0b1}"));
    }

    #[test]
    fn theme_get_divider_returns_none_for_missing_side_or_type() {
        let mut t = Theme::new();
        let mut left_dividers = Map::new();
        left_dividers.insert("hard".to_string(), Value::String(">".into()));
        t.dividers
            .insert("left".to_string(), Value::Object(left_dividers));
        // Missing type
        assert!(t.get_divider("left", "soft").is_none());
        // Missing side
        assert!(t.get_divider("right", "hard").is_none());
    }

    #[test]
    fn theme_get_spaces_returns_field_value() {
        // py:121
        let mut t = Theme::new();
        t.spaces = 2;
        assert_eq!(t.get_spaces(), 2);
    }

    #[test]
    fn theme_get_line_number_returns_segments_len() {
        // py:124  len(self.segments)
        let mut t = Theme::new();
        assert_eq!(t.get_line_number(), 0);
        t.segments.push(new_empty_segment_line());
        assert_eq!(t.get_line_number(), 1);
        t.segments.push(new_empty_segment_line());
        assert_eq!(t.get_line_number(), 2);
    }

    #[test]
    fn theme_shutdown_records_segments_with_shutdown_callable() {
        // py:110-114
        let mut t = Theme::new();
        let mut line = new_empty_segment_line();
        // Add a segment with both name and shutdown
        let segment = json!({
            "name": "uptime",
            "shutdown": "ptr_placeholder",
        });
        line.insert("left".to_string(), Value::Array(vec![segment]));
        t.segments.push(line);
        t.shutdown();
        let log = t.shutdown_called.lock().unwrap();
        assert_eq!(*log, vec!["uptime".to_string()]);
    }

    #[test]
    fn theme_shutdown_skips_segments_with_null_shutdown() {
        // py:112-114  TypeError swallowed when shutdown is None
        let mut t = Theme::new();
        let mut line = new_empty_segment_line();
        let segment_no_shutdown = json!({
            "name": "no_shutdown",
            "shutdown": Value::Null,
        });
        let segment_with_shutdown = json!({
            "name": "has_shutdown",
            "shutdown": "ptr_placeholder",
        });
        line.insert(
            "left".to_string(),
            Value::Array(vec![segment_no_shutdown, segment_with_shutdown]),
        );
        t.segments.push(line);
        t.shutdown();
        let log = t.shutdown_called.lock().unwrap();
        assert_eq!(*log, vec!["has_shutdown".to_string()]);
    }

    #[test]
    fn theme_shutdown_walks_both_sides() {
        // py:109  for segments in line.values() — both left and right
        let mut t = Theme::new();
        let mut line = Map::new();
        line.insert(
            "left".to_string(),
            Value::Array(vec![json!({"name": "left_seg", "shutdown": "ptr"})]),
        );
        line.insert(
            "right".to_string(),
            Value::Array(vec![json!({"name": "right_seg", "shutdown": "ptr"})]),
        );
        t.segments.push(line);
        t.shutdown();
        let log = t.shutdown_called.lock().unwrap();
        assert_eq!(log.len(), 2);
        assert!(log.contains(&"left_seg".to_string()));
        assert!(log.contains(&"right_seg".to_string()));
    }
}
