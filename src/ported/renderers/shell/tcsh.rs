// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/tcsh.py`.
//!
//! Powerline tcsh prompt segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell.zsh import ZshPromptRenderer                              // py:4

use crate::ported::renderers::shell::zsh::ZshPromptRenderer;
use std::collections::HashMap;

/// Port of `class TcshPromptRenderer(ZshPromptRenderer)` from
/// `powerline/renderers/shell/tcsh.py:7`.
///
/// Extends `ZshPromptRenderer` with tcsh-specific character
/// translations (`%`, `\`, `^`, `!`) and a render-time nbsp/end-marker
/// swap to work around tcsh's prompt-trailing-`%{%}`-strip bug.
pub struct TcshPromptRenderer;

impl TcshPromptRenderer {
    /// Inherits `escape_hl_start`/`escape_hl_end` from
    /// `ZshPromptRenderer` (`%{` / `%}`).
    pub const escape_hl_start: &'static str = ZshPromptRenderer::escape_hl_start;
    pub const escape_hl_end: &'static str = ZshPromptRenderer::escape_hl_end;

    /// Port of `TcshPromptRenderer.character_translations` from
    /// `powerline/renderers/shell/tcsh.py:9-13`.
    ///
    /// Extends zsh's `%` → `%%` with `\` → `\\\\`, `^` → `\\^`,
    /// `!` → `\\!`.
    pub fn character_translations() -> HashMap<char, &'static str> {
        // py:9  ZshPromptRenderer.character_translations.copy()
        let mut t = ZshPromptRenderer::character_translations();
        // py:10  character_translations[ord('%')] = '%%' (already inherited)
        t.insert('%', "%%");
        // py:11  character_translations[ord('\\')] = '\\\\'
        t.insert('\\', "\\\\");
        // py:12  character_translations[ord('^')] = '\\^'
        t.insert('^', "\\^");
        // py:13  character_translations[ord('!')] = '\\!'
        t.insert('!', "\\!");
        t
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/tcsh.py:30`.
#[allow(non_camel_case_types)]
pub type renderer = TcshPromptRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcsh_inherits_zsh_escape_markers() {
        assert_eq!(TcshPromptRenderer::escape_hl_start, "%{");
        assert_eq!(TcshPromptRenderer::escape_hl_end, "%}");
    }

    #[test]
    fn tcsh_translations_include_caret_bang_backslash() {
        let t = TcshPromptRenderer::character_translations();
        assert_eq!(t.get(&'%'), Some(&"%%"));
        assert_eq!(t.get(&'\\'), Some(&"\\\\"));
        assert_eq!(t.get(&'^'), Some(&"\\^"));
        assert_eq!(t.get(&'!'), Some(&"\\!"));
    }
}
