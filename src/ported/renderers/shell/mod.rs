// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/__init__.py`.
//!
//! Shell-renderer base. Defines:
//!   - `int_to_rgb(num)` — split a 24-bit RGB int into (r, g, b)
//!   - `PromptRenderer` — base for prompt segment renderers; tracks
//!     per-client widths for daemon-mode cache hits
//!   - `ShellRenderer` — ANSI-escape-based shell renderer with
//!     truecolor / fbterm / tmux / screen support
//!
//! Rust port surfaces:
//!   - `int_to_rgb` helper
//!   - `TermEscapeStyle` enum (Auto / Fbterm / Xterm)
//!   - `ShellRenderer` struct with the configuration fields the
//!     Python class attributes hold (escape_hl_start/end,
//!     term_truecolor, term_escape_style, tmux_escape, screen_escape)
//!   - `hlstyle(...)` ANSI emission with the truecolor + fbterm +
//!     tmux + screen branches faithful to py:100-160
//!
//! The PromptRenderer width-cache logic at py:38-72 is deferred —
//! it needs the parent Renderer's do_render output tuple shape
//! which isn't fully threaded through yet.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderer import Renderer           // py:4
// from powerline.theme import Theme                 // py:5
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE                 // py:6

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};
use std::collections::HashMap;

pub mod bash;
pub mod ksh;
pub mod rcsh;
pub mod readline;
pub mod tcsh;
pub mod zsh;

/// Port of `int_to_rgb()` from
/// `powerline/renderers/shell/__init__.py:9`.
///
/// Splits a 24-bit RGB int into `(r, g, b)` byte components.
pub fn int_to_rgb(num: u32) -> (u8, u8, u8) {
    // py:9   def int_to_rgb(num):
    // py:10  r = (num >> 16) & 0xff
    // py:11  g = (num >> 8) & 0xff
    // py:12  b = num & 0xff
    // py:13  return r, g, b
    let r = ((num >> 16) & 0xff) as u8;
    let g = ((num >> 8) & 0xff) as u8;
    let b = (num & 0xff) as u8;
    (r, g, b)
}

/// Port of `ShellRenderer.term_escape_style` config flag from
/// `powerline/renderers/shell/__init__.py:84`.
///
/// `Auto` selects per-call based on `$TERM` (fbterm vs xterm);
/// `Fbterm` / `Xterm` pin the style explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermEscapeStyle {
    Auto,
    Fbterm,
    Xterm,
}

impl TermEscapeStyle {
    /// Resolves `Auto` to the concrete style based on `$TERM`.
    /// Mirrors py:101-105:
    ///   if term_escape_style == 'auto':
    ///       used = 'fbterm' if env['TERM'] == 'fbterm' else 'xterm'
    pub fn resolve(self, term: Option<&str>) -> Self {
        // py:101-105  resolve auto via $TERM
        match self {
            TermEscapeStyle::Auto => {
                if term == Some("fbterm") {
                    TermEscapeStyle::Fbterm
                } else {
                    TermEscapeStyle::Xterm
                }
            }
            other => other,
        }
    }
}

/// Color descriptor: `(cterm_index, optional_truecolor_rgb)`.
///
/// Same shape as tmux.rs / vim.rs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorSpec {
    pub cterm: u16,
    pub truecolor: Option<u32>,
}

/// Port of `class PromptRenderer(Renderer)` from
/// `powerline/renderers/shell/__init__.py:16`.
///
/// Tracks per-(client_id, side, theme) widths for the daemon-mode
/// width cache. The full do_render orchestration depends on the
/// parent Renderer; the Rust port surfaces just the cache state.
pub struct PromptRenderer {
    /// Python: `self.old_widths` (py:20).
    pub old_widths: HashMap<(String, String, Option<u64>), usize>,
}

impl Default for PromptRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptRenderer {
    /// Port of `PromptRenderer.__init__()` from
    /// `powerline/renderers/shell/__init__.py:18`.
    pub fn new() -> Self {
        // py:16  class PromptRenderer(Renderer):
        // py:17  '''Powerline generic prompt segment renderer'''
        // py:19  def __init__(self, old_widths=None, **kwargs):
        // py:20  super(PromptRenderer, self).__init__(**kwargs)
        // py:21  self.old_widths = old_widths if old_widths is not None else {}
        Self {
            old_widths: HashMap::new(),
        }
    }

