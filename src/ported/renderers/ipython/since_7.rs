// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/ipython/since_7.py`.
//!
//! IPython renderer for IPython >= 7.0 (prompt_toolkit 3 era). Emits
//! pygments-token segment pairs and tracks a process-wide
//! `used_styles` table of `(token_name, style_string)` pairs that
//! `PowerlinePromptStyle.style_rules` exposes to prompt_toolkit.
//!
//! Heavy parts ported: the synthetic `pl_a*_f*_b*` token-name
//! builder, the per-token style-rule emission, the `used_styles` /
//! `seen` module-level tables, the `hl()` segment builder, and the
//! attrs → name list. Bases (`IPythonRenderer`, `DynamicStyle`) and
//! `IPythonInfo` segment_info wrapping are deferred.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:1 (implicit)
// import operator                                                                    // py:2
// from functools import reduce                                                       // py:7
// from pygments.token import Token                                                   // py:9
// from prompt_toolkit.styles import DynamicStyle                                     // py:10
// from powerline.renderers.ipython import IPythonRenderer                            // py:12
// from powerline.ipython import IPythonInfo                                          // py:13
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE           // py:14

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Port of the `used_styles` module-level list at
/// `powerline/renderers/ipython/since_7.py:16`.
///
/// Tuples of (token_name, style_rule_string). Lives behind a Mutex
/// so the renderer can append from multiple call sites.
pub fn used_styles() -> &'static Mutex<Vec<(String, String)>> {
    static U: OnceLock<Mutex<Vec<(String, String)>>> = OnceLock::new();
    U.get_or_init(|| Mutex::new(Vec::new()))
}

/// Port of the `seen` module-level set at
/// `powerline/renderers/ipython/since_7.py:17`.
///
/// Token names already emitted into `used_styles`; dedupes
/// repeated `hl()` calls with the same style.
pub fn seen() -> &'static Mutex<HashSet<String>> {
    static S: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Color descriptor: `(cterm_index, optional_truecolor_rgb)`.
///
/// Same shape as renderers/tmux.rs and renderers/vim.rs. Matches
/// the Python `(cterm, hex_int)` tuple.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColorSpec {
    pub cterm: u16,
    pub truecolor: Option<u32>,
}

/// Port of `class PowerlinePromptStyle(DynamicStyle)` from
/// `powerline/renderers/ipython/since_7.py:19`.
pub struct PowerlinePromptStyle;

impl Default for PowerlinePromptStyle {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerlinePromptStyle {
    /// Constructs an empty PowerlinePromptStyle.
    pub fn new() -> Self {
        Self
    }

    /// Port of `PowerlinePromptStyle.style_rules` (property) from
    /// `powerline/renderers/ipython/since_7.py:22`.
    ///
    /// Python concatenates the base style's rules with `used_styles`.
    /// Rust port returns the `used_styles` rules; the base
    /// (DynamicStyle) is unported.
    pub fn style_rules(&self) -> Vec<(String, String)> {
        // py:23  (self.get_style() or self._dummy).style_rules + used_styles
        used_styles().lock().unwrap().clone()
    }

    /// Port of `PowerlinePromptStyle.invalidation_hash()` from
    /// `powerline/renderers/ipython/since_7.py:25`.
    ///
    /// Python yields `h + 1` for each h in the base hash; Rust port
    /// surfaces the structural intent (incremented hashes) by
    /// returning a Vec of u64 zeros + 1 since the base hash is
    /// unported.
    pub fn invalidation_hash(&self) -> Vec<u64> {
        // py:26  (h + 1 for h in tuple(super().invalidation_hash()))
        vec![1]
    }
}

/// Port of `class IPythonPygmentsRenderer(IPythonRenderer)` from
/// `powerline/renderers/ipython/since_7.py:29`.
pub struct IPythonPygmentsRenderer {
    /// Python: `self.character_translations` — inherited from
    /// IPythonRenderer base. py:33 overrides the entry for `' '`.
    pub character_translations: std::collections::HashMap<char, String>,
}

impl Default for IPythonPygmentsRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonPygmentsRenderer {
    /// Port of `IPythonPygmentsRenderer.reduce_initial` from
    /// `powerline/renderers/ipython/since_7.py:30`.
    pub const REDUCE_INITIAL: [(); 0] = [];

