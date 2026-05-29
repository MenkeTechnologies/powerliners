// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/vim/__init__.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from powerline.bindings.vim import vim_getbufoption, buffer_name                        // py:6

pub mod plugin;

use crate::ported::bindings::vim::{buffer_name, vim_getbufoption, MatcherInfo};

/// Port of `help()` from `powerline/matchers/vim/__init__.py:9`.
pub fn help(matcher_info: &MatcherInfo) -> bool {
    // py:10  return vim_getbufoption(matcher_info, 'buftype') == 'help'
    vim_getbufoption(matcher_info, "buftype") == "help"
}

/// Port of `cmdwin()` from `powerline/matchers/vim/__init__.py:13`.
pub fn cmdwin(matcher_info: &MatcherInfo) -> bool {
    // py:14  name = buffer_name(matcher_info)
    let name = match buffer_name(matcher_info) {
        Some(n) => n,
        None => return false,
    };
    // py:15  name and os.path.basename(name) == b'[Command Line]'
    let basename = name.rsplitn(2, |&b| b == b'/').next().unwrap_or(&name);
    basename == b"[Command Line]"
}

/// Port of `quickfix()` from `powerline/matchers/vim/__init__.py:18`.
pub fn quickfix(matcher_info: &MatcherInfo) -> bool {
    // py:19  return vim_getbufoption(matcher_info, 'buftype') == 'quickfix'
    vim_getbufoption(matcher_info, "buftype") == "quickfix"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_buftype(buftype: &str) -> MatcherInfo {
        let mut opts = std::collections::HashMap::new();
        opts.insert("buftype".to_string(), buftype.to_string());
        MatcherInfo {
            buffer_options: opts,
            ..Default::default()
        }
    }

    #[test]
    fn help_matches_help_buftype() {
        assert!(help(&info_with_buftype("help")));
        assert!(!help(&info_with_buftype("")));
        assert!(!help(&info_with_buftype("quickfix")));
    }

    #[test]
    fn quickfix_matches_quickfix_buftype() {
        assert!(quickfix(&info_with_buftype("quickfix")));
        assert!(!quickfix(&info_with_buftype("")));
        assert!(!quickfix(&info_with_buftype("help")));
    }

    #[test]
    fn cmdwin_matches_command_line_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"[Command Line]".to_vec()),
            ..Default::default()
        };
        assert!(cmdwin(&info));
    }

    #[test]
    fn cmdwin_matches_path_prefixed_buffer() {
        let info = MatcherInfo {
            buffer_name: Some(b"/some/path/[Command Line]".to_vec()),
            ..Default::default()
        };
        assert!(cmdwin(&info));
    }

    #[test]
    fn cmdwin_rejects_other_buffers() {
        let info = MatcherInfo {
            buffer_name: Some(b"main.rs".to_vec()),
            ..Default::default()
        };
        assert!(!cmdwin(&info));
    }

    #[test]
    fn cmdwin_returns_false_for_empty_name() {
        let info = MatcherInfo::default();
        assert!(!cmdwin(&info));
    }
}
