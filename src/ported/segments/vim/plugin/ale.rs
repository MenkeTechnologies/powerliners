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
    // py:24  if not (vim_global_exists('ale_enabled') and ...): return None
    if !vim_global_exists("ale_enabled") {
        return None;
    }
    // py:25-44  has_errors loop — stub returns None (no vim).
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
