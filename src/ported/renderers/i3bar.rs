// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/i3bar.py`.
//!
//! I3bar Segment Renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import json                                      // py:4
// from powerline.renderer import Renderer          // py:6

use serde_json::{Map, Value};

/// Port of `class I3barRenderer(Renderer)` from
/// `powerline/renderers/i3bar.py:9`.
///
/// I3bar Segment Renderer.
///
/// Currently works only for i3bgbar (i3 bar with custom patches).
pub struct I3barRenderer;

impl I3barRenderer {
    /// Port of `I3barRenderer.hlstyle()` from
    /// `powerline/renderers/i3bar.py:15`.
    ///
    /// We don't need to explicitly reset attributes, so skip those calls.
    pub fn hlstyle() -> &'static str {
        // py:15  @staticmethod
        // py:16  def hlstyle(*args, **kwargs):
        // py:17  # We don't need to explicitly reset attributes, so skip those calls
        // py:18  return ''
        ""
    }

    /// Port of `I3barRenderer.hl()` from
    /// `powerline/renderers/i3bar.py:20`.
    ///
    /// Builds the i3bar protocol JSON segment for one highlighted run
    /// of contents. Returns `json.dumps(segment) + ','` per upstream
    /// py:33 — the trailing comma is part of i3bar's array-of-objects
    /// streaming format.
    pub fn hl(contents: &str, fg: Option<(i32, i64)>, bg: Option<(i32, i64)>) -> String {
        // py:20  def hl(self, contents, fg=None, bg=None, attrs=None, **kwargs):
        // py:21  segment = {
        // py:22  'full_text': contents,
        // py:23  'separator': False,
        // py:24  'separator_block_width': 0,  # no separators
        // py:25  }
        let mut segment = Map::new();
        segment.insert("full_text".to_string(), Value::String(contents.to_string()));
        segment.insert("separator".to_string(), Value::Bool(false));
        segment.insert("separator_block_width".to_string(), Value::from(0));

        // py:27  if fg is not None:
        if let Some((_, hex)) = fg {
            // py:28  if fg is not False and fg[1] is not False:
            if hex >= 0 {
                // py:29  segment['color'] = '#{0:06x}'.format(fg[1])
                segment.insert("color".to_string(), Value::String(format!("#{:06x}", hex)));
            }
        }
        // py:30  if bg is not None:
        if let Some((_, hex)) = bg {
            // py:31  if bg is not False and bg[1] is not False:
            if hex >= 0 {
                // py:32  segment['background'] = '#{0:06x}'.format(bg[1])
                segment.insert(
                    "background".to_string(),
                    Value::String(format!("#{:06x}", hex)),
                );
            }
        }
        // py:33  return json.dumps(segment) + ','
        format!("{},", Value::Object(segment))
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/i3bar.py:36`.
#[allow(non_camel_case_types)]
pub type renderer = I3barRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlstyle_returns_empty() {
        assert_eq!(I3barRenderer::hlstyle(), "");
    }

    #[test]
    fn hl_basic_contents_only() {
        let out = I3barRenderer::hl("hello", None, None);
        // Should end with comma per i3bar streaming format
        assert!(out.ends_with(','));
        // Strip trailing comma and parse JSON
        let json_part = &out[..out.len() - 1];
        let v: Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(v["full_text"], "hello");
        assert_eq!(v["separator"], false);
        assert_eq!(v["separator_block_width"], 0);
        assert!(v.get("color").is_none());
        assert!(v.get("background").is_none());
    }

    #[test]
    fn hl_with_fg_emits_color_field() {
        let out = I3barRenderer::hl("x", Some((231, 0xffffff)), None);
        let json_part = &out[..out.len() - 1];
        let v: Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(v["color"], "#ffffff");
    }

    #[test]
    fn hl_with_bg_emits_background_field() {
        let out = I3barRenderer::hl("x", None, Some((21, 0x0000ff)));
        let json_part = &out[..out.len() - 1];
        let v: Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(v["background"], "#0000ff");
    }
}
