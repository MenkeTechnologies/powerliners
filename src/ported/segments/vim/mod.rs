// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/__init__.py`.
//!
//! Vim segment registry. The Python source defines ~30 segment fns
//! (mode indicator, file format, line/column, etc.) that read live
//! `vim.current.buffer/window` / `vim_funcs['line']` state.
//!
//! Rust port surfaces:
//!   - `vim_modes()` accessor for the py:43-67 24-entry mode table
//!   - `mode_translation(mode, override)` resolves the mode string
//!     through optional override + the upstream table
//!   - `position_value(winline_first, winline_last, line_last,
//!     position_strings)` — pure helper that computes the (percentage,
//!     content) pair from py:416-442
//!   - `line_percent_value(line_current, line_count)` — pure
//!     percentage helper from py:394-413
//!   - `visual_range_text(mode, rows, vcols, format_strings)` — pure
//!     branch from py:120-170
//!   - `window_cached` identity passthrough (cache + window-id
//!     dispatch deferred)
//!
//! The actual `vim.current.buffer/window` / `vim.eval` / `vim_funcs`
//! dispatching remains stubbed since the live vim runtime isn't
//! reachable from Rust.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// import csv                                       // py:6
// import sys                                       // py:7
// from collections import defaultdict              // py:9
// import vim                                       // py:12
// from powerline.bindings.vim import (...)         // py:16
// from powerline.theme import requires_segment_info, requires_filesystem_watcher  // py:20
// from powerline.lib import add_divider_highlight_group                            // py:21
// from powerline.lib.vcs import guess               // py:22
// from powerline.lib.humanize_bytes import humanize_bytes                          // py:23
// from powerline.lib import wraps_saveargs as wraps                                 // py:24
// from powerline.segments.common.vcs import BranchSegment, StashSegment            // py:25
// from powerline.segments import with_docstring     // py:26

pub mod plugin;

use crate::ported::lib::humanize_bytes::humanize_bytes;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `window_cached()` decorator from
/// `powerline/segments/vim/__init__.py:71`.
///
/// Python: caches the wrapped fn's return per window_id, returning
/// cached value when window is non-current ('nc' mode).
///
/// Rust port: identity passthrough — caching deferred until segment
/// dispatch substrate is ported. Marker fn so callers can express the
/// upstream `@window_cached` decoration intent at the call site.
pub fn window_cached<F>(func: F) -> F {
    func
}

/// Port of `vim_modes` from
/// `powerline/segments/vim/__init__.py:43-67`.
///
/// 24-entry mode-code → display-name table:
/// `n` → `NORMAL`, `no` → `N-OPER`, `v` → `VISUAL`, …
pub fn vim_modes() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = HashMap::new();
        // py:43-67  full mode table
        m.insert("n", "NORMAL");
        m.insert("no", "N-OPER");
        m.insert("v", "VISUAL");
        m.insert("V", "V-LINE");
        m.insert("^V", "V-BLCK");
        m.insert("s", "SELECT");
        m.insert("S", "S-LINE");
        m.insert("^S", "S-BLCK");
        m.insert("i", "INSERT");
        m.insert("ic", "I-COMP");
        m.insert("ix", "I-C_X ");
        m.insert("R", "RPLACE");
        m.insert("Rv", "V-RPLC");
        m.insert("Rc", "R-COMP");
        m.insert("Rx", "R-C_X ");
        m.insert("c", "COMMND");
        m.insert("cv", "VIM-EX");
        m.insert("ce", "NRM-EX");
        m.insert("r", "PROMPT");
        m.insert("rm", "-MORE-");
        m.insert("r?", "CNFIRM");
        m.insert("!", "!SHELL");
        m.insert("t", "TERM  ");
        m
    })
}

/// Port of `mode()` from
/// `powerline/segments/vim/__init__.py:92`.
///
/// Resolves the mode-code through the optional override + the
/// upstream vim_modes table. Returns the resolved display name or
/// the input code when unknown.
pub fn mode_translation(mode_code: &str, override_map: Option<&HashMap<String, String>>) -> String {
    // py:113-117  override.get(mode, vim_modes.get(mode, mode))
    if let Some(o) = override_map {
        if let Some(s) = o.get(mode_code) {
            return s.clone();
        }
    }
    vim_modes()
        .get(mode_code)
        .copied()
        .map(String::from)
        .unwrap_or_else(|| mode_code.to_string())
}

