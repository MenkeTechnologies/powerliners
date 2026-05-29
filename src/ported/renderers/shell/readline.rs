// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/readline.py`.
//!
//! Renderer useful for some applications that use readline.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                     // py:4

use std::collections::HashMap;

/// Port of `class ReadlineRenderer(ShellRenderer)` from
/// `powerline/renderers/shell/readline.py:7`.
///
/// Carries the escape-highlight start/end markers + (empty) per-char
/// translation table. The actual render dispatch lives in the
/// (unported) ShellRenderer base; this struct exposes the
/// readline-specific overrides as data.
pub struct ReadlineRenderer;

impl ReadlineRenderer {
    /// Port of `ReadlineRenderer.escape_hl_start` from
    /// `powerline/renderers/shell/readline.py:10`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str = "\x01";

    /// Port of `ReadlineRenderer.escape_hl_end` from
    /// `powerline/renderers/shell/readline.py:11`.
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str = "\x02";

    /// Readline-specific per-character translations. Empty (inherits
    /// `ShellRenderer.character_translations` which is empty until the
    /// base class is ported).
    pub fn character_translations() -> HashMap<char, &'static str> {
        HashMap::new()
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/readline.py:14`.
///
/// Python: `renderer = ReadlineRenderer` (class alias).
#[allow(non_camel_case_types)]
pub type renderer = ReadlineRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_markers_match_upstream_readline() {
        // py:10  '\x01' (SOH — readline's non-display marker start)
        // py:11  '\x02' (STX — readline's non-display marker end)
        assert_eq!(ReadlineRenderer::escape_hl_start, "\x01");
        assert_eq!(ReadlineRenderer::escape_hl_end, "\x02");
    }
}
