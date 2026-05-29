// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/ipython/since_5.py`.
//!
//! IPython 5.0-6.x renderer (prompt_toolkit 1.x era). Structurally
//! parallel to `renderers/ipython/since_7.rs` with two deltas:
//!   1. Token-name prefix is `'Pl'` (capital) instead of `'pl'`
//!   2. RGB hex is formatted via `%6x` (space-padded) instead of
//!      `%06x` (zero-padded) per py:92-93 vs since_7 py:62-63
//!
//! The PowerlineStyleDict + PowerlinePromptStyle classes are heavier
//! since they tie into pygments token-tree attribute walking; the
//! Rust port surfaces the pure pieces (attrs name list builder,
//! token name builder, hl segment builder).

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import operator                                  // py:4
// from collections import defaultdict              // py:6
// from functools import reduce                     // py:11
// from pygments.token import Token                 // py:13
// from prompt_toolkit.styles import DynamicStyle, Attrs                                    // py:14
// from powerline.renderers.ipython import IPythonRenderer                                  // py:16
// from powerline.ipython import IPythonInfo        // py:17
// from powerline.colorscheme import ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE                 // py:18

use crate::ported::colorscheme::{ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE};
use crate::ported::renderers::ipython::since_7::ColorSpec;
use std::collections::HashMap;

/// Port of `IPythonPygmentsRenderer.reduce_initial` from
/// `powerline/renderers/ipython/since_5.py:84`.
pub const REDUCE_INITIAL: [(); 0] = [];

/// Builds the attrs name list (`'bold'`/`'italic'`/`'underline'`)
/// from a powerline ATTR_* bitfield per py:107-112.
pub fn attrs_to_attr_names(attrs: u32) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    // py:108-109  ATTR_BOLD
    if attrs & ATTR_BOLD != 0 {
        out.push("bold".to_string());
    }
    // py:110-111  ATTR_ITALIC
    if attrs & ATTR_ITALIC != 0 {
        out.push("italic".to_string());
    }
    // py:112-113  ATTR_UNDERLINE
    if attrs & ATTR_UNDERLINE != 0 {
        out.push("underline".to_string());
    }
    out
}

/// Builds the synthetic `Pl_a<attr>_f<rgb>_b<rgb>` token name from a
/// `(fg, bg, attrs)` triple per `powerline/renderers/ipython/
/// since_5.py:114-120`.
///
/// Note: the Python source uses `'_f%6x' % guifg` and `'_b%6x' %
/// guibg` — `%6x` is space-padded (not zero-padded) to a minimum
/// width of 6 chars. Compare with since_7's `%06x` zero-padded.
pub fn build_token_name(fg: Option<ColorSpec>, bg: Option<ColorSpec>, attrs: u32) -> String {
    let guifg = fg.and_then(|f| f.truecolor);
    let guibg = bg.and_then(|b| b.truecolor);
    let att = attrs_to_attr_names(attrs);
    // py:114-120  'Pl' + _a<attr>... + _f<6x> + _b<6x>
    let mut name = String::from("Pl");
    for a in &att {
        name.push_str("_a");
        name.push_str(a);
    }
    if let Some(rgb) = guifg {
        name.push_str(&format!("_f{:6x}", rgb));
    }
    if let Some(rgb) = guibg {
        name.push_str(&format!("_b{:6x}", rgb));
    }
    name
}

/// Port of `IPythonPygmentsRenderer.hl()` from
/// `powerline/renderers/ipython/since_5.py:93`.
///
/// Returns a list of `(token_name, contents)` pairs. Unlike the
/// since_7 variant this returns the token path as a single string
/// rather than a tuple of names (`Pl_..._f...` rather than the
/// `(name,)` 1-tuple).
pub fn hl(
    contents: &str,
    fg: Option<ColorSpec>,
    bg: Option<ColorSpec>,
    attrs: Option<u32>,
) -> Vec<(String, String)> {
    let attrs = attrs.unwrap_or(0);
    let name = build_token_name(fg, bg, attrs);
    // py:122  return [(getattr(Token.Generic.Prompt.Powerline, name), contents)]
    vec![(name, contents.to_string())]
}

/// Port of `IPythonPygmentsRenderer.hlstyle()` from
/// `powerline/renderers/ipython/since_5.py:125`.
pub fn hlstyle() -> Vec<()> {
    // py:126  return []
    Vec::new()
}

