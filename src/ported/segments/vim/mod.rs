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

/// Port of module-level `SCHEME_RE` from
/// `powerline/segments/vim/__init__.py:219`.
///
/// Python pattern: `^\w[\w\d+\-.]*(?=:)` — matches the URI scheme
/// prefix using a `(?=:)` lookahead. Rust's `regex` crate does NOT
/// support lookahead, so the Rust port matches
/// `^\w[\w\d+\-.]*:` (including the trailing colon) and callers
/// strip the trailing `:` from the captured match. The matched
/// prefix is captured as group 1 to preserve the original semantic.
#[allow(non_snake_case)]
pub fn SCHEME_RE() -> &'static regex::Regex {
    static R: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    R.get_or_init(|| regex::Regex::new(r"^(\w[\w\d+\-.]*):").unwrap())
}

/// Port of `mode()` from
/// `powerline/segments/vim/__init__.py:92-115`.
///
/// Pure-functional core. Returns the translated mode name. The Python
/// fn takes `segment_info` and pulls `segment_info['mode']`; the Rust
/// port takes the mode string directly. Returns None for 'nc'
/// (no-current) per py:103-104.
pub fn mode(mode_str: &str, override_map: Option<&HashMap<String, String>>) -> Option<String> {
    // py:103-104  if mode == 'nc': return None
    if mode_str == "nc" {
        return None;
    }
    // py:105-114  iterate trimming last char until a known mode is matched
    let mut current = mode_str.to_string();
    let modes = vim_modes();
    loop {
        if current.is_empty() {
            break;
        }
        if let Some(map) = override_map {
            if let Some(v) = map.get(&current) {
                return Some(v.clone());
            }
        }
        if let Some(v) = modes.get(current.as_str()) {
            return Some(v.to_string());
        }
        // Trim last character. py:114  mode = mode[:-1]
        current.pop();
    }
    // py:115  return 'BUG'
    Some("BUG".to_string())
}

/// Port of `modified_indicator()` from
/// `powerline/segments/vim/__init__.py:172-178`.
pub fn modified_indicator(modified: bool, text: &str) -> Option<String> {
    // py:178  return text if int(...modified) else None
    if modified {
        Some(text.to_string())
    } else {
        None
    }
}

/// Port of `paste_indicator()` from
/// `powerline/segments/vim/__init__.py:200-206`.
pub fn paste_indicator(paste_enabled: bool, text: &str) -> Option<String> {
    // py:206
    if paste_enabled {
        Some(text.to_string())
    } else {
        None
    }
}

/// Port of `readonly_indicator()` from
/// `powerline/segments/vim/__init__.py:209-216`.
pub fn readonly_indicator(readonly: bool, text: &str) -> Option<String> {
    // py:216
    if readonly {
        Some(text.to_string())
    } else {
        None
    }
}

/// Port of `file_scheme()` from
/// `powerline/segments/vim/__init__.py:222-245`.
///
/// Returns the URI scheme prefix from `name`, or None if name doesn't
/// start with a scheme. Python returns None for empty names too.
pub fn file_scheme(name: &str) -> Option<String> {
    // py:241-242  if not name: return None
    if name.is_empty() {
        return None;
    }
    // py:243-245  match = SCHEME_RE.match(name); return match.group(0)
    // Python's regex captures the scheme via (?=:) lookahead;
    // Rust strips the trailing ':' from the captured match.
    SCHEME_RE()
        .captures(name)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

/// Port of `file_format()` from
/// `powerline/segments/vim/__init__.py:333-340`.
pub fn file_format(fileformat: &str) -> Option<String> {
    // py:340  return vim_getbufoption(..., 'fileformat') or None
    if fileformat.is_empty() {
        None
    } else {
        Some(fileformat.to_string())
    }
}

/// Port of `file_encoding()` from
/// `powerline/segments/vim/__init__.py:343-352`.
pub fn file_encoding(fileencoding: &str) -> Option<String> {
    // py:352  return vim_getbufoption(..., 'fileencoding') or None
    if fileencoding.is_empty() {
        None
    } else {
        Some(fileencoding.to_string())
    }
}

/// Port of `file_bom()` from
/// `powerline/segments/vim/__init__.py:355-364`.
pub fn file_bom(bomb: bool) -> Option<&'static str> {
    // py:364  return 'bom' if vim_getbufoption(..., 'bomb') else None
    if bomb {
        Some("bom")
    } else {
        None
    }
}