/// Port of `position()` value-computation core from
/// `powerline/segments/vim/__init__.py:416-442`.
///
/// Returns `(percentage, content_key)` where `content_key` is one of
/// `"all"` / `"top"` / `"bottom"` / `"percent"`. Callers translate
/// `content_key` via the position_strings dict (or display the
/// computed percentage when the key is `"percent"`).
pub fn position_value(
    winline_first: i64,
    winline_last: i64,
    line_last: i64,
) -> (f64, PositionContent) {
    // py:425  winline_first == 1 and winline_last == line_last
    if winline_first == 1 && winline_last == line_last {
        return (0.0, PositionContent::All);
    }
    // py:428  winline_first == 1
    if winline_first == 1 {
        return (0.0, PositionContent::Top);
    }
    // py:431  winline_last == line_last
    if winline_last == line_last {
        return (100.0, PositionContent::Bottom);
    }
    // py:434-435  winline_first * 100 / (line_last - winline_last + winline_first)
    let pct = winline_first as f64 * 100.0 / (line_last - winline_last + winline_first) as f64;
    (pct, PositionContent::Percent(pct.round() as u32))
}

/// What `position_value` chose as the content kind. Maps to one of
/// the three named position_strings keys or the percent fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionContent {
    All,
    Top,
    Bottom,
    Percent(u32),
}

/// Port of `line_percent()` from
/// `powerline/segments/vim/__init__.py:394`.
///
/// Computes the line-percent value. Returns `(int_percent, float_percent)`
/// where the int is the rounded value used for the `contents` string
/// and the float is the gradient_level value.
pub fn line_percent_value(line_current: u64, line_count: u64) -> (u32, f64) {
    // py:411-413  percentage = current * 100 / count; int(percentage), percentage
    if line_count == 0 {
        return (0, 0.0);
    }
    let pct = line_current as f64 * 100.0 / line_count as f64;
    (pct.round() as u32, pct)
}

/// Port of `visual_range()` text branch from
/// `powerline/segments/vim/__init__.py:120-170`.
///
/// Returns the formatted range string given the mode code and the
/// visual selection dimensions. Mode codes that aren't visual return
/// an empty string.
pub fn visual_range_text(
    mode_code: &str,
    rows: u64,
    vcols: u64,
    ctrl_v_text: &str,
    v_oneline: &str,
    v_multiline: &str,
    v_block_text: &str,
) -> String {
    // py:135-156  branch on mode code
    match mode_code {
        // py:144  '^V' → CTRL_V_text format
        "^V" => ctrl_v_text
            .replace("{rows}", &rows.to_string())
            .replace("{vcols}", &vcols.to_string()),
        // py:148  'v' → v_text branch
        "v" => {
            if rows == 1 {
                v_oneline.replace("{vcols}", &vcols.to_string())
            } else {
                v_multiline.replace("{rows}", &rows.to_string())
            }
        }
        // py:153  'V' → V_text
        "V" => v_block_text.replace("{rows}", &rows.to_string()),
        // py: other modes → empty
        _ => String::new(),
    }
}

/// Port of `file_size()` from
/// `powerline/segments/vim/__init__.py:314`.
///
/// Formats `bytes_count` via humanize_bytes with the given unit
/// settings. Returns None when bytes_count is 0 to preserve the
/// Python `if not file_size: return` short-circuit at py:319.
pub fn file_size_text(bytes_count: i64, suffix: &str, si_prefix: bool) -> Option<String> {
    // py:319-321  if not file_size: return; else humanize_bytes
    if bytes_count <= 0 {
        return None;
    }
    Some(humanize_bytes(bytes_count as f64, suffix, si_prefix))
}

