// vim:fileencoding=utf-8:noet
//! Port of `powerline/selectors/vim.py`.
//!
//! Vim segment selectors — predicates that determine whether a segment
//! should render based on the current vim state.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.bindings.vim import list_tabpages                                        // py:4

use crate::ported::bindings::vim::{list_tabpages, MatcherInfo};

/// Port of `single_tab()` from `powerline/selectors/vim.py:7`.
///
/// Returns true if Vim has only one tab opened.
pub fn single_tab(_pl: &(), _segment_info: &MatcherInfo, _mode: &str) -> bool {
    // py:10  return len(list_tabpages()) == 1
    list_tabpages().len() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_tab_returns_false_when_no_vim() {
        // Without a live vim, list_tabpages() returns empty → 0 != 1 → false.
        let info = MatcherInfo::default();
        assert!(!single_tab(&(), &info, "normal"));
    }
}