    /// Port of `PromptRenderer.get_client_id()` from
    /// `powerline/renderers/shell/__init__.py:22`.
    ///
    /// Returns the `client_id` from `segment_info`, or None when
    /// the key isn't present.
    pub fn get_client_id(segment_info: &serde_json::Map<String, serde_json::Value>) -> Option<u64> {
        // py:23  def get_client_id(self, segment_info):
        // py:24-35  docstring
        // py:36  return segment_info.get('client_id') if isinstance(segment_info, dict) else None
        segment_info.get("client_id").and_then(|v| v.as_u64())
    }
}

/// Port of `class ShellRenderer(PromptRenderer)` from
/// `powerline/renderers/shell/__init__.py:80`.
pub struct ShellRenderer {
    pub base: PromptRenderer,
    /// Python class attribute: `escape_hl_start = ''` (py:81).
    pub escape_hl_start: String,
    /// Python class attribute: `escape_hl_end = ''` (py:82).
    pub escape_hl_end: String,
    /// Python class attribute: `term_truecolor = False` (py:83).
    pub term_truecolor: bool,
    /// Python class attribute: `term_escape_style = 'auto'` (py:84).
    pub term_escape_style: TermEscapeStyle,
    /// Python instance attribute set by do_render at py:99-105.
    /// Caches the resolved (non-Auto) style for hlstyle().
    pub used_term_escape_style: TermEscapeStyle,
    /// Python class attribute: `tmux_escape = False` (py:85).
    pub tmux_escape: bool,
    /// Python class attribute: `screen_escape = False` (py:86).
    pub screen_escape: bool,
    /// Python: `self.theme` (inherited from Renderer base) used by
    /// `get_theme` at py:169.
    pub theme: serde_json::Value,
    /// Python: `self.local_themes` (inherited) — matcher_info → match
    /// dict mapping per py:170-179.
    pub local_themes: HashMap<String, serde_json::Map<String, serde_json::Value>>,
    /// Python: `self.theme_config` (inherited) — passed as
    /// main_theme_config to Theme construction at py:176.
    pub theme_config: serde_json::Value,
    /// Python: `self.theme_kwargs` (inherited) — splat into Theme
    /// construction at py:177.
    pub theme_kwargs: serde_json::Map<String, serde_json::Value>,
}

impl Default for ShellRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellRenderer {
    /// Constructs a `ShellRenderer` with the upstream defaults.
    ///
    /// `common.additional_escapes` (`reference.rst:76-84`) maps to the
    /// public `tmux_escape` / `screen_escape` fields; callers should
    /// set them post-construction per the Python
    /// `Powerline.create_renderer` flow at `__init__.py:600-606`:
    /// `tmux_escape = additional_escapes == 'tmux'`,
    /// `screen_escape = additional_escapes == 'screen'`.
    pub fn new() -> Self {
        Self {
            base: PromptRenderer::new(),
            escape_hl_start: String::new(),
            escape_hl_end: String::new(),
            term_truecolor: false,
            term_escape_style: TermEscapeStyle::Auto,
            used_term_escape_style: TermEscapeStyle::Xterm,
            tmux_escape: false,
            screen_escape: false,
            theme: serde_json::Value::Null,
            local_themes: HashMap::new(),
            theme_config: serde_json::Value::Null,
            theme_kwargs: serde_json::Map::new(),
        }
    }

    /// Port of `ShellRenderer.render()` from
    /// `powerline/renderers/shell/__init__.py:90-96`.
    ///
    /// Pulls `segment_info['local_theme']` and returns it as the
    /// matcher_info value that the super().render() call would
    /// receive at py:93. The actual super().render() dispatch is
    /// deferred since the parent Renderer's render isn't fully
    /// threaded.
    pub fn render_matcher_info(
        segment_info: &serde_json::Map<String, serde_json::Value>,
    ) -> Option<String> {
        // py:91  local_theme = segment_info.get('local_theme')
        segment_info
            .get("local_theme")
            .and_then(|v| v.as_str().map(String::from))
    }

