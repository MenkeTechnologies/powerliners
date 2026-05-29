// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/coc.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import vim_command_exists                                   // py:9
// from powerline.theme import requires_segment_info                                       // py:10

use crate::ported::bindings::vim::{vim_command_exists, MatcherInfo};
use serde_json::{json, Value};

/// Port of `parse_coc_status()` from
/// `powerline/segments/vim/plugin/coc.py:13`.
///
/// coc_status's format: `E1 W2` — parses into (errors, warnings) counts.
///
/// Python takes a `tuple`; the only caller passes a single-element
/// tuple containing the vim eval result. Rust port takes the raw
/// status string directly since the wrapping tuple was Python sugar.
pub fn parse_coc_status(coc_status: &str) -> (i32, i32) {
    // py:16-17  errors_count = 0; warnings_count = 0
    let mut errors_count = 0;
    let mut warnings_count = 0;
    // py:18-23  if len(coc_status) <= 0 / status_str empty → return zeros
    if coc_status.is_empty() {
        return (errors_count, warnings_count);
    }
    // py:24-29  status_list = status_str.split(' '); for item in ...
    for item in coc_status.split(' ') {
        let bytes = item.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        // py:26-29  E<n> → errors_count = int(rest); W<n> → warnings_count
        if bytes[0] == b'E' {
            if let Ok(n) = item[1..].parse() {
                errors_count = n;
            }
        } else if bytes[0] == b'W' {
            if let Ok(n) = item[1..].parse() {
                warnings_count = n;
            }
        }
    }
    (errors_count, warnings_count)
}

/// Port of `coc()` from `powerline/segments/vim/plugin/coc.py:32`.
///
/// Show whether coc.nvim has found any errors or warnings.
///
/// Highlight groups used: `coc:warning` or `warning`, `coc:error` or `error`.
///
/// Rust port: returns empty Vec when CocCommand isn't defined
/// (matches py:37-38). With vim integration the body would call
/// `coc#status()` and dispatch on counts.
pub fn coc(_segment_info: &MatcherInfo, _pl: &()) -> Vec<Value> {
    // py:36  segments = []
    let mut segments = Vec::new();
    // py:37-38  if not vim_command_exists('CocCommand'): return segments
    if !vim_command_exists("CocCommand") {
        return segments;
    }
    // py:39-40  coc_status = vim.eval('coc#status()'),
    //           errors_count, warnings_count = parse_coc_status(coc_status)
    let coc_status = ""; // stub: vim.eval not wired
    let (errors_count, warnings_count) = parse_coc_status(coc_status);
    // py:41-44  if errors_count > 0: append({E:N, [coc:error, error]})
    if errors_count > 0 {
        segments.push(json!({
            "contents": format!("E:{}", errors_count),
            "highlight_groups": ["coc:error", "error"],
        }));
    }
    // py:45-50  if warnings_count > 0: append({W:N, [coc:warning, warning]})
    if warnings_count > 0 {
        segments.push(json!({
            "contents": format!("W:{}", warnings_count),
            "highlight_groups": ["coc:warning", "warning"],
        }));
    }
    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coc_status_empty_returns_zeros() {
        assert_eq!(parse_coc_status(""), (0, 0));
    }

    #[test]
    fn parse_coc_status_parses_errors_only() {
        assert_eq!(parse_coc_status("E5"), (5, 0));
    }

    #[test]
    fn parse_coc_status_parses_warnings_only() {
        assert_eq!(parse_coc_status("W3"), (0, 3));
    }

    #[test]
    fn parse_coc_status_parses_both() {
        assert_eq!(parse_coc_status("E2 W7"), (2, 7));
        assert_eq!(parse_coc_status("W3 E12"), (12, 3));
    }

    #[test]
    fn parse_coc_status_ignores_unknown_prefix() {
        assert_eq!(parse_coc_status("I9"), (0, 0));
    }

    #[test]
    fn coc_returns_empty_without_vim() {
        let info = MatcherInfo::default();
        assert!(coc(&info, &()).is_empty());
    }
}