/// Port of `IPythonPygmentsRenderer.hl_join()` (staticmethod) from
/// `powerline/renderers/ipython/since_5.py:89`.
pub fn hl_join<T: Clone>(segments: &[Vec<T>]) -> Vec<T> {
    // py:90  reduce(operator.iadd, segments, [])
    let mut out: Vec<T> = Vec::new();
    for s in segments {
        out.extend_from_slice(s);
    }
    out
}

/// Port of `class PowerlineStyleDict(defaultdict)` from
/// `powerline/renderers/ipython/since_5.py:26`.
///
/// Defaultdict-backed style lookup. Python uses
/// `defaultdict.__missing__` to call `self.missing_func(key)` on a
/// miss; Rust port surfaces a `lookup(key, fallback)` helper.
pub struct PowerlineStyleDict {
    pub inner: HashMap<String, Vec<(String, String)>>,
}

impl Default for PowerlineStyleDict {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerlineStyleDict {
    /// Port of `PowerlineStyleDict.__init__()` from
    /// `powerline/renderers/ipython/since_5.py:31`.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Port of `PowerlineStyleDict.__missing__()` from
    /// `powerline/renderers/ipython/since_5.py:35`.
    ///
    /// `missing_func` is the caller's closure that produces the
    /// default value for a missing key.
    pub fn lookup<F>(&self, key: &str, missing_func: F) -> Vec<(String, String)>
    where
        F: FnOnce(&str) -> Vec<(String, String)>,
    {
        match self.inner.get(key) {
            Some(v) => v.clone(),
            None => missing_func(key),
        }
    }

    /// Inserts a (key, value) pair into the dict.
    pub fn insert(&mut self, key: impl Into<String>, value: Vec<(String, String)>) {
        self.inner.insert(key.into(), value);
    }
}

/// Port of `PowerlinePromptStyle.invalidation_hash()` from
/// `powerline/renderers/ipython/since_5.py:79-80`.
///
/// Python: `super().invalidation_hash() + 1` — bumps the base
/// hash by 1.
pub fn powerline_prompt_style_invalidation_hash(base: u64) -> u64 {
    base + 1
}

/// Port of `PowerlinePromptStyle.get_attrs_for_token()` parsing
/// path from `powerline/renderers/ipython/since_5.py:40-65`.
///
/// Parses the synthetic `Pl_a<attr>_f<hex>_b<hex>` token-tail name
/// back into (color, bgcolor, attrs) per py:55-65. Returns
/// `None` when the token name isn't a Powerline-format name.
pub fn parse_token_attrs(name: &str) -> Option<TokenAttrs> {
    // py:56-65  iterate over '_'-split props
    if !name.starts_with("Pl") || name == "Pl" {
        return None;
    }
    let mut attrs = TokenAttrs::default();
    // py:57  for prop in token[-1][3:].split('_')
    let body = name.strip_prefix("Pl").unwrap_or(name);
    for prop in body.split('_') {
        if prop.is_empty() {
            continue;
        }
        let first = prop.as_bytes()[0] as char;
        let rest = &prop[1..];
        match first {
            // py:58-59  prop[0] == 'a' → ret[prop[1:]] = True
            'a' => match rest {
                "bold" => attrs.bold = true,
                "italic" => attrs.italic = true,
                "underline" => attrs.underline = true,
                _ => {}
            },
            // py:60-61  prop[0] == 'f' → color = prop[1:]
            'f' => attrs.color = Some(rest.trim().to_string()),
            // py:62-63  prop[0] == 'b' → bgcolor = prop[1:]
            'b' => attrs.bgcolor = Some(rest.trim().to_string()),
            _ => {}
        }
    }
    Some(attrs)
}

/// Port of the `Attrs(...)` named tuple returned by
/// `PowerlinePromptStyle.get_attrs_for_token` at py:65.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenAttrs {
    pub color: Option<String>,
    pub bgcolor: Option<String>,
    pub bold: bool,
    pub underline: bool,
    pub italic: bool,
    pub reverse: bool,
    pub blink: bool,
}

