// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/syntastic.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.segments.vim import window_cached                                       // py:9
// from powerline.bindings.vim import vim_global_exists                                   // py:10

use crate::ported::bindings::vim::vim_global_exists;
use serde_json::Value;

/// Port of `syntastic()` from
/// `powerline/segments/vim/plugin/syntastic.py:14`.
///
/// Show whether syntastic has found any errors or warnings.
///
/// :param err_format: Format string for errors.
/// :param warn_format: Format string for warnings.
///
/// Highlight groups used: `syntastic:warning` or `warning`,
/// `syntastic:error` or `error`.
///
/// Rust port: without vim.eval, returns `None` when `g:SyntasticLoclist`
/// isn't defined (matches py:24 short-circuit). The error/warning
/// extraction logic (py:26-44) is preserved structurally for when
/// vim integration lands.
pub fn syntastic(_pl: &(), _err_format: &str, _warn_format: &str) -> Option<Vec<Value>> {
    // py:13  @window_cached
    // py:14  def syntastic(pl, err_format='ERR:  {first_line} ({num}) ', warn_format='WARN:  {first_line} ({num}) '):
    // py:15-24  docstring
    // py:25  if not vim_global_exists('SyntasticLoclist'):
    if !vim_global_exists("SyntasticLoclist") {
        // py:26  return None
        return None;
    }
    // py:27  has_errors = int(vim.eval('g:SyntasticLoclist.current().hasErrorsOrWarningsToDisplay()'))
    // py:28  if not has_errors:
    // py:29  return
    // py:30  errors = vim.eval('g:SyntasticLoclist.current().errors()')
    // py:31  warnings = vim.eval('g:SyntasticLoclist.current().warnings()')
    // py:32  segments = []
    // py:33  if errors:
    // py:34  segments.append({
    // py:35  'contents': err_format.format(first_line=errors[0]['lnum'], num=len(errors)),
    // py:36  'highlight_groups': ['syntastic:error', 'error'],
    // py:37  })
    // py:38  if warnings:
    // py:39  segments.append({
    // py:40  'contents': warn_format.format(first_line=warnings[0]['lnum'], num=len(warnings)),
    // py:41  'highlight_groups': ['syntastic:warning', 'warning'],
    // py:42  })
    // py:43  return segments
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syntastic_returns_none_without_vim() {
        assert!(syntastic(&(), "ERR:  {first_line} ({num}) ", "WARN: ").is_none());
    }
}
