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
    // py:13  def parse_coc_status(coc_status):
    // py:14  # type(coc_status) is tuple
    // py:15  errors_count = 0
    // py:16  warnings_count = 0
    let mut errors_count = 0;
    let mut warnings_count = 0;
    // py:17  if len(coc_status) <= 0:
    // py:18  return errors_count, warnings_count
    // py:19  status_str = coc_status[0]
    // py:20  if len(status_str) <= 0:
    // py:21  return errors_count, warnings_count
    if coc_status.is_empty() {
        return (errors_count, warnings_count);
    }
    // py:22  status_list = status_str.split(' ')
    // py:23  for item in status_list:
    for item in coc_status.split(' ') {
        let bytes = item.as_bytes();
        if bytes.is_empty() {
            continue;
        }
        // py:24  if len(item) > 0 and item[0] == 'E':
        // py:25  errors_count = int(item[1:])
        if bytes[0] == b'E' {
            if let Ok(n) = item[1..].parse() {
                errors_count = n;
            }
        // py:26  if len(item) > 0 and item[0] == 'W':
        // py:27  warnings_count = int(item[1:])
        } else if bytes[0] == b'W' {
            if let Ok(n) = item[1..].parse() {
                warnings_count = n;
            }
        }
    }
    // py:28  return errors_count, warnings_count
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
    // py:30  @requires_segment_info
    // py:31  def coc(segment_info, pl):
    // py:32-35  docstring
    // py:36  segments = []
    let mut segments = Vec::new();
    // py:37  if not vim_command_exists('CocCommand'):
    if !vim_command_exists("CocCommand") {
        // py:38  return segments
        return segments;
    }
    // py:39  coc_status = vim.eval('coc#status()'),
    let coc_status = "";
    // py:40  errors_count, warnings_count = parse_coc_status(coc_status)
    let (errors_count, warnings_count) = parse_coc_status(coc_status);
    // py:41  if errors_count > 0:
    if errors_count > 0 {
        // py:42  segments.append({
        // py:43  'contents': 'E:' + str(errors_count),
        // py:44  'highlight_groups': ['coc:error', 'error'],
        // py:45  })
        segments.push(json!({
            "contents": format!("E:{}", errors_count),
            "highlight_groups": ["coc:error", "error"],
        }));
    }
    // py:46  if warnings_count > 0:
    if warnings_count > 0 {
        // py:47  segments.append({
        // py:48  'contents': 'W:' + str(warnings_count),
        // py:49  'highlight_groups': ['coc:warning', 'warning'],
        // py:50  })
        segments.push(json!({
            "contents": format!("W:{}", warnings_count),
            "highlight_groups": ["coc:warning", "warning"],
        }));
    }
    // py:51  return segments
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