    /// Port of `IPythonPygmentsRenderer.__init__()` from
    /// `powerline/renderers/ipython/since_7.py:32`.
    pub fn new() -> Self {
        // py:33-34  character_translations[ord(' ')] = ' '
        let mut character_translations = std::collections::HashMap::new();
        character_translations.insert(' ', " ".to_string());
        Self {
            character_translations,
        }
    }

    /// Port of `IPythonPygmentsRenderer.hl_join()` (staticmethod) from
    /// `powerline/renderers/ipython/since_7.py:40`.
    ///
    /// `reduce(operator.iadd, segments, [])` — flattens a list of
    /// segment-pair lists into a single list.
    pub fn hl_join<T: Clone>(segments: &[Vec<T>]) -> Vec<T> {
        // py:41  reduce(operator.iadd, segments, [])
        let mut out: Vec<T> = Vec::new();
        for s in segments {
            out.extend_from_slice(s);
        }
        out
    }

    /// Builds the attrs name list (`'bold'`/`'italic'`/`'underline'`)
    /// from a powerline ATTR_* bitfield per py:55-60.
    pub fn attrs_to_attr_names(attrs: u32) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        // py:56-57  ATTR_BOLD
        if attrs & ATTR_BOLD != 0 {
            out.push("bold".to_string());
        }
        // py:58-59  ATTR_ITALIC
        if attrs & ATTR_ITALIC != 0 {
            out.push("italic".to_string());
        }
        // py:60-61  ATTR_UNDERLINE
        if attrs & ATTR_UNDERLINE != 0 {
            out.push("underline".to_string());
        }
        out
    }

    /// Builds the synthetic `pl_a<attr>_f<rrggbb>_b<rrggbb>` token
    /// name from a (fg, bg, attrs) triple per py:62-67.
    pub fn build_token_name(fg: Option<ColorSpec>, bg: Option<ColorSpec>, attrs: u32) -> String {
        let guifg = fg.and_then(|f| f.truecolor);
        let guibg = bg.and_then(|b| b.truecolor);
        let att = Self::attrs_to_attr_names(attrs);
        let fg_hex = guifg.map(|v| format!("{:06x}", v)).unwrap_or_default();
        let bg_hex = guibg.map(|v| format!("{:06x}", v)).unwrap_or_default();
        // py:62-67  'pl' + _a<attr>... + _f<rrggbb> + _b<rrggbb>
        let mut name = String::from("pl");
        for a in &att {
            name.push_str("_a");
            name.push_str(a);
        }
        name.push_str("_f");
        name.push_str(&fg_hex);
        name.push_str("_b");
        name.push_str(&bg_hex);
        name
    }

    /// Builds the style-rule string accompanying the token name per
    /// py:73-77: leading-space joined attr names + ` fg:#<rrggbb>` (or
    /// just ` fg:`) + ` bg:#<rrggbb>` (or just ` bg:`).
    pub fn build_style_rule(fg: Option<ColorSpec>, bg: Option<ColorSpec>, attrs: u32) -> String {
        let guifg = fg.and_then(|f| f.truecolor);
        let guibg = bg.and_then(|b| b.truecolor);
        let att = Self::attrs_to_attr_names(attrs);
        let fg_hex = guifg.map(|v| format!("{:06x}", v)).unwrap_or_default();
        let bg_hex = guibg.map(|v| format!("{:06x}", v)).unwrap_or_default();
        let mut s = String::new();
        // py:73  ''.join((' ' + attr for attr in att))
        for a in &att {
            s.push(' ');
            s.push_str(a);
        }
        // py:74-75  ' fg:#' + fg if fg != '' else ' fg:'
        if !fg_hex.is_empty() {
            s.push_str(" fg:#");
            s.push_str(&fg_hex);
        } else {
            s.push_str(" fg:");
        }
        // py:76-77  ' bg:#' + bg if bg != '' else ' bg:'
        if !bg_hex.is_empty() {
            s.push_str(" bg:#");
            s.push_str(&bg_hex);
        } else {
            s.push_str(" bg:");
        }
        s
    }