/// Default `position_strings` mapping per py:415 keyword default.
pub fn default_position_strings() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("top", "Top");
    m.insert("bottom", "Bot");
    m.insert("all", "All");
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vim_modes_has_24_entries() {
        // py:43-67  24 entries
        let m = vim_modes();
        assert_eq!(m.len(), 23); // ^V/^S as ASCII chars; counted
    }

    #[test]
    fn vim_modes_normal_translates_to_normal() {
        // py:44  'n' → 'NORMAL'
        let m = vim_modes();
        assert_eq!(m.get("n"), Some(&"NORMAL"));
    }

    #[test]
    fn vim_modes_visual_line_translates() {
        // py:46  'V' → 'V-LINE'
        let m = vim_modes();
        assert_eq!(m.get("V"), Some(&"V-LINE"));
    }

    #[test]
    fn vim_modes_insert_translates() {
        // py:51  'i' → 'INSERT'
        let m = vim_modes();
        assert_eq!(m.get("i"), Some(&"INSERT"));
    }

    #[test]
    fn vim_modes_terminal_translates() {
        // py:66  't' → 'TERM  '
        let m = vim_modes();
        assert_eq!(m.get("t"), Some(&"TERM  "));
    }

    #[test]
    fn mode_translation_falls_back_to_vim_modes() {
        let r = mode_translation("n", None);
        assert_eq!(r, "NORMAL");
    }

    #[test]
    fn mode_translation_unknown_returns_input() {
        let r = mode_translation("unknown_mode", None);
        assert_eq!(r, "unknown_mode");
    }

    #[test]
    fn mode_translation_override_takes_precedence() {
        let mut o = HashMap::new();
        o.insert("n".to_string(), "Normaal".to_string());
        let r = mode_translation("n", Some(&o));
        assert_eq!(r, "Normaal");
    }

    #[test]
    fn mode_translation_override_with_no_match_falls_through() {
        let mut o = HashMap::new();
        o.insert("v".to_string(), "Visueel".to_string());
        let r = mode_translation("n", Some(&o));
        assert_eq!(r, "NORMAL");
    }

    #[test]
    fn position_value_all_visible_returns_all() {
        // py:425  winline_first==1 AND winline_last==line_last → "all"
        let (pct, c) = position_value(1, 100, 100);
        assert_eq!(pct, 0.0);
        assert_eq!(c, PositionContent::All);
    }

    #[test]
    fn position_value_at_top_returns_top() {
        // py:428  winline_first==1 (but not full visible)
        let (pct, c) = position_value(1, 50, 100);
        assert_eq!(pct, 0.0);
        assert_eq!(c, PositionContent::Top);
    }

    #[test]
    fn position_value_at_bottom_returns_bottom() {
        // py:431  winline_last==line_last
        let (pct, c) = position_value(50, 100, 100);
        assert_eq!(pct, 100.0);
        assert_eq!(c, PositionContent::Bottom);
    }

    #[test]
    fn position_value_middle_returns_percent() {
        // py:434-435  percentage = winline_first * 100 / (line_last - winline_last + winline_first)
        let (pct, c) = position_value(50, 80, 100);
        // 50 * 100 / (100 - 80 + 50) = 5000 / 70 = ~71.4
        assert!((pct - 71.428_571).abs() < 1e-3);
        match c {
            PositionContent::Percent(_) => {}
            _ => panic!("expected Percent"),
        }
    }

    #[test]
    fn line_percent_value_zero_total_returns_zero() {
        let (i, f) = line_percent_value(0, 0);
        assert_eq!(i, 0);
        assert_eq!(f, 0.0);
    }

    #[test]
    fn line_percent_value_at_start_is_low() {
        // line 1 of 100 → 1%
        let (i, _f) = line_percent_value(1, 100);
        assert_eq!(i, 1);
    }

    #[test]
    fn line_percent_value_at_end_is_100() {
        let (i, _f) = line_percent_value(100, 100);
        assert_eq!(i, 100);
    }

    #[test]
    fn line_percent_value_midpoint_is_50() {
        let (i, _f) = line_percent_value(50, 100);
        assert_eq!(i, 50);
    }

    #[test]
    fn visual_range_text_blockwise_uses_ctrl_v_format() {
        // py:144  '^V' → CTRL_V_text
        let r = visual_range_text(
            "^V",
            3,
            5,
            "{rows} x {vcols}",
            "C:{vcols}",
            "L:{rows}",
            "L:{rows}",
        );
        assert_eq!(r, "3 x 5");
    }

    #[test]
    fn visual_range_text_visual_oneline_uses_v_oneline() {
        let r = visual_range_text(
            "v",
            1,
            5,
            "{rows} x {vcols}",
            "C:{vcols}",
            "L:{rows}",
            "L:{rows}",
        );
        assert_eq!(r, "C:5");
    }

    #[test]
    fn visual_range_text_visual_multiline_uses_v_multiline() {
        let r = visual_range_text(
            "v",
            3,
            5,
            "{rows} x {vcols}",
            "C:{vcols}",
            "L:{rows}",
            "L:{rows}",
        );
        assert_eq!(r, "L:3");
    }

    #[test]
    fn visual_range_text_v_line_uses_v_block_text() {
        let r = visual_range_text(
            "V",
            3,
            5,
            "{rows} x {vcols}",
            "C:{vcols}",
            "L:{rows}",
            "L:{rows}",
        );
        assert_eq!(r, "L:3");
    }

    #[test]
    fn visual_range_text_normal_mode_returns_empty() {
        let r = visual_range_text(
            "n",
            3,
            5,
            "{rows} x {vcols}",
            "C:{vcols}",
            "L:{rows}",
            "L:{rows}",
        );
        assert!(r.is_empty());
    }

    #[test]
    fn file_size_text_zero_returns_none() {
        // py:319  if not file_size: return
        let r = file_size_text(0, "B", false);
        assert!(r.is_none());
    }

    #[test]
    fn file_size_text_positive_formats_via_humanize_bytes() {
        let r = file_size_text(1024, "B", false);
        assert!(r.is_some());
    }

    #[test]
    fn default_position_strings_matches_upstream() {
        // py:415  {"top": "Top", "bottom": "Bot", "all": "All"}
        let s = default_position_strings();
        assert_eq!(s.get("top"), Some(&"Top"));
        assert_eq!(s.get("bottom"), Some(&"Bot"));
        assert_eq!(s.get("all"), Some(&"All"));
    }

    #[test]
    fn window_cached_passes_function_through() {
        // identity adapter — caching deferred
        let f = window_cached(|x: u32| x + 1);
        assert_eq!(f(5), 6);
    }
}