/// Port of `class PowerlinePromptStyle(DynamicStyle)` from
/// `powerline/renderers/ipython/since_5.py:40`.
///
/// Marker struct that owns the dispatch for the token-attrs lookup.
/// The underlying `DynamicStyle` from prompt_toolkit isn't reachable
/// from Rust; the Rust port captures the `base_get_attrs_for_token`
/// closure so callers can wire in the super-class fallback at py:48.
pub struct PowerlinePromptStyle;

impl PowerlinePromptStyle {
    /// Port of `PowerlinePromptStyle.get_attrs_for_token()` from
    /// `powerline/renderers/ipython/since_5.py:41-65`.
    ///
    /// Returns the parsed `TokenAttrs` for a Powerline-format token
    /// per py:49-65. Returns None for non-Powerline tokens per
    /// py:42-48 (super delegation point); callers pass the base
    /// implementation result through their own dispatch.
    pub fn get_attrs_for_token(token: &str) -> Option<TokenAttrs> {
        // py:42-47  token not in PowerlinePromptToken / wrong length /
        //          missing Pl prefix / equal to 'Pl' → super delegation
        parse_token_attrs(token)
    }

    /// Port of `PowerlinePromptStyle.invalidation_hash()` from
    /// `powerline/renderers/ipython/since_5.py:78-79`.
    ///
    /// Delegates to the standalone helper for the
    /// `super().invalidation_hash() + 1` math.
    pub fn invalidation_hash(base: u64) -> u64 {
        powerline_prompt_style_invalidation_hash(base)
    }

    /// Port of `PowerlinePromptStyle.get_token_to_attributes_dict()`
    /// from `powerline/renderers/ipython/since_5.py:67-76`.
    ///
    /// Returns a `PowerlineStyleDict` initialised with a `fallback`
    /// closure that dispatches to `get_attrs_for_token`. The Rust
    /// port returns the `PowerlineStyleDict` populated with the
    /// supplied seed entries; callers query via `lookup(key, ...)`.
    pub fn get_token_to_attributes_dict(
        seed: HashMap<String, Vec<(String, String)>>,
    ) -> PowerlineStyleDict {
        // py:68-76  defaultdict-like dict with fallback
        let mut d = PowerlineStyleDict::new();
        d.inner = seed;
        d
    }
}

/// Port of `class IPythonPygmentsRenderer(IPythonRenderer)` from
/// `powerline/renderers/ipython/since_5.py:82`.
///
/// Marker struct holding the renderer state. The actual rendering
/// dispatch chains through `IPythonRenderer` (already ported) — the
/// Rust port surfaces the IPython-Pygments-specific methods + the
/// `id(self)` client-id stand-in.
pub struct IPythonPygmentsRenderer {
    /// Stable per-instance ID used as the cache key by daemon-mode
    /// renderer dispatch per py:127.
    pub id: u64,
}

impl IPythonPygmentsRenderer {
    /// Construct with a fresh ID. Each call increments a global
    /// counter so distinct instances get distinct IDs (matching
    /// Python's `id(self)` semantics — every fresh object has a
    /// distinct integer identity).
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self {
            id: COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }

    /// Port of `IPythonPygmentsRenderer.get_segment_info()` from
    /// `powerline/renderers/ipython/since_5.py:85-87`.
    ///
    /// Wraps `segment_info` with `IPythonInfo(...)` and delegates to
    /// the parent `IPythonRenderer.get_segment_info`. The Rust port
    /// returns the merged dict directly — caller wires it into the
    /// parent IPythonRenderer.
    pub fn get_segment_info(
        &self,
        segment_info: &serde_json::Value,
    ) -> serde_json::Map<String, serde_json::Value> {
        // py:86-87  super().get_segment_info(IPythonInfo(segment_info), mode)
        let mut r = serde_json::Map::new();
        r.insert("ipython".to_string(), segment_info.clone());
        r
    }

    /// Port of `IPythonPygmentsRenderer.get_client_id()` from
    /// `powerline/renderers/ipython/since_5.py:126-127`.
    ///
    /// Python: `return id(self)` — each instance's stable identity.
    /// The Rust port returns the renderer's `id` field.
    pub fn get_client_id(&self) -> u64 {
        // py:127  return id(self)
        self.id
    }
}

