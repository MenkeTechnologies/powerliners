// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/zsh.py`.
//!
//! Powerline zsh prompt segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                     // py:4

use std::collections::HashMap;

/// Port of `class ZshPromptRenderer(ShellRenderer)` from
/// `powerline/renderers/shell/zsh.py:7`.
///
/// zsh's `%{ ... %}` escape markers (PROMPT_SUBST-safe non-display
/// regions) + the `%` → `%%` translation so literal `%` survives
/// zsh's prompt expansion.
pub struct ZshPromptRenderer;

impl ZshPromptRenderer {
    /// Port of `ZshPromptRenderer.escape_hl_start` from
    /// `powerline/renderers/shell/zsh.py:9`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = "%{";

    /// Port of `ZshPromptRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/zsh.py:10`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str = "%}";

    /// Port of `ZshPromptRenderer.character_translations` from
    /// `powerline/renderers/shell/zsh.py:12-13`.
    ///
    /// Python: extends `ShellRenderer.character_translations` with
    /// `'%' → '%%'`. Until the base table is ported the Rust map is
    /// the diff-only view.
    pub fn character_translations() -> HashMap<char, &'static str> {
        // py:12  ShellRenderer.character_translations.copy()
        let mut t: HashMap<char, &'static str> = HashMap::new();
        // py:13  character_translations[ord('%')] = '%%'
        t.insert('%', "%%");
        t
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/zsh.py:16`.
#[allow(non_camel_case_types)]
pub type renderer = ZshPromptRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zsh_escape_markers_use_prompt_subst_safe_form() {
        assert_eq!(ZshPromptRenderer::escape_hl_start, "%{");
        assert_eq!(ZshPromptRenderer::escape_hl_end, "%}");
    }

    #[test]
    fn percent_is_doubled_in_translations() {
        let t = ZshPromptRenderer::character_translations();
        assert_eq!(t.get(&'%'), Some(&"%%"));
    }
}
