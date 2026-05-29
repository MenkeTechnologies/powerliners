// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/nerdtree.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import bufvar_exists                                       // py:9
// from powerline.segments.vim import window_cached                                       // py:10

use crate::ported::bindings::vim::bufvar_exists;
use serde_json::{json, Value};

/// Port of `nerdtree()` from
/// `powerline/segments/vim/plugin/nerdtree.py:14`.
///
/// Return directory that is shown by the current buffer.
///
/// Highlight groups used: `nerdtree:path` or `file_name`.
///
/// Rust port: without `vim.eval(...)` we cannot read NERDTreeRoot's
/// path. Returns `None` (matches the upstream py:17 short-circuit
/// when NERDTreeRoot is not set).
pub fn nerdtree(_pl: &()) -> Option<Vec<Value>> {
    // py:17  if not bufvar_exists(None, 'NERDTreeRoot'): return None
    if !bufvar_exists(None, "NERDTreeRoot") {
        return None;
    }
    // py:18-22  path_str = vim.eval(...); return [{contents, highlight_groups}]
    let path_str = String::new(); // vim.eval stub yields empty
    Some(vec![json!({
        "contents": path_str,
        "highlight_groups": ["nerdtree:path", "file_name"]
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nerdtree_returns_none_without_vim() {
        // bufvar_exists stub returns false → None.
        assert!(nerdtree(&()).is_none());
    }
}
