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
        // py:13  @staticmethod
        // py:14  def hlstyle(*args, **kwargs):
        // py:15  # We don't need to explicitly reset attributes, so skip those calls
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
        // py:18  def hl(self, contents, fg=None, bg=None, attrs=None, **kwargs):
        // py:19  '''Highlight a segment.'''
        // py:20  awesome_attr = []
        let mut awesome_attr: Vec<String> = Vec::new();
        // py:21  if fg is not None:
        if let Some((_, hex)) = fg {
            // py:22  if fg is not False and fg[1] is not False:
            if hex >= 0 {
                // py:23  awesome_attr += ['foreground="#{0:06x}"'.format(fg[1])]
                awesome_attr.push(format!("foreground=\"#{:06x}\"", hex));
            }
        }
        // py:24  if bg is not None:
        if let Some((_, hex)) = bg {
            // py:25  if bg is not False and bg[1] is not False:
            if hex >= 0 {
                // py:26  awesome_attr += ['background="#{0:06x}"'.format(bg[1])]
                awesome_attr.push(format!("background=\"#{:06x}\"", hex));
            }
        }
        // py:27  if attrs is not None and attrs is not False:
        if let Some(attrs) = attrs {
            // py:28  if attrs & ATTR_BOLD:
            if attrs & ATTR_BOLD != 0 {
                // py:29  awesome_attr += ['font_weight="bold"']
                awesome_attr.push("font_weight=\"bold\"".to_string());
            }
            // py:30  if attrs & ATTR_ITALIC:
            if attrs & ATTR_ITALIC != 0 {
                // py:31  awesome_attr += ['font_style="italic"']
                awesome_attr.push("font_style=\"italic\"".to_string());
            }
            // py:32  if attrs & ATTR_UNDERLINE:
            if attrs & ATTR_UNDERLINE != 0 {
                // py:33  awesome_attr += ['underline="single"']
                awesome_attr.push("underline=\"single\"".to_string());
            }
        }
        // py:34  return '<span ' + ' '.join(awesome_attr) + '>' + contents + '</span>'
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
