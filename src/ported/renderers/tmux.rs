// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/tmux.py`.
//!
//! tmux status-line renderer. Emits the tmux `#[fg=...,bg=...,attr...]`
//! styling sequences used by tmux's `status-left`/`status-right`
//! format strings. The renderer extends the base `Renderer` (currently
//! unported as `renderer.rs`); the Rust port surfaces the renderer's
//! pure functions (`attrs_to_tmux_attrs`, `hlstyle`) and the
//! character-translation table so it can be composed once the base
//! renderer wires up.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderer import Renderer            // py:4
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE                  // py:5

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};

/// Port of `attrs_to_tmux_attrs()` from
/// `powerline/renderers/tmux.py:8`.
///
/// `attrs == None` → returns `["nobold", "noitalics", "nounderscore"]`.
/// (The Python source's `attrs is False` test is the all-off sentinel;
/// Rust represents it via `None`.) Otherwise, emits one of the
/// matching/no-matching pair per attribute bit.
pub fn attrs_to_tmux_attrs(attrs: Option<u32>) -> Vec<String> {
    // py:8  def attrs_to_tmux_attrs(attrs):
    // py:9  if attrs is False:
    let Some(a) = attrs else {
        // py:10  return ['nobold', 'noitalics', 'nounderscore']
        return vec![
            "nobold".to_string(),
            "noitalics".to_string(),
            "nounderscore".to_string(),
        ];
    };
    // py:11  else:
    // py:12  ret = []
    let mut ret: Vec<String> = Vec::with_capacity(3);
    // py:13  if attrs & ATTR_BOLD:
    if a & ATTR_BOLD != 0 {
        // py:14  ret += ['bold']
        ret.push("bold".to_string());
    } else {
        // py:15  else:
        // py:16  ret += ['nobold']
        ret.push("nobold".to_string());
    }
    // py:17  if attrs & ATTR_ITALIC:
    if a & ATTR_ITALIC != 0 {
        // py:18  ret += ['italics']
        ret.push("italics".to_string());
    } else {
        // py:19  else:
        // py:20  ret += ['noitalics']
        ret.push("noitalics".to_string());
    }
    // py:21  if attrs & ATTR_UNDERLINE:
    if a & ATTR_UNDERLINE != 0 {
        // py:22  ret += ['underscore']
        ret.push("underscore".to_string());
    } else {
        // py:23  else:
        // py:24  ret += ['nounderscore']
        ret.push("nounderscore".to_string());
    }
    // py:25  return ret
    ret
}

/// Color descriptor: `(cterm_index, optional_truecolor_rgb)`.
///
/// Python's renderer passes color as `(cterm, hex_int)` tuples or
/// `False` for the "default" sentinel. The Rust port represents the
/// default sentinel as `None` and the cterm-only fallback via
/// `(cterm, None)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorSpec {
    pub cterm: u16,
    pub truecolor: Option<u32>,
}

/// Port of `class TmuxRenderer(Renderer)` from
/// `powerline/renderers/tmux.py:27`.
pub struct TmuxRenderer {
    /// Whether the terminal supports truecolor — selects between
    /// `fg=#xxxxxx` and `fg=colourN`. Maps to the Python attribute
    /// `self.term_truecolor` inherited from `Renderer`.
    pub term_truecolor: bool,
}

impl TmuxRenderer {
    /// Constructs a `TmuxRenderer` with the given truecolor toggle.
    pub fn new(term_truecolor: bool) -> Self {
        Self { term_truecolor }
    }

    /// Port of `TmuxRenderer.character_translations` from
    /// `powerline/renderers/tmux.py:30-31`.
    ///
    /// tmux uses `#` as its style escape, so `#` literals must be
    /// doubled to `##[]`. The base renderer carries a wider table;
    /// this is the tmux-specific override entry.
    pub fn character_translations() -> Vec<(char, &'static str)> {
        // py:30-31  character_translations[ord('#')] = '##[]'
        vec![('#', "##[]")]
    }

