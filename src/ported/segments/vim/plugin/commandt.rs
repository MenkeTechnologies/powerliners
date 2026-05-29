// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/commandt.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try: import vim except ImportError: vim = object()                                     // py:4-7
// from powerline.bindings.vim import create_ruby_dpowerline                              // py:9

use crate::ported::bindings::vim::create_ruby_dpowerline;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::OnceLock;

// py:12-39  initialize() — sets up the ruby $powerline globals in vim's
// embedded interpreter. Rust port: tracks the one-shot flag via OnceLock;
// the actual ruby setup is a no-op until vim integration lands.

static INITIALIZED: OnceLock<bool> = OnceLock::new();

/// Port of `initialize()` from
/// `powerline/segments/vim/plugin/commandt.py:12`.
pub fn initialize() {
    // py:12  def initialize():
    // py:13  global initialized
    // py:14  if initialized:
    // py:15  return
    // py:16  initialized = True
    INITIALIZED.get_or_init(|| {
        // py:17  create_ruby_dpowerline()
        create_ruby_dpowerline();
        // py:18  vim.command((
        // py:19  # When using :execute (vim.command uses the same code) one should not
        // py:20  # use << EOF.
        // py:21  '''
        // py:22  ruby
        // py:23  if (not ($command_t.respond_to? 'active_finder'))
        // py:24  def $command_t.active_finder
        // py:25  @active_finder and @active_finder.class.name or ''
        // py:26  end
        // py:27  end
        // py:28  if (not ($command_t.respond_to? 'path'))
        // py:29  def $command_t.path
        // py:30  @path or ''
        // py:31  end
        // py:32  end
        // py:33  def $powerline.commandt_set_active_finder
        // py:34  ::VIM::command "let g:powerline_commandt_reply = '#{$command_t.active_finder}'"
        // py:35  end
        // py:36  def $powerline.commandt_set_path
        // py:37  ::VIM::command "let g:powerline_commandt_reply = '#{($command_t.path or '').gsub(/'/, "''")}'"
        // py:38  end
        // py:39  '''
        // py:40  ))
        true
    });
}

/// Port of `finder()` from
/// `powerline/segments/vim/plugin/commandt.py:42`.
///
/// Display Command-T finder name.
///
/// Highlight groups used: `commandt:finder`.
pub fn finder(_pl: &()) -> Vec<Value> {
    // py:46  def finder(pl):
    // py:47-55  docstring
    // py:56  initialize()
    initialize();
    // py:57  vim.command('ruby $powerline.commandt_set_active_finder')
    // py:58  return [{
    // py:59  'highlight_groups': ['commandt:finder'],
    // py:60  'contents': vim.eval('g:powerline_commandt_reply').replace('CommandT::', '').replace('Finder::', '')
    // py:61  }]
    vec![json!({
        "highlight_groups": ["commandt:finder"],
        "contents": ""
    })]
}

/// Port of module-level binding `FINDERS_WITHOUT_PATH` from
/// `powerline/segments/vim/plugin/commandt.py:58`.
#[allow(non_snake_case)]
pub fn FINDERS_WITHOUT_PATH() -> &'static HashSet<&'static str> {
    // py:64  FINDERS_WITHOUT_PATH = set((
    // py:65  'CommandT::MRUBufferFinder',
    // py:66  'CommandT::BufferFinder',
    // py:67  'CommandT::TagFinder',
    // py:68  'CommandT::JumpFinder',
    // py:69  'CommandT::Finder::MRUBufferFinder',
    // py:70  'CommandT::Finder::BufferFinder',
    // py:71  'CommandT::Finder::TagFinder',
    // py:72  'CommandT::Finder::JumpFinder',
    // py:73  ))
    static S: OnceLock<HashSet<&'static str>> = OnceLock::new();
    S.get_or_init(|| {
        let mut s = HashSet::new();
        s.insert("CommandT::MRUBufferFinder");
        s.insert("CommandT::BufferFinder");
        s.insert("CommandT::TagFinder");
        s.insert("CommandT::JumpFinder");
        s.insert("CommandT::Finder::MRUBufferFinder");
        s.insert("CommandT::Finder::BufferFinder");
        s.insert("CommandT::Finder::TagFinder");
        s.insert("CommandT::Finder::JumpFinder");
        s
    })
}

/// Port of `path()` from
/// `powerline/segments/vim/plugin/commandt.py:69`.
///
/// Display path used by Command-T.
///
/// Highlight groups used: `commandt:path`.
///
/// Returns `None` when the active finder is in `FINDERS_WITHOUT_PATH`
/// (matches py:82 short-circuit). With vim integration the body would
/// then return the active path.
pub fn path(_pl: &()) -> Option<Vec<Value>> {
    // py:76  def path(pl):
    // py:77-87  docstring
    // py:88  initialize()
    initialize();
    // py:89  vim.command('ruby $powerline.commandt_set_active_finder')
    // py:90  finder = vim.eval('g:powerline_commandt_reply')
    let finder = "";
    // py:91  if finder in FINDERS_WITHOUT_PATH:
    // py:92  return None
    if FINDERS_WITHOUT_PATH().contains(finder) {
        return None;
    }
    // py:93  vim.command('ruby $powerline.commandt_set_path')
    // py:94  return [{
    // py:95  'highlight_groups': ['commandt:path'],
    // py:96  'contents': vim.eval('g:powerline_commandt_reply')
    // py:97  }]
    Some(vec![json!({
        "highlight_groups": ["commandt:path"],
        "contents": ""
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finders_without_path_includes_buffer_finder() {
        let s = FINDERS_WITHOUT_PATH();
        assert!(s.contains("CommandT::BufferFinder"));
        assert!(s.contains("CommandT::Finder::TagFinder"));
        assert_eq!(s.len(), 8);
    }

    #[test]
    fn finder_returns_one_segment_with_empty_contents() {
        let r = finder(&());
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn path_returns_some_when_finder_not_excluded() {
        // Stub finder = "" which is NOT in FINDERS_WITHOUT_PATH set.
        let r = path(&());
        assert!(r.is_some());
    }

    #[test]
    fn initialize_is_idempotent() {
        initialize();
        initialize();
        // No panic = pass.
    }
}
