// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/vim/plugin/nerdtree.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// from powerline.bindings.vim import buffer_name   // py:7

use crate::ported::bindings::vim::{buffer_name, MatcherInfo};
use regex::bytes::Regex;
use std::sync::OnceLock;

/// Port of module-level binding `NERD_TREE_RE` from
/// `powerline/matchers/vim/plugin/nerdtree.py:10`.
///
/// Python: `NERD_TREE_RE = re.compile(b'NERD_tree_\\d+')`.
/// Matches NERDTree's buffer-name pattern.
#[allow(non_snake_case)]
pub fn NERD_TREE_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"NERD_tree_\d+").unwrap())
}

/// Port of `nerdtree()` from
/// `powerline/matchers/vim/plugin/nerdtree.py:13`.
pub fn nerdtree(matcher_info: &MatcherInfo) -> bool {
    // py:14  name = buffer_name(matcher_info)
    let name = match buffer_name(matcher_info) {
        Some(n) if !n.is_empty() => n,
        _ => return false,                           // py:15  name and ...
    };
    // py:15  NERD_TREE_RE.match(os.path.basename(name))
    let basename = name
        .rsplitn(2, |&b| b == b'/')
        .next()
        .unwrap_or(&name);
    NERD_TREE_RE().is_match(basename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nerdtree_matches_nerd_tree_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"NERD_tree_1".to_vec()),
            ..Default::default()
        };
        assert!(nerdtree(&info));
    }

    #[test]
    fn nerdtree_matches_path_prefixed_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"/tmp/NERD_tree_42".to_vec()),
            ..Default::default()
        };
        assert!(nerdtree(&info));
    }

    #[test]
    fn nerdtree_rejects_regular_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"main.rs".to_vec()),
            ..Default::default()
        };
        assert!(!nerdtree(&info));
    }

    #[test]
    fn nerdtree_returns_false_for_empty_name() {
        let info = MatcherInfo::default();
        assert!(!nerdtree(&info));
    }
}