    /// Port of `TmuxRenderer.hlstyle()` from
    /// `powerline/renderers/tmux.py:40`.
    ///
    /// Emits `#[fg=...,bg=...,attr...]`. Returns an empty string if
    /// all three of `attrs`/`bg`/`fg` are absent (no style change
    /// needed).
    pub fn hlstyle(
        &self,
        fg: Option<ColorSpec>,
        bg: Option<ColorSpec>,
        attrs: Option<u32>,
    ) -> String {
        // py:41  def hlstyle(self, fg=None, bg=None, attrs=None, **kwargs):
        // py:42  '''Highlight a segment.'''
        // py:43  # We don't need to explicitly reset attributes, so skip those calls
        // py:44  if not attrs and not bg and not fg:
        if attrs.is_none() && bg.is_none() && fg.is_none() {
            // py:45  return ''
            return String::new();
        }
        // py:46  tmux_attrs = []
        let mut tmux_attrs: Vec<String> = Vec::new();
        // py:47  if fg is not None:
        // py:48  if fg is False or fg[0] is False:
        // py:49  tmux_attrs += ['fg=default']
        // py:50  else:
        // py:51  if self.term_truecolor and fg[1]:
        // py:52  tmux_attrs += ['fg=#{0:06x}'.format(int(fg[1]))]
        // py:53  else:
        // py:54  tmux_attrs += ['fg=colour' + str(fg[0])]
        if let Some(f) = fg {
            tmux_attrs.push(self.color_spec("fg", Some(f)));
        }
        // py:55  if bg is not None:
        // py:56  if bg is False or bg[0] is False:
        // py:57  tmux_attrs += ['bg=default']
        // py:58  else:
        // py:59  if self.term_truecolor and bg[1]:
        // py:60  tmux_attrs += ['bg=#{0:06x}'.format(int(bg[1]))]
        // py:61  else:
        // py:62  tmux_attrs += ['bg=colour' + str(bg[0])]
        if let Some(b) = bg {
            tmux_attrs.push(self.color_spec("bg", Some(b)));
        }
        // py:63  if attrs is not None:
        // py:64  tmux_attrs += attrs_to_tmux_attrs(attrs)
        if let Some(a) = attrs {
            tmux_attrs.extend(attrs_to_tmux_attrs(Some(a)));
        }
        // py:65  return '#[' + ','.join(tmux_attrs) + ']'
        format!("#[{}]", tmux_attrs.join(","))
    }

    /// Port of `TmuxRenderer.render()` from
    /// `powerline/renderers/tmux.py:34-39`.
    ///
    /// Clamps `width` to at least 10 columns after subtracting
    /// `segment_info['width_adjust']` (a per-pane cushion for tmux's
    /// status-right margin), then delegates to the base renderer's
    /// render.
    ///
    /// `base_render` is the super().render() result — closure-injected
    /// since the base `Renderer.render` dispatch isn't reachable from
    /// this typed Rust struct. The closure receives the adjusted width.
    pub fn render<F>(
        &self,
        width: Option<i64>,
        segment_info: &serde_json::Map<String, serde_json::Value>,
        base_render: F,
    ) -> String
    where
        F: FnOnce(Option<i64>) -> String,
    {
        // py:34  def render(self, width=None, segment_info={}, **kwargs):
        // py:35  if width and segment_info:
        let adjusted = if let Some(w) = width {
            if !segment_info.is_empty() {
                // py:36  width -= segment_info.get('width_adjust', 0)
                let adjust = segment_info
                    .get("width_adjust")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let mut new_w = w - adjust;
                // py:37  if width < 10:
                if new_w < 10 {
                    // py:38  width = 10
                    new_w = 10;
                }
                Some(new_w)
            } else {
                Some(w)
            }
        } else {
            None
        };
        // py:39  return super(TmuxRenderer, self).render(width=width, segment_info=segment_info, **kwargs)
        base_render(adjusted)
    }

