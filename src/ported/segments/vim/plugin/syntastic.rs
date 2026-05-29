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
pub fn syntastic(
    _pl: &(),
    _err_format: &str,
    _warn_format: &str,
) -> Option<Vec<Value>> {
    // py:24  if not vim_global_exists('SyntasticLoclist'): return None
    if !vim_global_exists("SyntasticLoclist") {
        return None;
    }
    // py:26-29  has_errors = int(vim.eval('g:SyntasticLoclist.current().hasErrorsOrWarningsToDisplay()'))
    //           if not has_errors: return
    // Stub: no vim → no errors.
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
