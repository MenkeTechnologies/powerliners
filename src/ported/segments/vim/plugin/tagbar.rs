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
pub fn current_tag(_segment_info: &MatcherInfo, _pl: &(), _flags: &str) -> Option<String> {
    // py:17  @requires_segment_info
    // py:18  def current_tag(segment_info, pl, flags='s'):
    // py:19-32  docstring
    // py:33  global currenttag
    // py:34  global tag_cache
    // py:35  window_id = segment_info['window_id']
    // py:36  if segment_info['mode'] == 'nc':
    // py:37  return tag_cache.get(window_id, (None,))[-1]
    // py:38  if not currenttag:
    // py:39  if vim_command_exists('Tagbar'):
    if !vim_command_exists("Tagbar") {
        // py:43  else:
        // py:44  return None
        return None;
    }
    // py:40  currenttag = vim_get_autoload_func('tagbar#currenttag')
    // py:41  if not currenttag:
    // py:42  return None
    // py:45  prev_key, r = tag_cache.get(window_id, (None, None))
    // py:46  key = (int(vim.eval('b:changedtick')), segment_info['window'].cursor[0])
    // py:47  if prev_key and key == prev_key:
    // py:48  return r
    // py:49  r = currenttag('%s', '', flags)
    // py:50  tag_cache[window_id] = (key, r)
    // py:51  return r
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