    /// Port of `TmuxRenderer.get_segment_info()` from
    /// `powerline/renderers/tmux.py:67-78`.
    ///
    /// Builds the per-segment info dict by merging the renderer's
    /// base `segment_info` with the caller-supplied dict, then
    /// injects `getcwd` and `mode` keys per py:71-77. The Python
    /// source resolves `getcwd` to a lambda; the Rust port records
    /// the resolved CWD string (since fn-references don't survive
    /// serde_json::Value round-trips), so callers read `getcwd` as a
    /// string instead of invoking it.
    pub fn get_segment_info(
        &self,
        renderer_segment_info: &serde_json::Map<String, serde_json::Value>,
        segment_info: &serde_json::Map<String, serde_json::Value>,
        mode: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        // py:67  def get_segment_info(self, segment_info, mode):
        // py:68  r = self.segment_info.copy()
        let mut r = renderer_segment_info.clone();
        // py:69  if segment_info:
        // py:70  r.update(segment_info)
        if !segment_info.is_empty() {
            for (k, v) in segment_info {
                r.insert(k.clone(), v.clone());
            }
        }
        // py:71  if 'pane_current_path' in r:
        if let Some(path) = r.get("pane_current_path").cloned() {
            // py:72  r['getcwd'] = lambda: r['pane_current_path']
            r.insert("getcwd".to_string(), path);
        } else if let Some(pid) = r.get("pane_id").cloned() {
            // py:73  elif 'pane_id' in r:
            // py:74  varname = 'TMUX_PWD_' + str(r['pane_id'])
            let pid_str = match pid {
                serde_json::Value::String(s) => s,
                v => v.to_string(),
            };
            let varname = format!("TMUX_PWD_{}", pid_str);
            // py:75  if varname in r['environ']:
            if let Some(env) = r.get("environ").and_then(|v| v.as_object()) {
                if let Some(cwd) = env.get(&varname).cloned() {
                    // py:76  r['getcwd'] = lambda: r['environ'][varname]
                    r.insert("getcwd".to_string(), cwd);
                }
            }
        }
        // py:77  r['mode'] = mode
        r.insert(
            "mode".to_string(),
            serde_json::Value::String(mode.to_string()),
        );
        // py:78  return r
        r
    }

    /// Helper that formats one (channel, ColorSpec) pair as the
    /// appropriate tmux directive. The `false` / `(False, _)` Python
    /// sentinel maps to `None` (use channel default).
    fn color_spec(&self, channel: &str, color: Option<ColorSpec>) -> String {
        let Some(c) = color else {
            return format!("{}=default", channel);
        };
        if self.term_truecolor {
            if let Some(rgb) = c.truecolor {
                return format!("{}=#{:06x}", channel, rgb);
            }
        }
        format!("{}=colour{}", channel, c.cterm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrs_to_tmux_attrs_none_returns_all_no_prefixes() {
        let r = attrs_to_tmux_attrs(None);
        assert_eq!(r, vec!["nobold", "noitalics", "nounderscore"]);
    }

    #[test]
    fn attrs_to_tmux_attrs_zero_returns_all_no_prefixes() {
        let r = attrs_to_tmux_attrs(Some(0));
        assert_eq!(r, vec!["nobold", "noitalics", "nounderscore"]);
    }

    #[test]
    fn attrs_to_tmux_attrs_bold_only() {
        let r = attrs_to_tmux_attrs(Some(ATTR_BOLD));
        assert_eq!(r, vec!["bold", "noitalics", "nounderscore"]);
    }

    #[test]
    fn attrs_to_tmux_attrs_italic_only() {
        let r = attrs_to_tmux_attrs(Some(ATTR_ITALIC));
        assert_eq!(r, vec!["nobold", "italics", "nounderscore"]);
    }

    #[test]
    fn attrs_to_tmux_attrs_underline_only() {
        let r = attrs_to_tmux_attrs(Some(ATTR_UNDERLINE));
        assert_eq!(r, vec!["nobold", "noitalics", "underscore"]);
    }

    #[test]
    fn attrs_to_tmux_attrs_all_three() {
        let r = attrs_to_tmux_attrs(Some(ATTR_BOLD | ATTR_ITALIC | ATTR_UNDERLINE));
        assert_eq!(r, vec!["bold", "italics", "underscore"]);
    }

    #[test]
    fn character_translations_contains_hash_override() {
        let t = TmuxRenderer::character_translations();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0], ('#', "##[]"));
    }

    #[test]
    fn hlstyle_no_args_returns_empty_string() {
        let r = TmuxRenderer::new(false);
        assert_eq!(r.hlstyle(None, None, None), "");
    }

