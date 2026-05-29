// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/ksh.py`.
//!
//! Powerline ksh prompt segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                     // py:4

use std::collections::HashMap;

/// Port of module-level binding `ESCAPE_CHAR` from
/// `powerline/renderers/shell/ksh.py:7`.
#[allow(non_upper_case_globals)]
pub const ESCAPE_CHAR: &str = "\x01";                // py:7

/// Port of `class KshPromptRenderer(ShellRenderer)` from
/// `powerline/renderers/shell/ksh.py:10`.
///
/// ksh's prompt escape: use `\001` as both start and end marker, and
/// prepend `\001\r` to every render so ksh's prompt-line-counter
/// doesn't confuse the highlighted region with visible width.
pub struct KshPromptRenderer;

impl KshPromptRenderer {
    /// Port of `KshPromptRenderer.escape_hl_start` from
    /// `powerline/renderers/shell/ksh.py:12`.
    pub const escape_hl_start: &'static str = "\x01";

    /// Port of `KshPromptRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/ksh.py:13`.
    pub const escape_hl_end: &'static str = "\x01";

    /// Prepend wrapper for ksh's prompt-rendering quirk.
    ///
    /// Port of `KshPromptRenderer.render()` from
    /// `powerline/renderers/shell/ksh.py:15`. Python:
    /// ```python
    /// def render(self, *args, **kwargs):
    ///     return '\001\r' + super().render(*args, **kwargs)
    /// ```
    pub fn render_prefix() -> &'static str {
        // py:16  '\001\r' + super().render(...)
        "\x01\r"
    }

    /// Inherits empty character_translations from ShellRenderer.
    pub fn character_translations() -> HashMap<char, &'static str> {
        HashMap::new()
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/ksh.py:19`.
#[allow(non_camel_case_types)]
pub type renderer = KshPromptRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ksh_uses_soh_for_both_markers() {
        assert_eq!(KshPromptRenderer::escape_hl_start, "\x01");
        assert_eq!(KshPromptRenderer::escape_hl_end, "\x01");
        assert_eq!(ESCAPE_CHAR, "\x01");
    }

    #[test]
    fn ksh_render_prefix_carries_soh_cr() {
        assert_eq!(KshPromptRenderer::render_prefix(), "\x01\r");
    }
}