    /// Port of `IPythonPygmentsRenderer.hl()` from
    /// `powerline/renderers/ipython/since_7.py:43`.
    ///
    /// Emits a single (token_name_tuple, contents) pair. On first
    /// emission for a (fg, bg, attrs) combination, registers a
    /// `pygments.<name>` style rule into `used_styles`.
    pub fn hl(
        &self,
        escaped_contents: &str,
        fg: Option<ColorSpec>,
        bg: Option<ColorSpec>,
        attrs: Option<u32>,
    ) -> Vec<(Vec<String>, String)> {
        let attrs = attrs.unwrap_or(0);
        let name = Self::build_token_name(fg, bg, attrs);
        // py:79-83  global seen; if not (name in seen): register style rule
        let mut seen_lock = seen().lock().unwrap();
        if !seen_lock.contains(&name) {
            let rule = Self::build_style_rule(fg, bg, attrs);
            used_styles()
                .lock()
                .unwrap()
                .push((format!("pygments.{}", name), rule));
            seen_lock.insert(name.clone());
        }
        drop(seen_lock);
        // py:84  return [((name,), escaped_contents)]
        vec![(vec![name], escaped_contents.to_string())]
    }

    /// Port of `IPythonPygmentsRenderer.hlstyle()` from
    /// `powerline/renderers/ipython/since_7.py:86`.
    pub fn hlstyle(&self) -> Vec<()> {
        // py:87  return []
        Vec::new()
    }

    /// Port of `IPythonPygmentsRenderer.get_client_id()` from
    /// `powerline/renderers/ipython/since_7.py:89`.
    ///
    /// Python returns `id(self)` — the object's memory address.
    /// Rust port returns a stable id derived from the renderer's
    /// address.
    pub fn get_client_id(&self) -> usize {
        self as *const _ as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that mutate the process-wide `used_styles` /
    /// `seen` globals. Returns a guard the test holds across the
    /// global-state manipulation; without it, cargo's parallel test
    /// runner interleaves writes to those Mutexes and breaks
    /// count-based assertions (poisons inner Mutexes on panic).
    ///
    /// Note: written as a macro rather than a fn returning
    /// MutexGuard<'static, ()> because the drift-gate brace counter
    /// in tests/ported_fn_names_match_py.rs mis-parses the `'static`
    /// lifetime as an opening char literal and prematurely closes
    /// the test-scope, causing every test fn to be flagged as
    /// "invented". Macro expansion sidesteps the issue.
    macro_rules! lock_globals {
        () => {{
            static L: OnceLock<Mutex<()>> = OnceLock::new();
            L.get_or_init(|| Mutex::new(()))
                .lock()
                .unwrap_or_else(|e| e.into_inner())
        }};
    }

    fn reset_globals() {
        // PoisonError-safe: previous panic may have left the inner
        // Mutex poisoned; take into_inner to recover.
        let mut u = used_styles().lock().unwrap_or_else(|e| e.into_inner());
        u.clear();
        let mut s = seen().lock().unwrap_or_else(|e| e.into_inner());
        s.clear();
    }

    #[test]
    fn used_styles_starts_empty_after_reset() {
        let _g = lock_globals!();
        reset_globals();
        assert!(used_styles().lock().unwrap().is_empty());
        assert!(seen().lock().unwrap().is_empty());
    }

    #[test]
    fn ipython_pygments_renderer_init_translates_space() {
        // py:33-34  character_translations[ord(' ')] = ' '
        let r = IPythonPygmentsRenderer::new();
        assert_eq!(r.character_translations.get(&' '), Some(&" ".to_string()));
    }

    #[test]
    fn hl_join_flattens_segments() {
        let segs = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
        let flat = IPythonPygmentsRenderer::hl_join(&segs);
        assert_eq!(flat, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn hl_join_empty_returns_empty() {
        let segs: Vec<Vec<i32>> = Vec::new();
        let flat = IPythonPygmentsRenderer::hl_join(&segs);
        assert!(flat.is_empty());
    }

    #[test]
    fn attrs_to_attr_names_zero_is_empty() {
        assert!(IPythonPygmentsRenderer::attrs_to_attr_names(0).is_empty());
    }

    #[test]
    fn attrs_to_attr_names_all_three() {
        let r =
            IPythonPygmentsRenderer::attrs_to_attr_names(ATTR_BOLD | ATTR_ITALIC | ATTR_UNDERLINE);
        assert_eq!(r, vec!["bold", "italic", "underline"]);
    }

    #[test]
    fn build_token_name_with_fg_bg_attrs() {
        let n = IPythonPygmentsRenderer::build_token_name(
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffaabb),
            }),
            Some(ColorSpec {
                cterm: 21,
                truecolor: Some(0x0000ff),
            }),
            ATTR_BOLD,
        );
        // py:62-67  'pl' + _abold + _fffaabb + _b0000ff
        assert_eq!(n, "pl_abold_fffaabb_b0000ff");
    }

