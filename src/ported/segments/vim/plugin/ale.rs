// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/ale.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import vim_global_exists                                   // py:9
// from powerline.theme import requires_segment_info                                      // py:10

use crate::ported::bindings::vim::{vim_global_exists, MatcherInfo};
use serde_json::Value;

/// Port of `ale()` from `powerline/segments/vim/plugin/ale.py:14`.
///
/// Show whether ALE has found any errors or warnings.
///
/// :param err_format: Format string for errors.
/// :param warn_format: Format string for warnings.
///
/// Highlight groups used: `ale:warning` or `warning`, `ale:error` or `error`.
///
/// Rust port: returns `None` until vim integration lands (matches the
/// upstream py:24 short-circuit when ALE isn't enabled).
pub fn ale(
    _segment_info: &MatcherInfo,
    _pl: &(),
    _err_format: &str,
    _warn_format: &str,
) -> Option<Vec<Value>> {
    // py:13  @requires_segment_info
    // py:14  def ale(segment_info, pl, err_format='ERR: ln {first_line} ({num}) ', warn_format='WARN: ln {first_line} ({num}) '):
    // py:15-24  docstring
    // py:25  if not (vim_global_exists('ale_enabled') and int(vim.eval('g:ale_enabled'))):
    if !vim_global_exists("ale_enabled") {
        // py:26  return None
        return None;
    }
    // py:27  has_errors = int(vim.eval('ale#statusline#Count(' + str(segment_info['bufnr']) + ').total'))
    // py:28  if not has_errors:
    // py:29  return
    // py:30  error = None
    // py:31  warning = None
    // py:32  errors_count = 0
    // py:33  warnings_count = 0
    // py:34  for issue in vim.eval('ale#engine#GetLoclist(' + str(segment_info['bufnr']) + ')'):
    // py:35  if issue['type'] == 'E':
    // py:36  error = error or issue
    // py:37  errors_count += 1
    // py:38  elif issue['type'] == 'W':
    // py:39  warning = warning or issue
    // py:40  warnings_count += 1
    // py:41  segments = []
    // py:42  if error:
    // py:43  segments.append({
    // py:44  'contents': err_format.format(first_line=error['lnum'], num=errors_count),
    // py:45  'highlight_groups': ['ale:error', 'error'],
    // py:46  })
    // py:47  if warning:
    // py:48  segments.append({
    // py:49  'contents': warn_format.format(first_line=warning['lnum'], num=warnings_count),
    // py:50  'highlight_groups': ['ale:warning', 'warning'],
    // py:51  })
    // py:52  return segments
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ale_returns_none_without_vim() {
        let info = MatcherInfo::default();
        assert!(ale(&info, &(), "ERR: ", "WARN: ").is_none());
    }
}
