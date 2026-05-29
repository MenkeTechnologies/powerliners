// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/bash.py`.
//!
//! Powerline bash prompt segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                     // py:4

use std::collections::HashMap;

/// Port of `class BashPromptRenderer(ShellRenderer)` from
/// `powerline/renderers/shell/bash.py:7`.
///
/// bash's `\[ ... \]` escape markers (PROMPT_PS1-safe non-display
/// regions) + translations for `$`, backtick, and backslash so the
/// shell doesn't interpret literals as command substitution.
pub struct BashPromptRenderer;

impl BashPromptRenderer {
    /// Port of `BashPromptRenderer.escape_hl_start` from
    /// `powerline/renderers/shell/bash.py:9`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = "\\[";

    /// Port of `BashPromptRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/bash.py:10`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str = "\\]";

    /// Port of `BashPromptRenderer.character_translations` from
    /// `powerline/renderers/shell/bash.py:12-15`.
    ///
    /// Python: extends `ShellRenderer.character_translations` with:
    ///   - `$` → `\$` (suppress command substitution)
    ///   - `\`` → `\\\`` (suppress backtick substitution)
    ///   - `\\` → `\\\\` (escape literal backslash)
    pub fn character_translations() -> HashMap<char, &'static str> {
        let mut t: HashMap<char, &'static str> = HashMap::new();
        t.insert('$', "\\$"); // py:13
        t.insert('`', "\\`"); // py:14
        t.insert('\\', "\\\\"); // py:15
        t
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/bash.py:84`.
#[allow(non_camel_case_types)]
pub type renderer = BashPromptRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_escape_markers_are_prompt_safe() {
        assert_eq!(BashPromptRenderer::escape_hl_start, "\\[");
        assert_eq!(BashPromptRenderer::escape_hl_end, "\\]");
    }

    #[test]
    fn bash_translations_escape_shell_specials() {
        let t = BashPromptRenderer::character_translations();
        assert_eq!(t.get(&'$'), Some(&"\\$"));
        assert_eq!(t.get(&'`'), Some(&"\\`"));
        assert_eq!(t.get(&'\\'), Some(&"\\\\"));
    }
}