    #[test]
    fn hlstyle_fg_cterm_only_emits_colour_n() {
        let r = TmuxRenderer::new(false);
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffffff),
            }),
            None,
            None,
        );
        // term_truecolor=false → cterm fallback
        assert_eq!(s, "#[fg=colour231]");
    }

    #[test]
    fn hlstyle_fg_truecolor_emits_hex() {
        let r = TmuxRenderer::new(true);
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffaa00),
            }),
            None,
            None,
        );
        assert_eq!(s, "#[fg=#ffaa00]");
    }

    #[test]
    fn hlstyle_bg_emits_bg_directive() {
        let r = TmuxRenderer::new(false);
        let s = r.hlstyle(
            None,
            Some(ColorSpec {
                cterm: 21,
                truecolor: Some(0x0000ff),
            }),
            None,
        );
        assert_eq!(s, "#[bg=colour21]");
    }

    #[test]
    fn hlstyle_attrs_emits_bold_italics_underline() {
        let r = TmuxRenderer::new(false);
        let s = r.hlstyle(None, None, Some(ATTR_BOLD | ATTR_UNDERLINE));
        assert_eq!(s, "#[bold,noitalics,underscore]");
    }

    #[test]
    fn hlstyle_combined_fg_bg_attrs_joins_with_commas() {
        let r = TmuxRenderer::new(false);
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 231,
                truecolor: None,
            }),
            Some(ColorSpec {
                cterm: 21,
                truecolor: None,
            }),
            Some(ATTR_BOLD),
        );
        assert_eq!(s, "#[fg=colour231,bg=colour21,bold,noitalics,nounderscore]");
    }

    #[test]
    fn hlstyle_truecolor_without_rgb_falls_back_to_cterm() {
        let r = TmuxRenderer::new(true);
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 42,
                truecolor: None,
            }),
            None,
            None,
        );
        // No truecolor provided → cterm path even with term_truecolor=true
        assert_eq!(s, "#[fg=colour42]");
    }

    #[test]
    fn render_subtracts_width_adjust_from_supplied_width() {
        // py:36  width -= segment_info.get('width_adjust', 0)
        let r = TmuxRenderer::new(false);
        let mut info = serde_json::Map::new();
        info.insert(
            "width_adjust".to_string(),
            serde_json::Value::Number(serde_json::Number::from(20)),
        );
        let captured = r.render(Some(120), &info, |w| format!("w={:?}", w));
        assert_eq!(captured, "w=Some(100)");
    }

    #[test]
    fn render_clamps_width_to_minimum_10() {
        // py:37-38  if width < 10: width = 10
        let r = TmuxRenderer::new(false);
        let mut info = serde_json::Map::new();
        info.insert(
            "width_adjust".to_string(),
            serde_json::Value::Number(serde_json::Number::from(100)),
        );
        // 50 - 100 = -50 → clamp to 10
        let captured = r.render(Some(50), &info, |w| format!("w={:?}", w));
        assert_eq!(captured, "w=Some(10)");
    }

    #[test]
    fn render_passes_through_width_when_segment_info_empty() {
        // py:35  if width and segment_info:  empty → skip adjust
        let r = TmuxRenderer::new(false);
        let info = serde_json::Map::new();
        let captured = r.render(Some(80), &info, |w| format!("w={:?}", w));
        assert_eq!(captured, "w=Some(80)");
    }

    #[test]
    fn get_segment_info_inserts_mode_key() {
        // py:77  r['mode'] = mode
        let r = TmuxRenderer::new(false);
        let base = serde_json::Map::new();
        let info = serde_json::Map::new();
        let out = r.get_segment_info(&base, &info, "right");
        assert_eq!(
            out.get("mode"),
            Some(&serde_json::Value::String("right".to_string()))
        );
    }

    #[test]
    fn get_segment_info_uses_pane_current_path_for_getcwd() {
        // py:71-72  pane_current_path → getcwd alias
        let r = TmuxRenderer::new(false);
        let mut info = serde_json::Map::new();
        info.insert(
            "pane_current_path".to_string(),
            serde_json::Value::String("/tmp/zz".to_string()),
        );
        let out = r.get_segment_info(&serde_json::Map::new(), &info, "left");
        assert_eq!(
            out.get("getcwd"),
            Some(&serde_json::Value::String("/tmp/zz".to_string()))
        );
    }

    #[test]
    fn get_segment_info_falls_back_to_tmux_pwd_var() {
        // py:73-76  fall back to TMUX_PWD_<pane_id> environ lookup
        let r = TmuxRenderer::new(false);
        let mut info = serde_json::Map::new();
        info.insert(
            "pane_id".to_string(),
            serde_json::Value::String("%5".to_string()),
        );
        let mut env = serde_json::Map::new();
        env.insert(
            "TMUX_PWD_%5".to_string(),
            serde_json::Value::String("/srv/work".to_string()),
        );
        info.insert("environ".to_string(), serde_json::Value::Object(env));
        let out = r.get_segment_info(&serde_json::Map::new(), &info, "left");
        assert_eq!(
            out.get("getcwd"),
            Some(&serde_json::Value::String("/srv/work".to_string()))
        );
    }
}
