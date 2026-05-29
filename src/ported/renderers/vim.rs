// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/vim.py`.
//!
//! Vim statusline renderer. Emits `%#GroupName#` highlight-group
//! references and a per-group `:hi GroupName ctermfg=... guifg=...`
//! command. The renderer maintains a `(fg, bg, attrs) → hl_group`
//! cache so each unique combination produces exactly one vim `hi`
//! command for the session.
//!
//! Heavy parts ported here: hl_group naming, attrs → cterm-attribute
//! list, the `hi` command string, the mode-translation table, the
//! character-translation table override. The render() / shutdown() /
//! get_theme() / get_segment_info() flows depend on the unported
//! Renderer base + vim module access and are deferred.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// import vim                                        // py:6  (Python vim module)
// from powerline.bindings.vim import vim_get_func, vim_getoption, environ, current_tabpage, get_vim_encoding   // py:8
// from powerline.renderer import Renderer            // py:9
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE                  // py:10
// from powerline.theme import Theme                  // py:11
// from powerline.lib.unicode import unichr, register_strwidth_error                          // py:12

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};
use std::collections::HashMap;

/// Color descriptor: `(cterm_index, optional_truecolor_rgb)`.
///
/// Same shape as the tmux renderer's ColorSpec; the Python source
/// passes raw `(cterm, hex_int)` tuples through both call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorSpec {
    pub cterm: u16,
    pub truecolor: Option<u32>,
}

/// Port of `mode_translations` from
/// `powerline/renderers/vim.py:20-23`.
///
/// Returns the table mapping ctrl-V / ctrl-S mode strings to their
/// displayable forms (`^V` / `^S`).
pub fn mode_translations() -> HashMap<String, String> {
    // py:21  unichr(ord('V') - 0x40) = '\x16' = ctrl-V
    // py:22  unichr(ord('S') - 0x40) = '\x13' = ctrl-S
    let mut t = HashMap::new();
    t.insert("\x16".to_string(), "^V".to_string());
    t.insert("\x13".to_string(), "^S".to_string());
    t
}

/// Cached highlight-group record. Mirrors the Python dict at
/// `powerline/renderers/vim.py:144-151`.
#[derive(Debug, Clone)]
pub struct HlGroup {
    /// Python: `hl_group['ctermfg']` — 'NONE' or the cterm index.
    pub ctermfg: String,
    /// Python: `hl_group['guifg']` — Option<u32> RGB.
    pub guifg: Option<u32>,
    /// Python: `hl_group['ctermbg']` — 'NONE' or the cterm index.
    pub ctermbg: String,
    /// Python: `hl_group['guibg']` — Option<u32> RGB.
    pub guibg: Option<u32>,
    /// Python: `hl_group['attrs']` — names like 'bold','italic'.
    pub attrs: Vec<String>,
    /// Python: `hl_group['name']` — the synthetic `Pl_..._...` name.
    pub name: String,
}

impl HlGroup {
    /// Builds the `:hi <name> ctermfg=... guifg=... ...` command
    /// string the Python renderer issues via `vim.command(...)` at
    /// py:174.
    pub fn vim_command(&self) -> String {
        // py:175-180
        let guifg = match self.guifg {
            Some(rgb) => format!("#{:06x}", rgb),
            None => "NONE".to_string(),
        };
        let guibg = match self.guibg {
            Some(rgb) => format!("#{:06x}", rgb),
            None => "NONE".to_string(),
        };
        let attrs = if self.attrs.is_empty() {
            "NONE".to_string()
        } else {
            self.attrs.join(",")
        };
        format!(
            "hi {} ctermfg={} guifg={} guibg={} ctermbg={} cterm={} gui={}",
            self.name, self.ctermfg, guifg, guibg, self.ctermbg, attrs, attrs
        )
    }
}

/// Port of `class VimRenderer(Renderer)` from
/// `powerline/renderers/vim.py:26`.
pub struct VimRenderer {
    /// Python: `self.hl_groups` — cache keyed by (fg, bg, attrs).
    pub hl_groups: HashMap<(Option<ColorSpec>, Option<ColorSpec>, u32), HlGroup>,
    /// Python: `self.prev_highlight` — last hlstyle args; lets the
    /// renderer return "" when two consecutive segments share a style
    /// (vim E541 mitigation per py:130-131).
    pub prev_highlight: Option<(Option<ColorSpec>, Option<ColorSpec>, u32)>,
}

impl Default for VimRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl VimRenderer {
    /// Port of `VimRenderer.__init__()` from
    /// `powerline/renderers/vim.py:35`.
    pub fn new() -> Self {
        Self {
            hl_groups: HashMap::new(),
            prev_highlight: None,
        }
    }

