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
    // py:21-24
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
    // py:28
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
    // py:32
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
    // py:36  amount, remainder = divmod(amount, 2)
    let (half, remainder) = (amount / 2, amount % 2);
    // py:37
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
    match align {
        // py:40-44
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
        Self {
            colorscheme: Value::Null,
            dividers: Map::new(),
            cursor_space_multiplier: None,
            cursor_columns: None,
            spaces: 0,
            outer_padding: 1,
            segments: Vec::new(),
            // py:74-77  EMPTY_SEGMENT shape
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
        // py:108-114
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
                    // py:111-114  try segment['shutdown'](); except TypeError: pass
                    if let Some(name) = segment.get("name").and_then(|v| v.as_str()) {
                        // Only record when 'shutdown' is non-None per py:112
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
        // py:121  return self.spaces
        self.spaces
    }

    /// Port of `Theme.get_line_number()` from
    /// `powerline/theme.py:123-124`.
    pub fn get_line_number(&self) -> usize {
        // py:124  return len(self.segments)
        self.segments.len()
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