    #[test]
    fn build_token_name_with_no_truecolor_emits_empty_hex_segments() {
        let n = IPythonPygmentsRenderer::build_token_name(None, None, 0);
        // py:62-67  'pl' + '' (no attrs) + '_f' + '' + '_b' + ''
        assert_eq!(n, "pl_f_b");
    }

    #[test]
    fn build_style_rule_includes_attrs_fg_bg() {
        let r = IPythonPygmentsRenderer::build_style_rule(
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffffff),
            }),
            Some(ColorSpec {
                cterm: 21,
                truecolor: Some(0x0000ff),
            }),
            ATTR_BOLD,
        );
        // py:73-77  ' bold' + ' fg:#ffffff' + ' bg:#0000ff'
        assert_eq!(r, " bold fg:#ffffff bg:#0000ff");
    }

    #[test]
    fn build_style_rule_no_colors_emits_bare_directives() {
        // py:74-77  ' fg:' / ' bg:'  when no color
        let r = IPythonPygmentsRenderer::build_style_rule(None, None, 0);
        assert_eq!(r, " fg: bg:");
    }

    #[test]
    fn hl_emits_token_name_and_contents() {
        let _g = lock_globals!();
        reset_globals();
        let r = IPythonPygmentsRenderer::new();
        let out = r.hl(
            "hello",
            Some(ColorSpec {
                cterm: 231,
                truecolor: Some(0xffffff),
            }),
            None,
            None,
        );
        assert_eq!(out.len(), 1);
        let (names, contents) = &out[0];
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("pl_f"));
        assert_eq!(contents, "hello");
    }

    #[test]
    fn hl_registers_style_rule_on_first_call() {
        let _g = lock_globals!();
        reset_globals();
        let r = IPythonPygmentsRenderer::new();
        let _ = r.hl(
            "x",
            Some(ColorSpec {
                cterm: 21,
                truecolor: Some(0x010203),
            }),
            None,
            Some(ATTR_BOLD),
        );
        let used = used_styles().lock().unwrap().clone();
        assert_eq!(used.len(), 1);
        assert!(used[0].0.starts_with("pygments.pl_abold_f010203"));
        assert!(used[0].1.contains("bold"));
        assert!(used[0].1.contains("fg:#010203"));
    }

    #[test]
    fn hl_deduplicates_repeated_token_name() {
        // py:79-83  if name not in seen: append. So repeated same style → 1 entry.
        let _g = lock_globals!();
        reset_globals();
        let r = IPythonPygmentsRenderer::new();
        let spec = Some(ColorSpec {
            cterm: 1,
            truecolor: Some(0x010101),
        });
        let _ = r.hl("a", spec, None, None);
        let _ = r.hl("b", spec, None, None);
        let _ = r.hl("c", spec, None, None);
        assert_eq!(used_styles().lock().unwrap().len(), 1);
    }

    #[test]
    fn hl_emits_distinct_token_names_for_different_styles() {
        let _g = lock_globals!();
        reset_globals();
        let r = IPythonPygmentsRenderer::new();
        let _ = r.hl(
            "x",
            Some(ColorSpec {
                cterm: 21,
                truecolor: Some(0x111111),
            }),
            None,
            None,
        );
        let _ = r.hl(
            "y",
            Some(ColorSpec {
                cterm: 22,
                truecolor: Some(0x222222),
            }),
            None,
            None,
        );
        assert_eq!(used_styles().lock().unwrap().len(), 2);
    }

    #[test]
    fn hlstyle_returns_empty() {
        let r = IPythonPygmentsRenderer::new();
        assert!(r.hlstyle().is_empty());
    }

    #[test]
    fn get_client_id_is_stable_for_same_instance() {
        let r = IPythonPygmentsRenderer::new();
        let a = r.get_client_id();
        let b = r.get_client_id();
        assert_eq!(a, b);
    }

    #[test]
    fn powerline_prompt_style_style_rules_returns_used_styles_snapshot() {
        let _g = lock_globals!();
        reset_globals();
        used_styles()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(("pygments.pl_f_b".to_string(), " fg: bg:".to_string()));
        let p = PowerlinePromptStyle::new();
        let rules = p.style_rules();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].0, "pygments.pl_f_b");
    }

    #[test]
    fn powerline_prompt_style_invalidation_hash_returns_non_empty() {
        let p = PowerlinePromptStyle::new();
        let h = p.invalidation_hash();
        assert!(!h.is_empty());
    }
}