    /// Port of `ShellRenderer.render()` from
    /// `powerline/renderers/shell/__init__.py:90-96`.
    ///
    /// Calls super().render() with `matcher_info=local_theme`
    /// derived from `segment_info`. Rust port takes the super
    /// dispatch as a closure since the base Renderer chain isn't
    /// reachable from a typed Rust struct.
    pub fn render<F>(
        segment_info: &serde_json::Map<String, serde_json::Value>,
        super_render: F,
    ) -> String
    where
        F: FnOnce(Option<&str>) -> String,
    {
        // py:90  def render(self, segment_info, **kwargs):
        // py:91  local_theme = segment_info.get('local_theme')
        let local_theme = Self::render_matcher_info(segment_info);
        // py:92-96  return super().render(matcher_info=local_theme, segment_info=..., **kwargs)
        super_render(local_theme.as_deref())
    }

    /// Port of `ShellRenderer.do_render()` from
    /// `powerline/renderers/shell/__init__.py:98-106`.
    ///
    /// Bare-name alias preserving the upstream Python `do_render`
    /// identifier. Resolves used_term_escape_style via
    /// `do_render_resolve_style`, then dispatches super().do_render
    /// via the caller-supplied closure.
    pub fn do_render<F>(
        &mut self,
        segment_info: &serde_json::Map<String, serde_json::Value>,
        super_do_render: F,
    ) -> String
    where
        F: FnOnce() -> String,
    {
        // py:98  def do_render(self, segment_info, **kwargs):
        // py:99-105  resolve used_term_escape_style
        self.do_render_resolve_style(segment_info);
        // py:106  return super().do_render(segment_info=..., **kwargs)
        super_do_render()
    }

    /// Port of `ShellRenderer.do_render()` from
    /// `powerline/renderers/shell/__init__.py:98-106`.
    ///
    /// Resolves `used_term_escape_style` based on `term_escape_style`
    /// and `segment_info['environ']['TERM']`. Mutates self per
    /// py:101-105. The super().do_render dispatch (py:106) is deferred
    /// to the Renderer port.
    pub fn do_render_resolve_style(
        &mut self,
        segment_info: &serde_json::Map<String, serde_json::Value>,
    ) {
        // py:99  if self.term_escape_style == 'auto':
        let resolved = if self.term_escape_style == TermEscapeStyle::Auto {
            // py:100-103  $TERM dispatch
            let term = segment_info
                .get("environ")
                .and_then(|e| e.as_object())
                .and_then(|m| m.get("TERM"))
                .and_then(|v| v.as_str());
            if term == Some("fbterm") {
                TermEscapeStyle::Fbterm
            } else {
                TermEscapeStyle::Xterm
            }
        } else {
            // py:104-105  used = configured value
            self.term_escape_style
        };
        self.used_term_escape_style = resolved;
    }

    /// Port of `ShellRenderer.get_theme()` from
    /// `powerline/renderers/shell/__init__.py:167-179`.
    ///
    /// If `matcher_info` is empty, returns `self.theme` per py:168-169.
    /// Otherwise resolves `local_themes[matcher_info]['theme']`,
    /// constructing it lazily from config + theme_config +
    /// theme_kwargs per py:173-179. Mirror of the IPython renderer
    /// pattern.
    pub fn get_theme(&mut self, matcher_info: Option<&str>) -> serde_json::Value {
        // py:168-169  if not matcher_info: return self.theme
        let m = match matcher_info {
            Some(s) if !s.is_empty() => s,
            _ => return self.theme.clone(),
        };
        // py:170-172  match['theme'] if present
        let match_entry = match self.local_themes.get(m) {
            Some(e) => e.clone(),
            None => return serde_json::Value::Null,
        };
        if let Some(t) = match_entry.get("theme") {
            return t.clone();
        }
        // py:174-179  Theme(theme_config=match['config'],
        //                   main_theme_config=self.theme_config,
        //                   **self.theme_kwargs)
        let constructed = serde_json::json!({
            "theme_config": match_entry.get("config").cloned().unwrap_or(serde_json::Value::Null),
            "main_theme_config": self.theme_config.clone(),
            "theme_kwargs": serde_json::Value::Object(self.theme_kwargs.clone()),
        });
        if let Some(e) = self.local_themes.get_mut(m) {
            e.insert("theme".to_string(), constructed.clone());
        }
        constructed
    }