impl Default for IPythonPygmentsRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attrs_to_attr_names_zero_is_empty() {
        assert!(attrs_to_attr_names(0).is_empty());
    }

    #[test]
    fn attrs_to_attr_names_bold() {
        assert_eq!(attrs_to_attr_names(ATTR_BOLD), vec!["bold"]);
    }

    #[test]
    fn attrs_to_attr_names_italic() {
        assert_eq!(attrs_to_attr_names(ATTR_ITALIC), vec!["italic"]);
    }

    #[test]
    fn attrs_to_attr_names_all_three() {
        let r = attrs_to_attr_names(ATTR_BOLD | ATTR_ITALIC | ATTR_UNDERLINE);
        assert_eq!(r, vec!["bold", "italic", "underline"]);
    }

    #[test]
    fn build_token_name_capital_pl_prefix() {
        // py:114  'Pl' (capital) — distinguishes from since_7's 'pl'
        let n = build_token_name(None, None, 0);
        assert_eq!(n, "Pl");
    }

    #[test]
    fn build_token_name_with_fg_uses_space_padded_hex() {
        // py:117  '_f%6x' % guifg — %6x is space-padded
        let n = build_token_name(
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0xffaabb),
            }),
            None,
            0,
        );
        // 0xffaabb = 16755387 → 6 hex chars, fits exactly, no padding
        assert_eq!(n, "Pl_fffaabb");
    }

    #[test]
    fn build_token_name_with_small_fg_pads_with_space() {
        // 0xa = 1 char "a", padded to 6 with leading spaces → "     a"
        let n = build_token_name(
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0xa),
            }),
            None,
            0,
        );
        assert_eq!(n, "Pl_f     a");
    }

    #[test]
    fn build_token_name_with_bg() {
        let n = build_token_name(
            None,
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0x0000ff),
            }),
            0,
        );
        // 0xff → "ff" padded to "    ff"
        assert_eq!(n, "Pl_b    ff");
    }

    #[test]
    fn build_token_name_with_attrs() {
        let n = build_token_name(None, None, ATTR_BOLD);
        assert_eq!(n, "Pl_abold");
    }

    #[test]
    fn build_token_name_with_all_three() {
        let n = build_token_name(
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0xffaabb),
            }),
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0x0000ff),
            }),
            ATTR_BOLD | ATTR_UNDERLINE,
        );
        assert_eq!(n, "Pl_abold_aunderline_fffaabb_b    ff");
    }

    #[test]
    fn hl_returns_single_segment_pair() {
        // py:122  return [(token, contents)]
        let r = hl("hello", None, None, None);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, "Pl");
        assert_eq!(r[0].1, "hello");
    }

    #[test]
    fn hl_uses_token_name_with_color() {
        let r = hl(
            "x",
            Some(ColorSpec {
                cterm: 0,
                truecolor: Some(0xff0000),
            }),
            None,
            None,
        );
        assert_eq!(r[0].0, "Pl_fff0000");
    }

    #[test]
    fn hlstyle_returns_empty() {
        // py:125-126  return []
        assert!(hlstyle().is_empty());
    }

    #[test]
    fn hl_join_flattens_segments() {
        // py:89-90  reduce(operator.iadd, segments, [])
        let segs = vec![vec![1, 2], vec![3], vec![4, 5, 6]];
        let flat = hl_join(&segs);
        assert_eq!(flat, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn powerline_style_dict_lookup_returns_existing() {
        let mut d = PowerlineStyleDict::new();
        d.insert("foo", vec![("k".to_string(), "v".to_string())]);
        let r = d.lookup("foo", |_| Vec::new());
        assert_eq!(r, vec![("k".to_string(), "v".to_string())]);
    }

    #[test]
    fn powerline_style_dict_lookup_calls_missing_func_on_miss() {
        // py:35  __missing__: self.missing_func(key)
        let d = PowerlineStyleDict::new();
        let mut called_with = String::new();
        let _ = d.lookup("missing", |key| {
            called_with = key.to_string();
            vec![("fallback".to_string(), "yes".to_string())]
        });
        assert_eq!(called_with, "missing");
    }

    #[test]
    fn powerline_prompt_style_invalidation_hash_adds_one() {
        // py:79-80  return super().invalidation_hash() + 1
        assert_eq!(powerline_prompt_style_invalidation_hash(0), 1);
        assert_eq!(powerline_prompt_style_invalidation_hash(42), 43);
    }

    #[test]
    fn parse_token_attrs_plain_pl_returns_default() {
        // "Pl" alone → not a Powerline format name per py:43-44
        let r = parse_token_attrs("Pl");
        assert!(r.is_none());
    }

    #[test]
    fn parse_token_attrs_non_pl_returns_none() {
        let r = parse_token_attrs("Generic");
        assert!(r.is_none());
    }

    #[test]
    fn parse_token_attrs_bold() {
        // py:58-59  'a<attr>' → ret[attr] = True
        let r = parse_token_attrs("Pl_abold").unwrap();
        assert!(r.bold);
        assert!(!r.italic);
        assert!(!r.underline);
    }

    #[test]
    fn parse_token_attrs_fg_color() {
        // py:60-61  'f<hex>' → color
        let r = parse_token_attrs("Pl_fffaabb").unwrap();
        assert_eq!(r.color.as_deref(), Some("ffaabb"));
        assert!(r.bgcolor.is_none());
    }

    #[test]
    fn parse_token_attrs_bg_color() {
        // py:62-63  'b<hex>' → bgcolor
        let r = parse_token_attrs("Pl_b0000ff").unwrap();
        assert_eq!(r.bgcolor.as_deref(), Some("0000ff"));
    }

    #[test]
    fn parse_token_attrs_all_three() {
        let r = parse_token_attrs("Pl_abold_aunderline_fffaabb_b0000ff").unwrap();
        assert!(r.bold);
        assert!(r.underline);
        assert!(!r.italic);
        assert_eq!(r.color.as_deref(), Some("ffaabb"));
        assert_eq!(r.bgcolor.as_deref(), Some("0000ff"));
    }

    #[test]
    fn parse_token_attrs_default_reverse_and_blink_false() {
        // py:51-55  initial dict has reverse: False, blink: False
        let r = parse_token_attrs("Pl_abold").unwrap();
        assert!(!r.reverse);
        assert!(!r.blink);
    }

    #[test]
    fn reduce_initial_const_is_empty_array() {
        assert_eq!(REDUCE_INITIAL.len(), 0);
    }

    #[test]
    fn powerline_prompt_style_get_attrs_for_token_pl_format() {
        // py:42-65  Powerline token → TokenAttrs
        let r = PowerlinePromptStyle::get_attrs_for_token("Pl_abold_fff0000").unwrap();
        assert!(r.bold);
        assert_eq!(r.color.as_deref(), Some("ff0000"));
    }

    #[test]
    fn powerline_prompt_style_get_attrs_for_token_non_pl_returns_none() {
        // py:42-47  non-Powerline tokens delegated to super
        assert!(PowerlinePromptStyle::get_attrs_for_token("Generic").is_none());
    }

    #[test]
    fn powerline_prompt_style_invalidation_hash_bumps_by_one() {
        // py:79
        assert_eq!(PowerlinePromptStyle::invalidation_hash(10), 11);
    }

    #[test]
    fn powerline_prompt_style_get_token_to_attributes_dict_returns_dict() {
        // py:67-76
        let mut seed = HashMap::new();
        seed.insert(
            "Pl_abold".to_string(),
            vec![("k".to_string(), "v".to_string())],
        );
        let d = PowerlinePromptStyle::get_token_to_attributes_dict(seed);
        let r = d.lookup("Pl_abold", |_| Vec::new());
        assert_eq!(r, vec![("k".to_string(), "v".to_string())]);
    }

    #[test]
    fn ipython_pygments_renderer_each_instance_gets_unique_id() {
        // py:127  id(self) — distinct per instance
        let r1 = IPythonPygmentsRenderer::new();
        let r2 = IPythonPygmentsRenderer::new();
        assert_ne!(r1.id, r2.id);
        assert_ne!(r1.get_client_id(), r2.get_client_id());
    }

    #[test]
    fn ipython_pygments_renderer_get_client_id_returns_self_id() {
        let r = IPythonPygmentsRenderer::new();
        assert_eq!(r.get_client_id(), r.id);
    }

    #[test]
    fn ipython_pygments_renderer_get_segment_info_wraps_payload_under_ipython_key() {
        // py:86-87  super().get_segment_info(IPythonInfo(segment_info), mode)
        let r = IPythonPygmentsRenderer::new();
        let payload = serde_json::json!({"prompt_count": 5});
        let merged = r.get_segment_info(&payload);
        assert_eq!(merged["ipython"]["prompt_count"], 5);
    }
}
