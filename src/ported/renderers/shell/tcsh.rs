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
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = ZshPromptRenderer::escape_hl_start;
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str = ZshPromptRenderer::escape_hl_end;

    /// Port of `TcshPromptRenderer.character_translations` from
    /// `powerline/renderers/shell/tcsh.py:9-13`.
    ///
    /// Extends zsh's `%` → `%%` with `\` → `\\\\`, `^` → `\\^`,
    /// `!` → `\\!`.
    pub fn character_translations() -> HashMap<char, &'static str> {
        // py:9  character_translations = ZshPromptRenderer.character_translations.copy()
        let mut t = ZshPromptRenderer::character_translations();
        // py:10  character_translations[ord('%')] = '%%'
        t.insert('%', "%%");
        // py:11  character_translations[ord('\\')] = '\\\\'
        t.insert('\\', "\\\\");
        // py:12  character_translations[ord('^')] = '\\^'
        t.insert('^', "\\^");
        // py:13  character_translations[ord('!')] = '\\!'
        t.insert('!', "\\!");
        t
    }

    /// Port of `TcshPromptRenderer.do_render()` from
    /// `powerline/renderers/shell/tcsh.py:15`.
    ///
    /// Works around tcsh's prompt-trailing-`%{%}`-strip behaviour by
    /// swapping a trailing `nbsp + end` into `end + nbsp`, or adding
    /// nbsp if not already present.
    pub fn do_render(rendered: &str, hlstyle_end: &str, nbsp: &str) -> String {
        // py:15  def do_render(self, **kwargs):
        // py:16  ret = super(TcshPromptRenderer, self).do_render(**kwargs)
        let ret = rendered.to_string();
        // py:17  nbsp = self.character_translations.get(ord(' '), ' ')
        // py:18  end = self.hlstyle()
        let end = hlstyle_end;
        // py:19  assert not ret or ret.endswith(end)
        // py:20  if ret.endswith(nbsp + end):
        let trailing = format!("{}{}", nbsp, end);
        if ret.ends_with(&trailing) {
            // py:21  # Exchange nbsp and highlight end because tcsh removes trailing
            // py:22  # %{%} part of the prompt for whatever reason
            // py:23  ret = ret[:-(len(nbsp) + len(end))] + end + nbsp
            let trim_len = nbsp.len() + end.len();
            format!("{}{}{}", &ret[..ret.len() - trim_len], end, nbsp)
        } else {
            // py:24  else:
            // py:25  # We *must* end prompt with non-%{%} sequence for the reasons
            // py:26  # explained above. So add nbsp if it is not already there.
            // py:27  ret += nbsp
            format!("{}{}", ret, nbsp)
            // py:28  return ret
        }
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
