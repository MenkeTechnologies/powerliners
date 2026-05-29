// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/tagbar.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import vim_command_exists, vim_get_autoload_func           // py:9
// from powerline.theme import requires_segment_info                                      // py:10

use crate::ported::bindings::vim::{vim_command_exists, MatcherInfo};

// py:13  currenttag = None  (module-level cache — deferred to thread-local
// when the vim integration lands)
// py:14  tag_cache = {}     (same)

/// Port of `current_tag()` from
/// `powerline/segments/vim/plugin/tagbar.py:17`.
///
/// Return tag that is near the cursor.
///
/// :param flags: Specifies additional properties of the displayed tag:
///   - `s` - display complete signature
///   - `f` - display the full hierarchy of the tag
///   - `p` - display the raw prototype
///
/// Rust port: returns `None` until vim integration lands (matches the
/// upstream py:34/37 short-circuit when Tagbar isn't available).
pub fn current_tag(
    _segment_info: &MatcherInfo,
    _pl: &(),
    _flags: &str,
) -> Option<String> {
    // py:33  if not currenttag:
    if !vim_command_exists("Tagbar") {
        return None;
    }
    // py:43-45  current tag lookup via vim eval — stub returns None
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_tag_returns_none_without_vim() {
        let info = MatcherInfo::default();
        assert!(current_tag(&info, &(), "s").is_none());
    }
}
