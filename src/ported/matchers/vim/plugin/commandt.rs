// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/vim/plugin/commandt.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from powerline.bindings.vim import vim_getbufoption, buffer_name                          // py:6

use crate::ported::bindings::vim::{buffer_name, vim_getbufoption, MatcherInfo};

/// Port of `commandt()` from
/// `powerline/matchers/vim/plugin/commandt.py:9`.
pub fn commandt(matcher_info: &MatcherInfo) -> bool {
    // py:10  name = buffer_name(matcher_info)
    let name = buffer_name(matcher_info);
    // py:11-14  vim_getbufoption(...) == 'command-t'
    //           or (name and os.path.basename(name) == b'GoToFile')
    if vim_getbufoption(matcher_info, "filetype") == "command-t" {
        return true;
    }
    if let Some(n) = name {
        let basename = n.rsplitn(2, |&b| b == b'/').next().unwrap_or(&n);
        return basename == b"GoToFile";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commandt_matches_filetype() {
        let mut opts = std::collections::HashMap::new();
        opts.insert("filetype".into(), "command-t".into());
        let info = MatcherInfo {
            buffer_options: opts,
            ..Default::default()
        };
        assert!(commandt(&info));
    }

    #[test]
    fn commandt_matches_gotofile_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"/path/to/GoToFile".to_vec()),
            ..Default::default()
        };
        assert!(commandt(&info));
    }

    #[test]
    fn commandt_rejects_other_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"main.rs".to_vec()),
            ..Default::default()
        };
        assert!(!commandt(&info));
    }
}