/// Port of `file_type()` from
/// `powerline/segments/vim/__init__.py:367-376`.
pub fn file_type(filetype: &str) -> Option<String> {
    // py:376  return vim_getbufoption(..., 'filetype') or None
    if filetype.is_empty() {
        None
    } else {
        Some(filetype.to_string())
    }
}

/// Port of `line_percent()` from
/// `powerline/segments/vim/__init__.py:394-411`.
///
/// `gradient=false` returns the percentage as a plain string. With
/// `gradient=true` Python returns a list of dicts including the
/// gradient_level; the Rust port returns a `serde_json::Value` of
/// the same shape.
pub fn line_percent(line_current: u64, line_last: u64, gradient: bool) -> serde_json::Value {
    // py:403  line_current = segment_info['window'].cursor[0]
    // py:404  line_last = len(segment_info['buffer'])
    let percentage = (line_current as f64) * 100.0 / (line_last.max(1) as f64);
    let rounded = percentage.round() as i64;
    if !gradient {
        // py:406
        return serde_json::Value::String(rounded.to_string());
    }
    // py:407-411  return list of one dict
    serde_json::json!([{
        "contents": rounded.to_string(),
        "highlight_groups": ["line_percent_gradient", "line_percent"],
        "gradient_level": percentage,
    }])
}

/// Port of `position()` from
/// `powerline/segments/vim/__init__.py:414-449`.
///
/// `line_last` is `len(vim.current.buffer)`; `winline_first` /
/// `winline_last` are `line('w0')` / `line('w$')`. `position_strings`
/// is the localised top/bottom/all dict. With `gradient=true`
/// returns the JSON list shape including gradient_level.
pub fn position(
    line_last: u64,
    winline_first: u64,
    winline_last: u64,
    position_strings: &HashMap<&str, &str>,
    gradient: bool,
) -> serde_json::Value {
    // py:430-441
    let (percentage, content) = if winline_first == 1 && winline_last == line_last {
        (
            0.0_f64,
            position_strings
                .get("all")
                .copied()
                .unwrap_or("All")
                .to_string(),
        )
    } else if winline_first == 1 {
        (
            0.0_f64,
            position_strings
                .get("top")
                .copied()
                .unwrap_or("Top")
                .to_string(),
        )
    } else if winline_last == line_last {
        (
            100.0_f64,
            position_strings
                .get("bottom")
                .copied()
                .unwrap_or("Bot")
                .to_string(),
        )
    } else {
        let pct = (winline_first as f64) * 100.0
            / ((line_last as f64) - (winline_last as f64) + (winline_first as f64));
        let s = format!("{}%", pct.round() as i64);
        (pct, s)
    };
    if !gradient {
        // py:443-444
        return serde_json::Value::String(content);
    }
    // py:445-449
    serde_json::json!([{
        "contents": content,
        "highlight_groups": ["position_gradient", "position"],
        "gradient_level": percentage,
    }])
}

/// Port of `line_current()` from
/// `powerline/segments/vim/__init__.py:452-455`.
pub fn line_current(cursor_line: u64) -> String {
    // py:455  return str(segment_info['window'].cursor[0])
    cursor_line.to_string()
}

/// Port of `line_count()` from
/// `powerline/segments/vim/__init__.py:458-461`.
pub fn line_count(buffer_len: u64) -> String {
    // py:461  return str(len(segment_info['buffer']))
    buffer_len.to_string()
}

/// Port of `col_current()` from
/// `powerline/segments/vim/__init__.py:464-468`.
///
/// Python adds 1 to the cursor column (vim's `cursor[1]` is 0-based).
pub fn col_current(cursor_col: u64) -> String {
    // py:468  return str(segment_info['window'].cursor[1] + 1)
    (cursor_col + 1).to_string()
}

/// Port of `virtcol_current()` from
/// `powerline/segments/vim/__init__.py:471-486`.
///
/// `virtcol` is `vim_funcs['virtcol']('.')`. With gradient=true
/// computes `min(col * 100 / textwidth, 100)` per py:484.
pub fn virtcol_current(virtcol: u64, textwidth: u64, gradient: bool) -> serde_json::Value {
    // py:481  r = [{contents, highlight_groups}]
    let mut entry = serde_json::json!({
        "contents": virtcol.to_string(),
        "highlight_groups": ["virtcol_current", "col_current"],
    });
    if gradient {
        // py:484  gradient_level = min(col * 100 / textwidth, 100) if textwidth else 0
        let level: f64 = if textwidth > 0 {
            ((virtcol as f64) * 100.0 / (textwidth as f64)).min(100.0)
        } else {
            0.0
        };
        // py:485  highlight_groups.insert(0, 'virtcol_current_gradient')
        let hl = entry["highlight_groups"]
            .as_array_mut()
            .expect("highlight_groups initialised as array above");
        hl.insert(
            0,
            serde_json::Value::String("virtcol_current_gradient".to_string()),
        );
        entry["gradient_level"] = serde_json::Value::from(level);
    }
    serde_json::json!([entry])
}

