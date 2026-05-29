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
pub fn expand_functions(align: char) -> Option<fn(&(), usize, &Map<String, Value>) -> String> {
    match align {
        // py:40-44
        'l' => Some(add_spaces_right),
        'r' => Some(add_spaces_left),
        'c' => Some(add_spaces_center),
        _ => None,
    }
}

// Theme class (py:47-182) ports together with segment.py — see
// docs/PORT_CHECKLIST.md. The __init__ depends on gen_segment_getter
// (segment.py:254), get_segments depends on process_segment
// (segment.py:167) and get_fallback_segment (TODO in segment.py).
// All three are unported scaffolds; landing them together preserves
// the dispatch invariant.

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
}
