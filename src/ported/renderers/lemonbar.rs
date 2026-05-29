// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/lemonbar.py`.
//!
//! lemonbar (formerly bar/bar ain't recursive) renderer.
//!
//! See documentation of [lemonbar](https://github.com/LemonBoy/bar)
//! and the upstream usage instructions.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderer import Renderer          // py:4
// from powerline.theme import Theme                // py:5
// from powerline.colorscheme import ATTR_UNDERLINE // py:6

use crate::ported::colorscheme::ATTR_UNDERLINE;
use std::collections::HashMap;

/// Port of `class LemonbarRenderer(Renderer)` from
/// `powerline/renderers/lemonbar.py:9`.
pub struct LemonbarRenderer;

impl LemonbarRenderer {
    /// Port of `LemonbarRenderer.character_translations` from
    /// `powerline/renderers/lemonbar.py:16-17`.
    ///
    /// Python: extends `Renderer.character_translations` with
    /// `'%' → '%%{}'` — lemonbar's escape for literal `%`.
    pub fn character_translations() -> HashMap<char, &'static str> {
        let mut t: HashMap<char, &'static str> = HashMap::new();
        // py:17  character_translations[ord('%')] = '%%{}'
        t.insert('%', "%%{}");
        t
    }

    /// Port of `LemonbarRenderer.hlstyle()` from
    /// `powerline/renderers/lemonbar.py:19`.
    pub fn hlstyle() -> &'static str {
        // py:22  return ''
        ""
    }

    /// Port of `LemonbarRenderer.hl()` from
    /// `powerline/renderers/lemonbar.py:24`.
    ///
    /// Wraps `contents` in lemonbar formatting codes:
    /// `%{F#ffXXXXXX}` for foreground, `%{B#ffXXXXXX}` for background,
    /// `%{+u}` for underline, terminating with `%{F-B--u}`.
    pub fn hl(
        contents: &str,
        fg: Option<(i32, i64)>,
        bg: Option<(i32, i64)>,
        attrs: u32,
    ) -> String {
        // py:25  text = ''
        let mut text = String::new();
        // py:27-29  fg dispatch
        if let Some((_, hex)) = fg {
            if hex >= 0 {
                text.push_str(&format!("%{{F#ff{:06x}}}", hex));
            }
        }
        // py:30-32  bg dispatch
        if let Some((_, hex)) = bg {
            if hex >= 0 {
                text.push_str(&format!("%{{B#ff{:06x}}}", hex));
            }
        }
        // py:34-35  underline attr
        if attrs & ATTR_UNDERLINE != 0 {
            text.push_str("%{+u}");
        }
        // py:37  return text + contents + '%{F-B--u}'
        format!("{}{}%{{F-B--u}}", text, contents)
    }

    /// Port of `LemonbarRenderer.render()` from
    /// `powerline/renderers/lemonbar.py:39`.
    ///
    /// Wraps the left+right halves of the statusline in
    /// `%{l}<left>%{r}<right>` lemonbar position markers.
    pub fn render(left_half: &str, right_half: &str) -> String {
        // py:40-43  '%{{l}}{left}%{{r}}{right}'.format(...)
        format!("%{{l}}{}%{{r}}{}", left_half, right_half)
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/lemonbar.py:60`.
#[allow(non_camel_case_types)]
pub type renderer = LemonbarRenderer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ported::colorscheme::ATTR_UNDERLINE;

    #[test]
    fn character_translations_escapes_percent() {
        let t = LemonbarRenderer::character_translations();
        assert_eq!(t.get(&'%'), Some(&"%%{}"));
    }

    #[test]
    fn hl_plain_wraps_with_reset_marker() {
        let out = LemonbarRenderer::hl("hi", None, None, 0);
        assert_eq!(out, "hi%{F-B--u}");
    }

    #[test]
    fn hl_with_fg_emits_F_marker() {
        let out = LemonbarRenderer::hl("hi", Some((231, 0xffffff)), None, 0);
        assert!(out.contains("%{F#ffffffff}"));
    }

    #[test]
    fn hl_with_bg_emits_B_marker() {
        let out = LemonbarRenderer::hl("hi", None, Some((21, 0x0000ff)), 0);
        assert!(out.contains("%{B#ff0000ff}"));
    }

    #[test]
    fn hl_with_underline_emits_u_marker() {
        let out = LemonbarRenderer::hl("hi", None, None, ATTR_UNDERLINE);
        assert!(out.contains("%{+u}"));
    }

    #[test]
    fn hl_with_all_three_emits_all_markers_in_order() {
        let out = LemonbarRenderer::hl(
            "x",
            Some((231, 0xffffff)),
            Some((21, 0x0000ff)),
            ATTR_UNDERLINE,
        );
        let f_pos = out.find("%{F#ff").unwrap();
        let b_pos = out.find("%{B#ff").unwrap();
        let u_pos = out.find("%{+u}").unwrap();
        assert!(f_pos < b_pos);
        assert!(b_pos < u_pos);
    }

    #[test]
    fn render_wraps_left_and_right_with_position_markers() {
        let out = LemonbarRenderer::render("LEFT", "RIGHT");
        assert_eq!(out, "%{l}LEFT%{r}RIGHT");
    }

    #[test]
    fn hlstyle_returns_empty() {
        assert_eq!(LemonbarRenderer::hlstyle(), "");
    }
}
