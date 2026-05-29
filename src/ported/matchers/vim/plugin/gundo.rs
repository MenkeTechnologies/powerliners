// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/vim/plugin/gundo.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from powerline.bindings.vim import buffer_name   // py:6

use crate::ported::bindings::vim::{buffer_name, MatcherInfo};

/// Port of `gundo()` from
/// `powerline/matchers/vim/plugin/gundo.py:9`.
pub fn gundo(matcher_info: &MatcherInfo) -> bool {
    // py:10  name = buffer_name(matcher_info)
    let name = match buffer_name(matcher_info) {
        Some(n) => n,
        None => return false,
    };
    // py:11  name and os.path.basename(name) == b'__Gundo__'
    let basename = name.rsplitn(2, |&b| b == b'/').next().unwrap_or(&name);
    basename == b"__Gundo__"
}

/// Port of `gundo_preview()` from
/// `powerline/matchers/vim/plugin/gundo.py:14`.
pub fn gundo_preview(matcher_info: &MatcherInfo) -> bool {
    // py:15  name = buffer_name(matcher_info)
    let name = match buffer_name(matcher_info) {
        Some(n) => n,
        None => return false,
    };
    // py:16  name and os.path.basename(name) == b'__Gundo_Preview__'
    let basename = name.rsplitn(2, |&b| b == b'/').next().unwrap_or(&name);
    basename == b"__Gundo_Preview__"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gundo_matches_gundo_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"__Gundo__".to_vec()),
            ..Default::default()
        };
        assert!(gundo(&info));
        assert!(!gundo_preview(&info));
    }

    #[test]
    fn gundo_preview_matches_preview_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"__Gundo_Preview__".to_vec()),
            ..Default::default()
        };
        assert!(gundo_preview(&info));
        assert!(!gundo(&info));
    }

    #[test]
    fn gundo_rejects_other_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"main.rs".to_vec()),
            ..Default::default()
        };
        assert!(!gundo(&info));
        assert!(!gundo_preview(&info));
    }
}
