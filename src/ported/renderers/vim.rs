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
    /// Python: `hl_group['guifg']` — `Option<u32>` RGB.
    pub guifg: Option<u32>,
    /// Python: `hl_group['ctermbg']` — 'NONE' or the cterm index.
    pub ctermbg: String,
    /// Python: `hl_group['guibg']` — `Option<u32>` RGB.
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
    /// Python: `self.theme` (inherited from Renderer base). Used by
    /// shutdown at py:48 and get_theme at py:72.
    pub theme: serde_json::Value,
    /// Python: `self.local_themes` (inherited) — `matcher_info_key →
    /// match_dict`. The `match_dict` mirror's Python's `{'config': ...,
    /// 'theme': ...}` shape with the theme constructed lazily.
    pub local_themes: std::collections::HashMap<String, serde_json::Map<String, serde_json::Value>>,
    /// Python: `self.theme_config` (inherited) — passed as
    /// main_theme_config to Theme construction at py:62.
    pub theme_config: serde_json::Value,
    /// Python: `self.theme_kwargs` (inherited) — splat into Theme
    /// construction at py:62.
    pub theme_kwargs: serde_json::Map<String, serde_json::Value>,
    /// Python: `self.segment_info` (inherited via Renderer class
    /// attribute at py:32-33). The `environ` key gets patched in
    /// per py:33; Rust stores the merged dict directly.
    pub segment_info: serde_json::Map<String, serde_json::Value>,
    /// Records shutdown-call order — used in lieu of the
    /// Theme.shutdown() side effect since the Theme class isn't yet
    /// ported. See py:48-51.
    pub shutdown_called: std::sync::Mutex<Vec<String>>,
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
        // py:35  def __init__(self, *args, **kwargs):
        // py:36  if not hasattr(vim, 'strwidth'):
        // py:37  # Hope nobody want to change this at runtime
        // py:38  if vim.eval('&ambiwidth') == 'double':
        // py:39  kwargs = dict(**kwargs)
        // py:40  kwargs['ambigious'] = 2
        // py:41  super(VimRenderer, self).__init__(*args, **kwargs)
        // py:42  self.hl_groups = {}
        // py:43  self.prev_highlight = None
        // py:44  self.strwidth_error_name = register_strwidth_error(self.strwidth)
        // py:45  self.encoding = get_vim_encoding()
        Self {
            hl_groups: HashMap::new(),
            prev_highlight: None,
            theme: serde_json::Value::Null,
            local_themes: std::collections::HashMap::new(),
            theme_config: serde_json::Value::Null,
            theme_kwargs: serde_json::Map::new(),
            segment_info: serde_json::Map::new(),
            shutdown_called: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Port of `VimRenderer.shutdown()` from
    /// `powerline/renderers/vim.py:47-51`.
    ///
    /// Calls `theme.shutdown()` + every local theme's `shutdown()`
    /// per py:48-51. The Rust port records the shutdown order in
    /// `shutdown_called` for test assertion since `Theme.shutdown` is
    /// not yet ported.
    pub fn shutdown(&self) {
        // py:48  self.theme.shutdown()
        let mut log = self
            .shutdown_called
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        log.push("theme".to_string());
        // py:49-51  for match in self.local_themes.values():
        //             if 'theme' in match: match['theme'].shutdown()
        for (name, match_entry) in &self.local_themes {
            if match_entry.contains_key("theme") {
                log.push(name.clone());
            }
        }
    }

    /// Port of `VimRenderer.add_local_theme()` from
    /// `powerline/renderers/vim.py:53-56`.
    ///
    /// Inserts `theme` into `local_themes[matcher]`. Returns an
    /// `Err` matching Python's `KeyError('There is already a local
    /// theme ...')` per py:55 when the matcher is already present.
    pub fn add_local_theme(
        &mut self,
        matcher: &str,
        theme: serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), String> {
        // py:54-55  if matcher in self.local_themes: raise KeyError
        if self.local_themes.contains_key(matcher) {
            return Err("There is already a local theme with given matcher".to_string());
        }
        // py:56  self.local_themes[matcher] = theme
        self.local_themes.insert(matcher.to_string(), theme);
        Ok(())
    }

    /// Port of `VimRenderer.get_matched_theme()` from
    /// `powerline/renderers/vim.py:58-63`.
    ///
    /// Lazy theme builder: returns the cached `theme` if present,
    /// else constructs a fresh Theme dict from `config` +
    /// `theme_config` + `theme_kwargs` per py:62 and caches it back
    /// into the match dict.
    ///
    /// The Theme class isn't yet ported so the constructed value is a
    /// JSON object snapshot of the construction kwargs. Same pattern
    /// as IPython/Shell renderer ports.
    pub fn get_matched_theme(&self, matcher: &str) -> serde_json::Value {
        let match_entry = match self.local_themes.get(matcher) {
            Some(e) => e,
            None => return serde_json::Value::Null,
        };
        // py:60  return match['theme'] if present
        if let Some(t) = match_entry.get("theme") {
            return t.clone();
        }
        // py:62  match['theme'] = Theme(theme_config=..., main_theme_config=..., **theme_kwargs)
        serde_json::json!({
            "theme_config": match_entry.get("config").cloned().unwrap_or(serde_json::Value::Null),
            "main_theme_config": self.theme_config.clone(),
            "theme_kwargs": serde_json::Value::Object(self.theme_kwargs.clone()),
        })
    }

    /// Mutating variant of [`Self::get_matched_theme`] that caches the
    /// constructed theme back into the match dict per py:62. The
    /// non-mutating variant above is safe to call from `get_theme`
    /// which needs to walk the matcher table.
    pub fn get_matched_theme_cached(&mut self, matcher: &str) -> serde_json::Value {
        let constructed = self.get_matched_theme(matcher);
        if let Some(entry) = self.local_themes.get_mut(matcher) {
            if !entry.contains_key("theme") {
                entry.insert("theme".to_string(), constructed.clone());
            }
        }
        constructed
    }

    /// Port of `VimRenderer.get_theme()` from
    /// `powerline/renderers/vim.py:65-72`.
    ///
    /// `matcher_info` is the segment_info dict for the current vim
    /// buffer/window (or None to use the `None` matcher slot).
    /// `matchers` is the caller-supplied list of `(matcher_key,
    /// matcher_fn)` pairs since Python's matcher closures
    /// (e.g. `powerline.matchers.vim.help`) aren't reachable from
    /// Rust at the lint-port level.
    ///
    /// If `matcher_info` is None, returns the theme registered under
    /// the empty matcher key (Python's `None` key at py:66-67).
    /// Otherwise walks the matchers + returns the first matching
    /// local theme; falls through to `self.theme` per py:72.
    pub fn get_theme<F>(
        &mut self,
        matcher_info: Option<&serde_json::Map<String, serde_json::Value>>,
        matchers: &[(&str, F)],
    ) -> serde_json::Value
    where
        F: Fn(&serde_json::Map<String, serde_json::Value>) -> bool,
    {
        // py:66-67  if matcher_info is None: return get_matched_theme(local_themes[None])
        let info = match matcher_info {
            Some(i) => i,
            None => {
                // py:67  self.local_themes[None] — Rust uses "" as the
                // None-key analog
                return self.get_matched_theme_cached("");
            }
        };
        // py:68-70  for matcher in local_themes.keys(): if matcher and matcher(info): return ...
        for (key, matcher_fn) in matchers {
            if !key.is_empty() && matcher_fn(info) && self.local_themes.contains_key(*key) {
                return self.get_matched_theme_cached(key);
            }
        }
        // py:72  return self.theme
        self.theme.clone()
    }

    /// Port of `VimRenderer.get_segment_info()` from
    /// `powerline/renderers/vim.py:85-86`.
    ///
    /// Returns the supplied `segment_info` when present, otherwise
    /// the renderer's default `self.segment_info`.
    pub fn get_segment_info(
        &self,
        segment_info: Option<&serde_json::Map<String, serde_json::Value>>,
        _mode: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        // py:86  return segment_info or self.segment_info
        match segment_info {
            Some(s) if !s.is_empty() => s.clone(),
            _ => self.segment_info.clone(),
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

    /// Port of `VimRenderer.strwidth()` from
    /// `powerline/renderers/vim.py:76-83`.
    ///
    /// Python wraps `vim.strwidth(string)` (a builtin VimL function
    /// that returns the display width in cells, accounting for
    /// wide-character and ambiguous-width handling per vim's
    /// `'ambiwidth'` option).
    ///
    /// Rust can't reach vim's strwidth without an RPC bridge; the
    /// port falls back to char-counting which matches strwidth for
    /// the ASCII subset used by every powerline highlight name.
    /// Callers needing exact vim parity should route through the
    /// existing strwidth in `Renderer` or wire a vim-RPC bridge.
    pub fn strwidth(string: &str) -> usize {
        // py:76  if sys.version_info < (3,):
        // py:77-79  vim.strwidth(string.encode(self.encoding, 'replace'))
        // py:80-83  Py3: vim.strwidth(string)
        string.chars().count()
    }

    /// Port of `VimRenderer.render()` from
    /// `powerline/renderers/vim.py:88-119`.
    ///
    /// Builds the segment_info dict and dispatches to the base
    /// Renderer.render with vim-specific keys (window, window_id,
    /// winnr) merged in. Python uses `vim.current.window` to detect
    /// the active window and pick the mode (`vim_mode()` or `'nc'`
    /// for inactive windows per py:92-96).
    ///
    /// Rust port can't reach vim.current; callers supply
    /// `is_current_window` + `mode` resolution as args. Returns the
    /// merged segment_info dict ready for base Renderer.render
    /// dispatch.
    pub fn render(
        renderer_segment_info: &serde_json::Map<String, serde_json::Value>,
        window: Option<i64>,
        window_id: Option<u64>,
        winnr: Option<i64>,
        is_tabline: bool,
        is_current_window: bool,
        current_mode_translation: Option<&str>,
    ) -> serde_json::Map<String, serde_json::Value> {
        // py:88  def render(self, window=None, window_id=None, winnr=None, is_tabline=False):
        // py:90  segment_info = self.segment_info.copy()
        let mut segment_info = renderer_segment_info.clone();
        // py:92-96  mode dispatch based on whether window is current
        let mode = if is_current_window {
            current_mode_translation.unwrap_or("nc").to_string()
        } else {
            "nc".to_string()
        };
        // py:97-99  inject (window, window_id, winnr, mode, is_tabline)
        if let Some(w) = window {
            segment_info.insert(
                "window".to_string(),
                serde_json::Value::Number(w.into()),
            );
        }
        if let Some(wid) = window_id {
            segment_info.insert(
                "window_id".to_string(),
                serde_json::Value::Number(wid.into()),
            );
        }
        if let Some(n) = winnr {
            segment_info.insert(
                "winnr".to_string(),
                serde_json::Value::Number(n.into()),
            );
        }
        segment_info.insert("mode".to_string(), serde_json::Value::String(mode));
        segment_info.insert(
            "is_tabline".to_string(),
            serde_json::Value::Bool(is_tabline),
        );
        segment_info
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
            // py:145  if not (fg, bg, attrs) in self.hl_groups:
            // py:146  hl_group = {
            // py:147  'ctermfg': 'NONE',
            // py:148  'guifg': None,
            // py:149  'ctermbg': 'NONE',
            // py:150  'guibg': None,
            // py:151  'attrs': ['NONE'],
            // py:152  'name': '',
            // py:153  }
            let mut g = HlGroup {
                ctermfg: "NONE".to_string(),
                guifg: None,
                ctermbg: "NONE".to_string(),
                guibg: None,
                attrs: vec!["NONE".to_string()],
                name: String::new(),
            };
            // py:154  if fg is not None and fg is not False:
            // py:155  hl_group['ctermfg'] = fg[0]
            // py:156  hl_group['guifg'] = fg[1]
            if let Some(f) = fg {
                g.ctermfg = f.cterm.to_string();
                g.guifg = f.truecolor;
            }
            // py:157  if bg is not None and bg is not False:
            // py:158  hl_group['ctermbg'] = bg[0]
            // py:159  hl_group['guibg'] = bg[1]
            if let Some(b) = bg {
                g.ctermbg = b.cterm.to_string();
                g.guibg = b.truecolor;
            }
            // py:160  if attrs:
            // py:161  hl_group['attrs'] = []
            // py:162  if attrs & ATTR_BOLD:
            // py:163  hl_group['attrs'].append('bold')
            // py:164  if attrs & ATTR_ITALIC:
            // py:165  hl_group['attrs'].append('italic')
            // py:166  if attrs & ATTR_UNDERLINE:
            // py:167  hl_group['attrs'].append('underline')
            if attrs != 0 {
                g.attrs = Self::attrs_to_hi_attrs(attrs);
            }
            // py:168  hl_group['name'] = (
            // py:169  'Pl_'
            // py:170  + str(hl_group['ctermfg']) + '_'
            // py:171  + str(hl_group['guifg']) + '_'
            // py:172  + str(hl_group['ctermbg']) + '_'
            // py:173  + str(hl_group['guibg']) + '_'
            // py:174  + ''.join(hl_group['attrs'])
            // py:175  )
            g.name = Self::build_group_name(&g);
            // py:176  self.hl_groups[(fg, bg, attrs)] = hl_group
            // py:177  vim.command('hi {group} ctermfg={ctermfg} guifg={guifg} ...'.format(...))
            commands.push(g.vim_command());
            self.hl_groups.insert(key, g);
        }
        // py:185  return '%#' + self.hl_groups[(fg, bg, attrs)]['name'] + '#'
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

    #[test]
    fn shutdown_records_main_theme_first() {
        // py:48
        let r = VimRenderer::new();
        r.shutdown();
        let log = r.shutdown_called.lock().unwrap();
        assert_eq!(log[0], "theme");
    }

    #[test]
    fn shutdown_walks_local_themes_with_theme_key() {
        // py:49-51  only matches WITH 'theme' key get shutdown
        let mut r = VimRenderer::new();
        let mut with_theme = serde_json::Map::new();
        with_theme.insert("theme".to_string(), serde_json::json!({}));
        let no_theme: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
        r.local_themes.insert("ready".to_string(), with_theme);
        r.local_themes.insert("not_ready".to_string(), no_theme);
        r.shutdown();
        let log = r.shutdown_called.lock().unwrap();
        assert!(log.contains(&"ready".to_string()));
        assert!(!log.contains(&"not_ready".to_string()));
    }

    #[test]
    fn add_local_theme_inserts_into_dict() {
        // py:53-56
        let mut r = VimRenderer::new();
        let mut theme = serde_json::Map::new();
        theme.insert("config".to_string(), serde_json::json!({"key": "value"}));
        let result = r.add_local_theme("help", theme);
        assert!(result.is_ok());
        assert!(r.local_themes.contains_key("help"));
    }

    #[test]
    fn add_local_theme_rejects_duplicate_matcher() {
        // py:54-55  raise KeyError
        let mut r = VimRenderer::new();
        let theme1 = serde_json::Map::new();
        let theme2 = serde_json::Map::new();
        assert!(r.add_local_theme("help", theme1).is_ok());
        let err = r.add_local_theme("help", theme2).unwrap_err();
        assert!(err.contains("already a local theme"));
    }

    #[test]
    fn get_matched_theme_returns_existing_theme() {
        // py:60  if 'theme' in match: return match['theme']
        let mut r = VimRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert("theme".to_string(), serde_json::json!({"name": "help"}));
        r.local_themes.insert("help".to_string(), entry);
        let t = r.get_matched_theme("help");
        assert_eq!(t["name"], "help");
    }

    #[test]
    fn get_matched_theme_constructs_from_config_when_missing() {
        // py:62  match['theme'] = Theme(theme_config=match['config'], ...)
        let mut r = VimRenderer::new();
        r.theme_config = serde_json::json!({"colorscheme": "default"});
        r.theme_kwargs
            .insert("extra".to_string(), serde_json::json!("kw_value"));
        let mut entry = serde_json::Map::new();
        entry.insert("config".to_string(), serde_json::json!({"segments": []}));
        r.local_themes.insert("help".to_string(), entry);

        let t = r.get_matched_theme("help");
        assert_eq!(t["theme_config"]["segments"], serde_json::json!([]));
        assert_eq!(t["main_theme_config"]["colorscheme"], "default");
        assert_eq!(t["theme_kwargs"]["extra"], "kw_value");
    }

    #[test]
    fn get_matched_theme_cached_back_inserts_theme_key() {
        // py:62  match['theme'] = ... (mutating cache-back)
        let mut r = VimRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert("config".to_string(), serde_json::json!({"a": 1}));
        r.local_themes.insert("help".to_string(), entry);

        let _ = r.get_matched_theme_cached("help");
        let cached = r
            .local_themes
            .get("help")
            .and_then(|m| m.get("theme"))
            .cloned();
        assert!(cached.is_some());
    }

    #[test]
    fn get_theme_none_matcher_uses_empty_key_local_theme() {
        // py:66-67  if matcher_info is None: return get_matched_theme(local_themes[None])
        let mut r = VimRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert("theme".to_string(), serde_json::json!({"name": "default"}));
        r.local_themes.insert("".to_string(), entry);

        let matchers: &[(
            &str,
            fn(&serde_json::Map<String, serde_json::Value>) -> bool,
        )] = &[];
        let t = r.get_theme(None, matchers);
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn get_theme_walks_matchers_and_returns_first_match() {
        // py:68-70
        let mut r = VimRenderer::new();
        let mut entry = serde_json::Map::new();
        entry.insert(
            "theme".to_string(),
            serde_json::json!({"name": "help-theme"}),
        );
        r.local_themes.insert("help".to_string(), entry);

        let mut info = serde_json::Map::new();
        info.insert("filetype".to_string(), serde_json::json!("help"));

        // Matcher 'help' fires when filetype == 'help'
        let matchers: Vec<(
            &str,
            fn(&serde_json::Map<String, serde_json::Value>) -> bool,
        )> = vec![("help", |i| {
            i.get("filetype").and_then(|v| v.as_str()) == Some("help")
        })];
        let t = r.get_theme(Some(&info), &matchers);
        assert_eq!(t["name"], "help-theme");
    }

    #[test]
    fn get_theme_no_matcher_match_falls_back_to_self_theme() {
        // py:72  return self.theme
        let mut r = VimRenderer::new();
        r.theme = serde_json::json!({"name": "default"});

        let info = serde_json::Map::new();
        let matchers: Vec<(
            &str,
            fn(&serde_json::Map<String, serde_json::Value>) -> bool,
        )> = vec![("never", |_| false)];
        let t = r.get_theme(Some(&info), &matchers);
        assert_eq!(t["name"], "default");
    }

    #[test]
    fn get_segment_info_returns_supplied_when_present() {
        // py:86  segment_info or self.segment_info
        let r = VimRenderer::new();
        let mut info = serde_json::Map::new();
        info.insert("buffer".to_string(), serde_json::json!("foo"));
        let result = r.get_segment_info(Some(&info), "n");
        assert_eq!(result["buffer"], "foo");
    }

    #[test]
    fn get_segment_info_falls_back_to_self_when_none() {
        let mut r = VimRenderer::new();
        r.segment_info
            .insert("default".to_string(), serde_json::json!("yes"));
        let result = r.get_segment_info(None, "n");
        assert_eq!(result["default"], "yes");
    }

    #[test]
    fn get_segment_info_falls_back_to_self_when_supplied_empty() {
        // py:86  empty dict is falsy in Python → falls back
        let mut r = VimRenderer::new();
        r.segment_info
            .insert("default".to_string(), serde_json::json!("yes"));
        let empty = serde_json::Map::new();
        let result = r.get_segment_info(Some(&empty), "n");
        assert_eq!(result["default"], "yes");
    }

    #[test]
    fn strwidth_counts_ascii_chars() {
        // py:80-83  vim.strwidth(string) — Rust port uses char count
        assert_eq!(VimRenderer::strwidth("hello"), 5);
        assert_eq!(VimRenderer::strwidth(""), 0);
    }

    #[test]
    fn render_returns_nc_mode_when_not_current_window() {
        // py:95-96  if window is not vim.current.window → mode = 'nc'
        let base = serde_json::Map::new();
        let result = VimRenderer::render(&base, Some(1), Some(42), Some(2), false, false, Some("n"));
        assert_eq!(
            result.get("mode"),
            Some(&serde_json::Value::String("nc".to_string()))
        );
    }

    #[test]
    fn render_uses_supplied_mode_for_current_window() {
        // py:92-94  if window is vim.current.window → mode = vim_mode()
        let base = serde_json::Map::new();
        let result = VimRenderer::render(&base, Some(1), Some(42), Some(2), false, true, Some("INS"));
        assert_eq!(
            result.get("mode"),
            Some(&serde_json::Value::String("INS".to_string()))
        );
    }

    #[test]
    fn render_injects_window_id_and_winnr_keys() {
        let base = serde_json::Map::new();
        let result = VimRenderer::render(&base, Some(7), Some(13), Some(2), true, false, None);
        assert_eq!(result.get("window_id").and_then(|v| v.as_u64()), Some(13));
        assert_eq!(result.get("winnr").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(
            result.get("is_tabline"),
            Some(&serde_json::Value::Bool(true))
        );
    }
}