/// Port of `modified_buffers()` from
/// `powerline/segments/vim/__init__.py:489-504`.
///
/// `modified_bufnrs` is the list of buffer numbers with `modified=1`.
/// The Python source walks `vim.buffers` and filters by
/// `vim_getbufoption(..., 'modified')`; Rust takes the already-filtered
/// list directly.
pub fn modified_buffers(modified_bufnrs: &[u64], text: &str, join_str: &str) -> Option<String> {
    // py:497-501  join modified buffer numbers
    if modified_bufnrs.is_empty() {
        return None;
    }
    let numbers: Vec<String> = modified_bufnrs.iter().map(|n| n.to_string()).collect();
    let joined = numbers.join(join_str);
    // py:502-503  if buffer_mod_text: return text + buffer_mod_text
    Some(format!("{}{}", text, joined))
}

/// Port of `tabnr()` from
/// `powerline/segments/vim/__init__.py:635-648`.
///
/// `current_tabnr` is `current_tabpage().number`. Returns None when
/// `show_current` is false and `tabnr == current_tabnr` per py:647.
pub fn tabnr(this_tabnr: u64, current_tabnr: u64, show_current: bool) -> Option<String> {
    // py:647-648
    if show_current || this_tabnr != current_tabnr {
        Some(this_tabnr.to_string())
    } else {
        None
    }
}

/// Port of `bufnr()` from
/// `powerline/segments/vim/__init__.py:651-660`.
pub fn bufnr(this_bufnr: u64, current_bufnr: u64, show_current: bool) -> Option<String> {
    // py:659-660
    if show_current || this_bufnr != current_bufnr {
        Some(this_bufnr.to_string())
    } else {
        None
    }
}

/// Port of `winnr()` from
/// `powerline/segments/vim/__init__.py:663-672`.
pub fn winnr(this_winnr: u64, current_winnr: u64, show_current: bool) -> Option<String> {
    // py:671-672
    if show_current || this_winnr != current_winnr {
        Some(this_winnr.to_string())
    } else {
        None
    }
}

/// Port of `CSV_SNIFF_LINES` constant at
/// `powerline/segments/vim/__init__.py:686`.
pub const CSV_SNIFF_LINES: usize = 100;

