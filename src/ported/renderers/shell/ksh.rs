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
pub const ESCAPE_CHAR: &str = "\x01"; // py:7

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
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = "\x01";

    /// Port of `KshPromptRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/ksh.py:13`.
    #[allow(non_upper_case_globals)]
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
        // py:15  def render(self, *args, **kwargs):
        // py:16  return '\001\r' + super(KshPromptRenderer, self).render(*args, **kwargs)
        "\x01\r"
    }

    /// Port of `KshPromptRenderer.render()` from
    /// `powerline/renderers/shell/ksh.py:15-16`.
    ///
    /// Prepends `\001\r` to the base renderer's output. Python's
    /// super().render dispatch is closure-injected here since the
    /// base ShellRenderer's render isn't reachable through a typed
    /// Rust struct.
    pub fn render<F>(super_render: F) -> String
    where
        F: FnOnce() -> String,
    {
        // py:15  def render(self, *args, **kwargs):
        // py:16  return '\001\r' + super().render(*args, **kwargs)
        format!("{}{}", Self::render_prefix(), super_render())
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

    #[test]
    fn ksh_render_prepends_soh_cr_to_super_output() {
        // py:16  '\001\r' + super().render(...)
        let result = KshPromptRenderer::render(|| "PROMPT".to_string());
        assert_eq!(result, "\x01\rPROMPT");
    }

    #[test]
    fn ksh_render_empty_super_yields_just_prefix() {
        let result = KshPromptRenderer::render(String::new);
        assert_eq!(result, "\x01\r");
    }
}
