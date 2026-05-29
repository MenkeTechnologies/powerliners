// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/rcsh.py`.
//!
//! Upstream source in full:
//!
//! ```python
//! # vim:fileencoding=utf-8:noet
//! from __future__ import (unicode_literals, division, absolute_import, print_function)
//!
//! from powerline.renderers.shell.readline import ReadlineRenderer
//!
//! renderer = ReadlineRenderer
//! ```
//!
//! `rc` shell uses the same escape conventions as readline. Port is a
//! thin alias pointing at `ReadlineRenderer`.

// py:4  from powerline.renderers.shell.readline import ReadlineRenderer
pub use crate::ported::renderers::shell::readline::ReadlineRenderer;

/// Port of module-level binding `renderer` from
/// `powerline/renderers/shell/rcsh.py:7`.
#[allow(non_camel_case_types)]
pub type renderer = ReadlineRenderer; // py:7

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rcsh_aliases_readline_renderer() {
        // Verify the renderer type alias resolves to ReadlineRenderer
        assert_eq!(renderer::escape_hl_start, "\x01");
        assert_eq!(renderer::escape_hl_end, "\x02");
    }
}