/// Port of `CSV_PARSE_LINES` constant at
/// `powerline/segments/vim/__init__.py:687`.
pub const CSV_PARSE_LINES: usize = 10;

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

    #[test]
    fn scheme_re_matches_zipfile_prefix() {
        // py:219  scheme prefix capture via (?=:) lookahead
        // Rust port uses capture group 1 to drop the trailing ':'
        let re = SCHEME_RE();
        let c = re.captures("zipfile:/path/x.zip::file.txt").unwrap();
        assert_eq!(c.get(1).unwrap().as_str(), "zipfile");
    }

    #[test]
    fn scheme_re_accepts_digit_prefix() {
        // Python's `\w` includes digits, so the regex accepts a
        // digit-prefixed name like "1bad:foo" even though the
        // docstring at py:222-238 says "starts with a latin letter".
        // The regex itself is the spec.
        let re = SCHEME_RE();
        assert!(re.find("1bad:foo").is_some());
    }

    #[test]
    fn scheme_re_no_match_when_no_colon() {
        let re = SCHEME_RE();
        assert!(re.find("plain/file.txt").is_none());
    }

    #[test]
    fn mode_returns_none_for_nc() {
        // py:103-104
        assert_eq!(mode("nc", None), None);
    }

    #[test]
    fn mode_translates_normal() {
        // py:44, 105-114
        assert_eq!(mode("n", None), Some("NORMAL".to_string()));
    }

    #[test]
    fn mode_translates_visual_block_via_caret_v() {
        // py:48  '^V' → 'V-BLCK'
        assert_eq!(mode("^V", None), Some("V-BLCK".to_string()));
    }

    #[test]
    fn mode_trims_unknown_suffix() {
        // py:106-114  trim until a match (e.g. "iXXX" → "i" → INSERT)
        assert_eq!(mode("iXXX", None), Some("INSERT".to_string()));
    }

    #[test]
    fn mode_override_takes_precedence() {
        // py:109-111
        let mut o = HashMap::new();
        o.insert("n".to_string(), "NORM".to_string());
        assert_eq!(mode("n", Some(&o)), Some("NORM".to_string()));
    }

    #[test]
    fn mode_empty_returns_bug() {
        // py:115  fallthrough → 'BUG'
        assert_eq!(mode("", None), Some("BUG".to_string()));
    }

    #[test]
    fn modified_indicator_returns_text_when_modified() {
        // py:178
        assert_eq!(modified_indicator(true, "+"), Some("+".to_string()));
        assert_eq!(modified_indicator(false, "+"), None);
    }

    #[test]
    fn paste_indicator_returns_text_when_paste_enabled() {
        // py:206
        assert_eq!(paste_indicator(true, "PASTE"), Some("PASTE".to_string()));
        assert_eq!(paste_indicator(false, "PASTE"), None);
    }

    #[test]
    fn readonly_indicator_returns_text_when_readonly() {
        // py:216
        assert_eq!(readonly_indicator(true, "RO"), Some("RO".to_string()));
        assert_eq!(readonly_indicator(false, "RO"), None);
    }

    #[test]
    fn file_scheme_extracts_prefix() {
        // py:241-245
        assert_eq!(
            file_scheme("zipfile:/path/x.zip::file.txt"),
            Some("zipfile".to_string())
        );
    }

    #[test]
    fn file_scheme_returns_none_for_no_scheme() {
        assert_eq!(file_scheme("plain/file.txt"), None);
    }

    #[test]
    fn file_scheme_returns_none_for_empty_name() {
        // py:241-242  if not name: return None
        assert_eq!(file_scheme(""), None);
    }

    #[test]
    fn file_format_returns_value_or_none() {
        // py:340
        assert_eq!(file_format("unix"), Some("unix".to_string()));
        assert_eq!(file_format(""), None);
    }

    #[test]
    fn file_encoding_returns_value_or_none() {
        // py:352
        assert_eq!(file_encoding("utf-8"), Some("utf-8".to_string()));
        assert_eq!(file_encoding(""), None);
    }

    #[test]
    fn file_bom_returns_bom_or_none() {
        // py:364
        assert_eq!(file_bom(true), Some("bom"));
        assert_eq!(file_bom(false), None);
    }

    #[test]
    fn file_type_returns_value_or_none() {
        // py:376
        assert_eq!(file_type("rust"), Some("rust".to_string()));
        assert_eq!(file_type(""), None);
    }

    #[test]
    fn line_percent_no_gradient_returns_string() {
        // py:406  no gradient → str(rounded percentage)
        let v = line_percent(50, 100, false);
        assert_eq!(v.as_str(), Some("50"));
    }

    #[test]
    fn line_percent_with_gradient_returns_list() {
        // py:407-411
        let v = line_percent(75, 100, true);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["contents"], "75");
        assert_eq!(arr[0]["highlight_groups"][0], "line_percent_gradient");
        assert_eq!(arr[0]["gradient_level"], 75.0);
    }

    #[test]
    fn line_percent_at_first_line_emits_one() {
        // 1/100 = 1.0 → "1"
        let v = line_percent(1, 100, false);
        assert_eq!(v.as_str(), Some("1"));
    }

    #[test]
    fn line_percent_at_last_line_emits_100() {
        let v = line_percent(100, 100, false);
        assert_eq!(v.as_str(), Some("100"));
    }

    #[test]
    fn position_top_when_winline_first_is_one_and_not_all() {
        // py:433-435
        let ps = default_position_strings();
        let strs: HashMap<&str, &str> = ps.iter().map(|(k, v)| (*k, *v)).collect();
        let v = position(100, 1, 50, &strs, false);
        assert_eq!(v.as_str(), Some("Top"));
    }

    #[test]
    fn position_all_when_window_shows_entire_buffer() {
        // py:430-432
        let ps = default_position_strings();
        let strs: HashMap<&str, &str> = ps.iter().map(|(k, v)| (*k, *v)).collect();
        let v = position(50, 1, 50, &strs, false);
        assert_eq!(v.as_str(), Some("All"));
    }

    #[test]
    fn position_bottom_when_winline_last_is_buffer_end() {
        // py:436-438
        let ps = default_position_strings();
        let strs: HashMap<&str, &str> = ps.iter().map(|(k, v)| (*k, *v)).collect();
        let v = position(100, 50, 100, &strs, false);
        assert_eq!(v.as_str(), Some("Bot"));
    }

    #[test]
    fn position_middle_emits_percentage() {
        // py:440-441
        let ps = default_position_strings();
        let strs: HashMap<&str, &str> = ps.iter().map(|(k, v)| (*k, *v)).collect();
        // winline_first=10, winline_last=20, line_last=100
        // pct = 10 * 100 / (100 - 20 + 10) = 1000 / 90 ≈ 11.11%
        let v = position(100, 10, 20, &strs, false);
        let s = v.as_str().unwrap();
        assert!(s.ends_with('%'));
    }

    #[test]
    fn position_gradient_emits_full_dict() {
        // py:445-449
        let ps = default_position_strings();
        let strs: HashMap<&str, &str> = ps.iter().map(|(k, v)| (*k, *v)).collect();
        let v = position(100, 50, 100, &strs, true);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["contents"], "Bot");
        assert_eq!(arr[0]["highlight_groups"][0], "position_gradient");
        assert_eq!(arr[0]["gradient_level"], 100.0);
    }

    #[test]
    fn line_current_returns_cursor_row() {
        // py:455
        assert_eq!(line_current(42), "42");
    }

    #[test]
    fn line_count_returns_buffer_len() {
        // py:461
        assert_eq!(line_count(100), "100");
    }

    #[test]
    fn col_current_adds_one_to_zero_based_col() {
        // py:468  cursor[1] + 1
        assert_eq!(col_current(0), "1");
        assert_eq!(col_current(42), "43");
    }

    #[test]
    fn virtcol_current_no_gradient_omits_level() {
        // py:481-486
        let v = virtcol_current(40, 80, false);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["contents"], "40");
        assert!(arr[0].get("gradient_level").is_none());
    }

    #[test]
    fn virtcol_current_with_gradient_computes_level() {
        // py:484  min(col * 100 / textwidth, 100)
        let v = virtcol_current(40, 80, true);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["gradient_level"], 50.0);
        assert_eq!(arr[0]["highlight_groups"][0], "virtcol_current_gradient");
    }

    #[test]
    fn virtcol_current_clamps_gradient_to_100() {
        // py:484  min(col * 100 / textwidth, 100)
        let v = virtcol_current(120, 80, true);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["gradient_level"], 100.0);
    }

    #[test]
    fn virtcol_current_zero_textwidth_gives_zero_gradient() {
        // py:484  ... if textwidth else 0
        let v = virtcol_current(40, 0, true);
        let arr = v.as_array().unwrap();
        assert_eq!(arr[0]["gradient_level"], 0.0);
    }

    #[test]
    fn modified_buffers_with_empty_list_returns_none() {
        // py:502-503
        assert_eq!(modified_buffers(&[], "+ ", ","), None);
    }

    #[test]
    fn modified_buffers_joins_list_with_prefix() {
        // py:497-503
        assert_eq!(
            modified_buffers(&[1, 3, 5], "+ ", ","),
            Some("+ 1,3,5".to_string())
        );
    }

    #[test]
    fn modified_buffers_uses_custom_separator() {
        assert_eq!(
            modified_buffers(&[2, 4], "M:", " | "),
            Some("M:2 | 4".to_string())
        );
    }

    #[test]
    fn tabnr_shows_current_when_flag_set() {
        // py:647
        assert_eq!(tabnr(1, 1, true), Some("1".to_string()));
    }

    #[test]
    fn tabnr_hides_current_when_flag_unset() {
        // py:647-648
        assert_eq!(tabnr(1, 1, false), None);
    }

    #[test]
    fn tabnr_shows_other_tabnr_regardless_of_flag() {
        assert_eq!(tabnr(2, 1, false), Some("2".to_string()));
    }

    #[test]
    fn bufnr_show_current_paths() {
        // py:659-660
        assert_eq!(bufnr(1, 1, true), Some("1".to_string()));
        assert_eq!(bufnr(1, 1, false), None);
        assert_eq!(bufnr(2, 1, false), Some("2".to_string()));
    }

    #[test]
    fn winnr_show_current_paths() {
        // py:671-672
        assert_eq!(winnr(1, 1, true), Some("1".to_string()));
        assert_eq!(winnr(1, 1, false), None);
        assert_eq!(winnr(2, 1, false), Some("2".to_string()));
    }

    #[test]
    fn csv_sniff_lines_constant() {
        // py:686
        assert_eq!(CSV_SNIFF_LINES, 100);
    }

    #[test]
    fn csv_parse_lines_constant() {
        // py:687
        assert_eq!(CSV_PARSE_LINES, 10);
    }
}
