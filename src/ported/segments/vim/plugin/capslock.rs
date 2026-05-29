// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/capslock.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import vim_func_exists                                     // py:9
// from powerline.theme import requires_segment_info                                      // py:10

use crate::ported::bindings::vim::{vim_func_exists, MatcherInfo};

/// Port of `capslock_indicator()` from
/// `powerline/segments/vim/plugin/capslock.py:14`.
///
/// Shows the indicator if tpope/vim-capslock plugin is enabled.
///
/// :param text: String to show when software capslock is active.
///
/// Rust port: without vim.eval, the `CapsLockStatusline()` call returns
/// empty. Returns `None` when the function isn't defined (matches py:25
/// short-circuit) or when the eval returns empty (py:28).
pub fn capslock_indicator(
    _pl: &(),
    _segment_info: &MatcherInfo,
    text: &str,
) -> Option<String> {
    // py:25  if not vim_func_exists('CapsLockStatusline'): return None
    if !vim_func_exists("CapsLockStatusline") {
        return None;
    }
    // py:28  return text if vim.eval('CapsLockStatusline()') else None
    let active = false; // vim.eval stub yields empty/false
    if active {
        Some(text.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capslock_returns_none_without_vim() {
        let info = MatcherInfo::default();
        assert!(capslock_indicator(&(), &info, "CAPS").is_none());
    }
}