    /// Port of `VimRenderer.character_translations` from
    /// `powerline/renderers/vim.py:30-31`.
    ///
    /// Vim format strings use `%` as the escape character, so literal
    /// `%` must be doubled.
    pub fn character_translations() -> Vec<(char, &'static str)> {
        // py:30-31  character_translations[ord('%')] = '%%'
        vec![('%', "%%")]
    }

    /// Port of `VimRenderer.reset_highlight()` from
    /// `powerline/renderers/vim.py:121`.
    pub fn reset_highlight(&mut self) {
        // py:122  self.hl_groups.clear()
        self.hl_groups.clear();
    }

    /// Builds the `Pl_<ctermfg>_<guifg>_<ctermbg>_<guibg>_<attrs>`
    /// synthetic group name per py:165-172.
    fn build_group_name(g: &HlGroup) -> String {
        let guifg = match g.guifg {
            Some(rgb) => format!("{}", rgb),
            None => "None".to_string(),
        };
        let guibg = match g.guibg {
            Some(rgb) => format!("{}", rgb),
            None => "None".to_string(),
        };
        // py:165-172  'Pl_' + str(...) + '_' + ... + ''.join(attrs)
        format!(
            "Pl_{}_{}_{}_{}_{}",
            g.ctermfg,
            guifg,
            g.ctermbg,
            guibg,
            g.attrs.join("")
        )
    }