    /// Port of `ShellRenderer.hlstyle()` from
    /// `powerline/renderers/shell/__init__.py:110`.
    ///
    /// Emits the ANSI escape sequence(s) for the given fg/bg/attrs.
    /// `term` is the `$TERM` value used to resolve `Auto` to either
    /// `Fbterm` or `Xterm`. `escape=true` wraps the output with
    /// `escape_hl_start`/`escape_hl_end`.
    pub fn hlstyle(
        &self,
        fg: Option<ColorSpec>,
        bg: Option<ColorSpec>,
        attrs: Option<u32>,
        escape: bool,
        term: Option<&str>,
    ) -> String {
        // py:108  def hlstyle(self, fg=None, bg=None, attrs=None, escape=True, **kwargs):
        // py:109-114  docstring
        // py:115  ansi = [0]
        let mut ansi: Vec<u32> = vec![0];
        // py:116  is_fbterm = self.used_term_escape_style == 'fbterm'
        let style = self.term_escape_style.resolve(term);
        let is_fbterm = style == TermEscapeStyle::Fbterm;
        // py:117  term_truecolor = not is_fbterm and self.term_truecolor
        let term_truecolor = !is_fbterm && self.term_truecolor;

        // py:118  if fg is not None:
        // py:119  if fg is False or fg[0] is False:
        // py:120  ansi += [39]
        // py:121  else:
        // py:122  if term_truecolor:
        // py:123  ansi += [38, 2] + list(int_to_rgb(fg[1]))
        // py:124  else:
        // py:125  ansi += [38, 5, fg[0]]
        if let Some(f) = fg {
            if let (true, Some(tc)) = (term_truecolor, f.truecolor) {
                let (r, g, b) = int_to_rgb(tc);
                ansi.extend_from_slice(&[38, 2, r as u32, g as u32, b as u32]);
            } else {
                ansi.extend_from_slice(&[38, 5, f.cterm as u32]);
            }
        }
        // py:126  if bg is not None:
        // py:127  if bg is False or bg[0] is False:
        // py:128  ansi += [49]
        // py:129  else:
        // py:130  if term_truecolor:
        // py:131  ansi += [48, 2] + list(int_to_rgb(bg[1]))
        // py:132  else:
        // py:133  ansi += [48, 5, bg[0]]
        if let Some(b) = bg {
            if let (true, Some(tc)) = (term_truecolor, b.truecolor) {
                let (r, g, bl) = int_to_rgb(tc);
                ansi.extend_from_slice(&[48, 2, r as u32, g as u32, bl as u32]);
            } else {
                ansi.extend_from_slice(&[48, 5, b.cterm as u32]);
            }
        }
        // py:134  if attrs is not None:
        // py:135  if attrs is False:
        // py:136  ansi += [22]
        // py:137  else:
        // py:138  if attrs & ATTR_BOLD:
        // py:139  ansi += [1]
        // py:140  elif attrs & ATTR_ITALIC:
        // py:141  # Note: is likely not to work or even be inverse in place of italic.
        // py:143  ansi += [3]
        // py:144  elif attrs & ATTR_UNDERLINE:
        // py:145  ansi += [4]
        if let Some(a) = attrs {
            if a & ATTR_BOLD != 0 {
                ansi.push(1);
            } else if a & ATTR_ITALIC != 0 {
                ansi.push(3);
            } else if a & ATTR_UNDERLINE != 0 {
                ansi.push(4);
            }
        }

        // py:146  if is_fbterm:
        // py:147  r = []
        // py:148  while ansi:
        // py:149  cur_ansi = ansi.pop(0)
        // py:150  if cur_ansi == 38:
        // py:151  ansi.pop(0)
        // py:152  r.append('\033[1;{0}}}'.format(ansi.pop(0)))
        // py:153  elif cur_ansi == 48:
        // py:154  ansi.pop(0)
        // py:155  r.append('\033[2;{0}}}'.format(ansi.pop(0)))
        // py:156  else:
        // py:157  r.append('\033[{0}m'.format(cur_ansi))
        // py:158  r = ''.join(r)
        let r = if is_fbterm {
            let mut out = String::new();
            let mut iter = ansi.into_iter();
            while let Some(cur) = iter.next() {
                if cur == 38 {
                    iter.next();
                    let idx = iter.next().unwrap_or(0);
                    out.push_str(&format!("\x1b[1;{}}}", idx));
                } else if cur == 48 {
                    iter.next();
                    let idx = iter.next().unwrap_or(0);
                    out.push_str(&format!("\x1b[2;{}}}", idx));
                } else {
                    out.push_str(&format!("\x1b[{}m", cur));
                }
            }
            out
        } else {
            // py:159  else:
            // py:160  r = '\033[{0}m'.format(';'.join(str(attr) for attr in ansi))
            let joined: Vec<String> = ansi.into_iter().map(|n| n.to_string()).collect();
            format!("\x1b[{}m", joined.join(";"))
        };

        // py:161  if self.tmux_escape:
        // py:162  r = '\033Ptmux;' + r.replace('\033', '\033\033') + '\033\\'
        // py:163  elif self.screen_escape:
        // py:164  r = '\033P' + r.replace('\033', '\033\033') + '\033\\'
        let r = if self.tmux_escape {
            format!("\x1bPtmux;{}\x1b\\", r.replace('\x1b', "\x1b\x1b"))
        } else if self.screen_escape {
            format!("\x1bP{}\x1b\\", r.replace('\x1b', "\x1b\x1b"))
        } else {
            r
        };
        // py:165  return self.escape_hl_start + r + self.escape_hl_end if escape else r
        if escape {
            format!("{}{}{}", self.escape_hl_start, r, self.escape_hl_end)
        } else {
            r
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    #[test]
    fn int_to_rgb_extracts_components() {
        // py:10-13
        assert_eq!(int_to_rgb(0xff0000), (0xff, 0, 0));
        assert_eq!(int_to_rgb(0x00ff00), (0, 0xff, 0));
        assert_eq!(int_to_rgb(0x0000ff), (0, 0, 0xff));
        assert_eq!(int_to_rgb(0xff8000), (0xff, 0x80, 0));
        assert_eq!(int_to_rgb(0x000000), (0, 0, 0));
    }

    #[test]
    fn term_escape_style_auto_resolves_fbterm() {
        // py:101-103
        let r = TermEscapeStyle::Auto.resolve(Some("fbterm"));
        assert_eq!(r, TermEscapeStyle::Fbterm);
    }

    #[test]
    fn term_escape_style_auto_resolves_xterm_for_other_term() {
        let r = TermEscapeStyle::Auto.resolve(Some("xterm-256color"));
        assert_eq!(r, TermEscapeStyle::Xterm);
    }

    #[test]
    fn term_escape_style_auto_resolves_xterm_when_term_unset() {
        let r = TermEscapeStyle::Auto.resolve(None);
        assert_eq!(r, TermEscapeStyle::Xterm);
    }

    #[test]
    fn term_escape_style_pinned_value_passthrough() {
        assert_eq!(
            TermEscapeStyle::Fbterm.resolve(Some("xterm")),
            TermEscapeStyle::Fbterm
        );
        assert_eq!(
            TermEscapeStyle::Xterm.resolve(Some("fbterm")),
            TermEscapeStyle::Xterm
        );
    }

    #[test]
    fn prompt_renderer_get_client_id_returns_value() {
        let mut info = Map::new();
        info.insert("client_id".to_string(), serde_json::json!(42));
        let r = PromptRenderer::get_client_id(&info);
        assert_eq!(r, Some(42));
    }

    #[test]
    fn prompt_renderer_get_client_id_returns_none_when_absent() {
        let info = Map::new();
        assert!(PromptRenderer::get_client_id(&info).is_none());
    }

    #[test]
    fn shell_renderer_default_field_values_match_upstream() {
        let r = ShellRenderer::new();
        assert_eq!(r.escape_hl_start, "");
        assert_eq!(r.escape_hl_end, "");
        assert!(!r.term_truecolor);
        assert_eq!(r.term_escape_style, TermEscapeStyle::Auto);
        assert!(!r.tmux_escape);
        assert!(!r.screen_escape);
    }

    #[test]
    fn hlstyle_no_args_returns_just_reset_sequence() {
        // py:117  ansi = [0]
        let r = ShellRenderer::new();
        let s = r.hlstyle(None, None, None, false, Some("xterm"));
        assert_eq!(s, "\x1b[0m");
    }

    #[test]
    fn hlstyle_fg_cterm_emits_38_5_n() {
        // py:126-128  fg 256-color
        let r = ShellRenderer::new();
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 196,
                truecolor: Some(0xff0000),
            }),
            None,
            None,
            false,
            Some("xterm"),
        );
        assert_eq!(s, "\x1b[0;38;5;196m");
    }

    #[test]
    fn hlstyle_fg_truecolor_emits_38_2_rgb_when_enabled() {
        let mut r = ShellRenderer::new();
        r.term_truecolor = true;
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 196,
                truecolor: Some(0xff0000),
            }),
            None,
            None,
            false,
            Some("xterm"),
        );
        assert_eq!(s, "\x1b[0;38;2;255;0;0m");
    }

    #[test]
    fn hlstyle_truecolor_disabled_on_fbterm() {
        // py:119  term_truecolor = not is_fbterm and self.term_truecolor
        let mut r = ShellRenderer::new();
        r.term_truecolor = true;
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 196,
                truecolor: Some(0xff0000),
            }),
            None,
            None,
            false,
            Some("fbterm"),
        );
        // fbterm dispatches differently — sequence breaks at 38
        assert!(s.contains("\x1b[1;196}"));
    }

    #[test]
    fn hlstyle_bg_emits_48_5_n() {
        let r = ShellRenderer::new();
        let s = r.hlstyle(
            None,
            Some(ColorSpec {
                cterm: 21,
                truecolor: None,
            }),
            None,
            false,
            Some("xterm"),
        );
        assert_eq!(s, "\x1b[0;48;5;21m");
    }

    #[test]
    fn hlstyle_attrs_bold_emits_1() {
        let r = ShellRenderer::new();
        let s = r.hlstyle(None, None, Some(ATTR_BOLD), false, Some("xterm"));
        assert_eq!(s, "\x1b[0;1m");
    }

    #[test]
    fn hlstyle_attrs_italic_emits_3() {
        let r = ShellRenderer::new();
        let s = r.hlstyle(None, None, Some(ATTR_ITALIC), false, Some("xterm"));
        assert_eq!(s, "\x1b[0;3m");
    }

    #[test]
    fn hlstyle_attrs_underline_emits_4() {
        let r = ShellRenderer::new();
        let s = r.hlstyle(None, None, Some(ATTR_UNDERLINE), false, Some("xterm"));
        assert_eq!(s, "\x1b[0;4m");
    }

    #[test]
    fn hlstyle_combined_fg_bg_attrs() {
        let r = ShellRenderer::new();
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
            false,
            Some("xterm"),
        );
        assert_eq!(s, "\x1b[0;38;5;231;48;5;21;1m");
    }

    #[test]
    fn hlstyle_escape_true_wraps_with_hl_start_and_end() {
        // py:168  start + r + end if escape
        let mut r = ShellRenderer::new();
        r.escape_hl_start = "<".to_string();
        r.escape_hl_end = ">".to_string();
        let s = r.hlstyle(None, None, None, true, Some("xterm"));
        assert_eq!(s, "<\x1b[0m>");
    }

    #[test]
    fn hlstyle_tmux_escape_wraps_with_dcs() {
        // py:163-164  '\033Ptmux;...\033\\'
        let mut r = ShellRenderer::new();
        r.tmux_escape = true;
        let s = r.hlstyle(None, None, None, false, Some("xterm"));
        // ESC ESC = doubled escape
        assert!(s.starts_with("\x1bPtmux;"));
        assert!(s.ends_with("\x1b\\"));
        assert!(s.contains("\x1b\x1b["));
    }

    #[test]
    fn hlstyle_screen_escape_wraps_with_dcs() {
        // py:165-166  '\033P...\033\\'
        let mut r = ShellRenderer::new();
        r.screen_escape = true;
        let s = r.hlstyle(None, None, None, false, Some("xterm"));
        assert!(s.starts_with("\x1bP"));
        assert!(s.ends_with("\x1b\\"));
        assert!(s.contains("\x1b\x1b["));
    }

    #[test]
    fn render_matcher_info_pulls_local_theme() {
        // py:91  segment_info.get('local_theme')
        let mut info = Map::new();
        info.insert("local_theme".to_string(), serde_json::json!("ipython"));
        assert_eq!(
            ShellRenderer::render_matcher_info(&info),
            Some("ipython".to_string())
        );
    }

    #[test]
    fn render_matcher_info_returns_none_when_unset() {
        let info = Map::new();
        assert!(ShellRenderer::render_matcher_info(&info).is_none());
    }

    #[test]
    fn do_render_resolve_style_auto_dispatches_fbterm() {
        // py:99-103
        let mut r = ShellRenderer::new();
        let mut info = Map::new();
        info.insert("environ".to_string(), serde_json::json!({"TERM": "fbterm"}));
        r.do_render_resolve_style(&info);
        assert_eq!(r.used_term_escape_style, TermEscapeStyle::Fbterm);
    }

    #[test]
    fn do_render_resolve_style_auto_defaults_to_xterm() {
        // py:101-103  TERM != 'fbterm' → xterm
        let mut r = ShellRenderer::new();
        let mut info = Map::new();
        info.insert(
            "environ".to_string(),
            serde_json::json!({"TERM": "xterm-256color"}),
        );
        r.do_render_resolve_style(&info);
        assert_eq!(r.used_term_escape_style, TermEscapeStyle::Xterm);
    }

    #[test]
    fn do_render_resolve_style_pinned_value_passes_through() {
        // py:104-105
        let mut r = ShellRenderer::new();
        r.term_escape_style = TermEscapeStyle::Fbterm;
        let mut info = Map::new();
        info.insert(
            "environ".to_string(),
            serde_json::json!({"TERM": "xterm-256color"}),
        );
        r.do_render_resolve_style(&info);
        assert_eq!(r.used_term_escape_style, TermEscapeStyle::Fbterm);
    }

    #[test]
    fn shell_renderer_get_theme_no_matcher_returns_self_theme() {
        // py:168-169
        let mut r = ShellRenderer::new();
        r.theme = serde_json::json!({"name": "default"});
        let t = r.get_theme(None);
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn shell_renderer_get_theme_empty_string_returns_self_theme() {
        // py:168  if not matcher_info → empty string is falsy
        let mut r = ShellRenderer::new();
        r.theme = serde_json::json!({"name": "default"});
        let t = r.get_theme(Some(""));
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn shell_renderer_get_theme_returns_existing_local_theme() {
        // py:171-172
        let mut r = ShellRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert(
            "theme".to_string(),
            serde_json::json!({"name": "shell-out"}),
        );
        r.local_themes.insert("out".to_string(), entry);
        let t = r.get_theme(Some("out"));
        assert_eq!(t["name"], "shell-out");
    }

    #[test]
    fn shell_renderer_get_theme_constructs_lazy_theme() {
        // py:173-179
        let mut r = ShellRenderer::new();
        r.theme_config = serde_json::json!({"colorscheme": "default"});
        r.theme_kwargs
            .insert("extra".to_string(), serde_json::json!("kw_value"));
        let mut entry = serde_json::Map::new();
        entry.insert("config".to_string(), serde_json::json!({"segments": []}));
        r.local_themes.insert("rewrite".to_string(), entry);

        let t = r.get_theme(Some("rewrite"));
        assert_eq!(t["theme_config"]["segments"], serde_json::json!([]));
        assert_eq!(t["main_theme_config"]["colorscheme"], "default");
        assert_eq!(t["theme_kwargs"]["extra"], "kw_value");
    }

    #[test]
    fn shell_renderer_get_theme_caches_constructed_theme() {
        // py:174  match['theme'] = Theme(...)
        let mut r = ShellRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert("config".to_string(), serde_json::json!({"a": 1}));
        r.local_themes.insert("rewrite".to_string(), entry);

        let _ = r.get_theme(Some("rewrite"));
        let cached = r
            .local_themes
            .get("rewrite")
            .and_then(|m| m.get("theme"))
            .cloned();
        assert!(cached.is_some(), "constructed theme not cached");
    }

    #[test]
    fn shell_renderer_get_theme_missing_matcher_returns_null() {
        let mut r = ShellRenderer::new();
        assert_eq!(r.get_theme(Some("nonexistent")), serde_json::Value::Null);
    }
}
