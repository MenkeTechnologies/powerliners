// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/pango_markup.py`.
//!
//! Powerline Pango markup segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from xml.sax.saxutils import escape as _escape   // py:4
// from powerline.renderer import Renderer          // py:6
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE                // py:7

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};

/// Port of `class PangoMarkupRenderer(Renderer)` from
/// `powerline/renderers/pango_markup.py:10`.
///
/// Powerline Pango markup segment renderer.
pub struct PangoMarkupRenderer;

impl PangoMarkupRenderer {
    /// Port of `PangoMarkupRenderer.hlstyle()` from
    /// `powerline/renderers/pango_markup.py:13`.
    pub fn hlstyle() -> &'static str {
        // py:16  return ''
        ""
    }

    /// Port of `PangoMarkupRenderer.hl()` from
    /// `powerline/renderers/pango_markup.py:18`.
    ///
    /// Highlight a segment.
    ///
    /// Wraps `contents` in `<span ...>contents</span>` with attribute
    /// list built from fg/bg/attrs.
    pub fn hl(
        contents: &str,
        fg: Option<(i32, i64)>,
        bg: Option<(i32, i64)>,
        attrs: Option<u32>,
    ) -> String {
        // py:20  awesome_attr = []
        let mut awesome_attr: Vec<String> = Vec::new();
        // py:21-23  fg
        if let Some((_, hex)) = fg {
            if hex >= 0 {
                awesome_attr.push(format!("foreground=\"#{:06x}\"", hex));
            }
        }
        // py:24-26  bg
        if let Some((_, hex)) = bg {
            if hex >= 0 {
                awesome_attr.push(format!("background=\"#{:06x}\"", hex));
            }
        }
        // py:27-33  attrs
        if let Some(attrs) = attrs {
            if attrs & ATTR_BOLD != 0 {
                awesome_attr.push("font_weight=\"bold\"".to_string());
            }
            if attrs & ATTR_ITALIC != 0 {
                awesome_attr.push("font_style=\"italic\"".to_string());
            }
            if attrs & ATTR_UNDERLINE != 0 {
                awesome_attr.push("underline=\"single\"".to_string());
            }
        }
        // py:34  '<span ' + ' '.join(awesome_attr) + '>' + contents + '</span>'
        format!("<span {}>{}</span>", awesome_attr.join(" "), contents)
    }

    /// Port of `PangoMarkupRenderer.escape` (staticmethod from xml.sax.saxutils)
    /// from `powerline/renderers/pango_markup.py:36`.
    ///
    /// XML-escape `&`, `<`, `>` in `contents`.
    pub fn escape(contents: &str) -> String {
        // py:36  staticmethod(_escape)  where _escape = xml.sax.saxutils.escape
        // Python's xml.sax.saxutils.escape replaces &, <, > by default.
        contents
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/pango_markup.py:39`.
#[allow(non_camel_case_types)]
pub type renderer = PangoMarkupRenderer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};

    #[test]
    fn hl_no_attrs_returns_empty_span() {
        let out = PangoMarkupRenderer::hl("hi", None, None, None);
        assert_eq!(out, "<span >hi</span>");
    }

    #[test]
    fn hl_with_fg_includes_foreground_attribute() {
        let out = PangoMarkupRenderer::hl("hi", Some((231, 0xffffff)), None, None);
        assert!(out.contains("foreground=\"#ffffff\""));
        assert!(out.contains(">hi</span>"));
    }

    #[test]
    fn hl_with_bg_includes_background_attribute() {
        let out = PangoMarkupRenderer::hl("hi", None, Some((21, 0x0000ff)), None);
        assert!(out.contains("background=\"#0000ff\""));
    }

    #[test]
    fn hl_with_bold_attr_includes_font_weight() {
        let out = PangoMarkupRenderer::hl("hi", None, None, Some(ATTR_BOLD));
        assert!(out.contains("font_weight=\"bold\""));
    }

    #[test]
    fn hl_with_italic_attr_includes_font_style() {
        let out = PangoMarkupRenderer::hl("hi", None, None, Some(ATTR_ITALIC));
        assert!(out.contains("font_style=\"italic\""));
    }

    #[test]
    fn hl_with_underline_attr_includes_underline() {
        let out = PangoMarkupRenderer::hl("hi", None, None, Some(ATTR_UNDERLINE));
        assert!(out.contains("underline=\"single\""));
    }

    #[test]
    fn hl_with_combined_attrs() {
        let out = PangoMarkupRenderer::hl(
            "hi",
            Some((231, 0xffffff)),
            Some((21, 0x0000ff)),
            Some(ATTR_BOLD | ATTR_ITALIC),
        );
        assert!(out.contains("foreground=\"#ffffff\""));
        assert!(out.contains("background=\"#0000ff\""));
        assert!(out.contains("font_weight=\"bold\""));
        assert!(out.contains("font_style=\"italic\""));
    }

    #[test]
    fn escape_handles_xml_specials() {
        assert_eq!(PangoMarkupRenderer::escape("<a & b>"), "&lt;a &amp; b&gt;");
        assert_eq!(PangoMarkupRenderer::escape("plain"), "plain");
    }

    #[test]
    fn hlstyle_returns_empty_string() {
        assert_eq!(PangoMarkupRenderer::hlstyle(), "");
    }
}