    /// Builds the attrs name list (`'bold'`, `'italic'`,
    /// `'underline'`) from a powerline ATTR_* bitfield per py:157-163.
    pub fn attrs_to_hi_attrs(attrs: u32) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // py:158-159  ATTR_BOLD
        if attrs & ATTR_BOLD != 0 {
            out.push("bold".to_string());
        }
        // py:160-161  ATTR_ITALIC
        if attrs & ATTR_ITALIC != 0 {
            out.push("italic".to_string());
        }
        // py:162-163  ATTR_UNDERLINE
        if attrs & ATTR_UNDERLINE != 0 {
            out.push("underline".to_string());
        }
        out
    }

    /// Port of `VimRenderer.hlstyle()` from
    /// `powerline/renderers/vim.py:124`.
    ///
    /// Returns a `%#GROUP_NAME#` reference if a style is requested,
    /// empty string when the style is unchanged or absent.
    /// The accompanying `:hi` command is appended to `commands` so the
    /// caller can issue it via vim's command interface (Python's
    /// `vim.command(...)` at py:174 isn't reachable from Rust).
    pub fn hlstyle(
        &mut self,
        fg: Option<ColorSpec>,
        bg: Option<ColorSpec>,
        attrs: Option<u32>,
        commands: &mut Vec<String>,
    ) -> String {
        // py:131  attrs = attrs or 0
        let attrs = attrs.unwrap_or(0);
        // py:132-134  (fg, bg, attrs) == prev → skip
        if Some((fg, bg, attrs)) == self.prev_highlight {
            return String::new();
        }
        self.prev_highlight = Some((fg, bg, attrs));

        // py:137-138  no attrs/bg/fg → no-op (reset implicit in vim)
        if attrs == 0 && bg.is_none() && fg.is_none() {
            return String::new();
        }

        let key = (fg, bg, attrs);
        // py:140-181  if (fg, bg, attrs) not in hl_groups: build + cache.
        // clippy: HashMap-or-default insertion lint allowed because
        // the Python source phrases the cache check as
        // `if key not in self.hl_groups:` and the matching Rust idiom
        // here preserves intent.
        #[allow(clippy::map_entry)]
        if !self.hl_groups.contains_key(&key) {
            // py:142-151  build hl_group dict
            let mut g = HlGroup {
                ctermfg: "NONE".to_string(),
                guifg: None,
                ctermbg: "NONE".to_string(),
                guibg: None,
                attrs: vec!["NONE".to_string()],
                name: String::new(),
            };
            // py:152-154  fg
            if let Some(f) = fg {
                g.ctermfg = f.cterm.to_string();
                g.guifg = f.truecolor;
            }
            // py:155-156  bg
            if let Some(b) = bg {
                g.ctermbg = b.cterm.to_string();
                g.guibg = b.truecolor;
            }
            // py:157-163  attrs
            if attrs != 0 {
                g.attrs = Self::attrs_to_hi_attrs(attrs);
            }
            // py:165-172  synthetic name
            g.name = Self::build_group_name(&g);
            // py:174-181  issue vim command (deferred via commands buffer)
            commands.push(g.vim_command());
            self.hl_groups.insert(key, g);
        }
        // py:182  '%#' + name + '#'
        format!("%#{}#", self.hl_groups[&key].name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_translations_contains_ctrl_v_and_ctrl_s() {
        let t = mode_translations();
        // py:21  ctrl-V = \x16
        assert_eq!(t.get("\x16").map(|s| s.as_str()), Some("^V"));
        // py:22  ctrl-S = \x13
        assert_eq!(t.get("\x13").map(|s| s.as_str()), Some("^S"));
    }

    #[test]
    fn character_translations_doubles_percent() {
        let t = VimRenderer::character_translations();
        assert_eq!(t, vec![('%', "%%")]);
    }

    #[test]
    fn attrs_to_hi_attrs_zero_is_empty() {
        // py:158  attrs path only runs when attrs is truthy
        assert!(VimRenderer::attrs_to_hi_attrs(0).is_empty());
    }

    #[test]
    fn attrs_to_hi_attrs_bold() {
        assert_eq!(VimRenderer::attrs_to_hi_attrs(ATTR_BOLD), vec!["bold"]);
    }

    #[test]
    fn attrs_to_hi_attrs_all_three() {
        let r = VimRenderer::attrs_to_hi_attrs(ATTR_BOLD | ATTR_ITALIC | ATTR_UNDERLINE);
        assert_eq!(r, vec!["bold", "italic", "underline"]);
    }

    #[test]
    fn reset_highlight_clears_cache() {
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        r.hlstyle(
            Some(ColorSpec {
                cterm: 231,
                truecolor: None,
            }),
            None,
            Some(0),
            &mut commands,
        );
        assert!(!r.hl_groups.is_empty());
        r.reset_highlight();
        assert!(r.hl_groups.is_empty());
    }

    #[test]
    fn hlstyle_no_args_returns_empty() {
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        assert_eq!(r.hlstyle(None, None, None, &mut commands), "");
        assert!(commands.is_empty());
    }

    #[test]
    fn hlstyle_emits_group_reference() {
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        let s = r.hlstyle(
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffffff),
            }),
            None,
            None,
            &mut commands,
        );
        assert!(s.starts_with("%#Pl_231_"));
        assert!(s.ends_with("#"));
        // One `hi` command queued.
        assert_eq!(commands.len(), 1);
        assert!(commands[0].starts_with("hi Pl_231_"));
        assert!(commands[0].contains("guifg=#ffffff"));
    }

    #[test]
    fn hlstyle_consecutive_identical_returns_empty() {
        // py:130-131  squash dup style
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        let fg = Some(ColorSpec {
            cterm: 21,
            truecolor: None,
        });
        let first = r.hlstyle(fg, None, None, &mut commands);
        assert!(!first.is_empty());
        // Second consecutive call with same args → empty (no new command).
        let second = r.hlstyle(fg, None, None, &mut commands);
        assert_eq!(second, "");
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn hlstyle_caches_repeated_group_after_other_style() {
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        let fg_a = Some(ColorSpec {
            cterm: 21,
            truecolor: None,
        });
        let fg_b = Some(ColorSpec {
            cterm: 22,
            truecolor: None,
        });
        let _ = r.hlstyle(fg_a, None, None, &mut commands);
        let _ = r.hlstyle(fg_b, None, None, &mut commands);
        let s = r.hlstyle(fg_a, None, None, &mut commands);
        // Group already cached → no new `hi` command, but reference returned.
        assert!(s.starts_with("%#Pl_21_"));
        assert_eq!(commands.len(), 2); // only two unique groups created.
    }

    #[test]
    fn hl_group_vim_command_format() {
        let g = HlGroup {
            ctermfg: "231".to_string(),
            guifg: Some(0xffffff),
            ctermbg: "21".to_string(),
            guibg: Some(0x0000ff),
            attrs: vec!["bold".to_string()],
            name: "Pl_231_16777215_21_255_bold".to_string(),
        };
        let cmd = g.vim_command();
        assert!(cmd.starts_with("hi Pl_231_16777215_21_255_bold "));
        assert!(cmd.contains("ctermfg=231"));
        assert!(cmd.contains("guifg=#ffffff"));
        assert!(cmd.contains("ctermbg=21"));
        assert!(cmd.contains("guibg=#0000ff"));
        assert!(cmd.contains("cterm=bold"));
        assert!(cmd.contains("gui=bold"));
    }

    #[test]
    fn hl_group_vim_command_none_attrs_emits_none() {
        let g = HlGroup {
            ctermfg: "NONE".to_string(),
            guifg: None,
            ctermbg: "NONE".to_string(),
            guibg: None,
            attrs: vec!["NONE".to_string()],
            name: "Pl_NONE_None_NONE_None_NONE".to_string(),
        };
        let cmd = g.vim_command();
        assert!(cmd.contains("guifg=NONE"));
        assert!(cmd.contains("guibg=NONE"));
        assert!(cmd.contains("cterm=NONE"));
    }

    #[test]
    fn hlstyle_attrs_only_emits_cterm_attrs() {
        let mut r = VimRenderer::new();
        let mut commands = Vec::new();
        let s = r.hlstyle(None, None, Some(ATTR_BOLD | ATTR_UNDERLINE), &mut commands);
        assert!(s.starts_with("%#Pl_NONE_None_NONE_None_boldunderline#"));
        assert!(commands[0].contains("cterm=bold,underline"));
    }
}
