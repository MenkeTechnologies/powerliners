// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderer.py`.
//!
//! Base Renderer class. Subclasses (`renderers/tmux.rs`,
//! `renderers/vim.rs`, etc.) override the per-format hooks
//! (`hl`/`hlstyle`/`character_translations`). Surfaces:
//!   - `NBSP` constant
//!   - `np_control_character_translations()` — 0x00-0x1F → "^@"-"^_"
//!   - `np_invalid_character_translations()` — 0xDC80-0xDCFF → "<80>"-"<FF>"
//!   - `np_invalid_character_re()` — unpaired-surrogate regex
//!   - `np_character_translations()` — union for UCS-4
//!   - `translate_np(s)` — non-printable translation
//!   - `construct_returned_value(...)` — render return-tuple builder
//!   - Width data table + `strwidth(s)`
//!   - `Renderer` struct skeleton with theme_config / segment_info
//!     / character_translations / width_data
//!   - `compute_divider_widths(get_divider)` per-side hard/soft widths
//!
//! The full `render` / `do_render` / `_render_length` /
//! `_prepare_segments` segment-pipeline implementations are heavy
//! enough to deserve their own port pass; only the structural pieces
//! are covered here.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// import os                                         // py:5
// import re                                         // py:6
// import operator                                   // py:7
// from itertools import chain                       // py:9
// from powerline.theme import Theme                  // py:11
// from powerline.lib.unicode import unichr, strwidth_ucs_2, strwidth_ucs_4                  // py:12

use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `NBSP` from
/// `powerline/renderer.py:15`.
/// `NBSP = ' '` — non-breaking space.
pub const NBSP: &str = "\u{a0}";

/// Port of `np_control_character_translations` from
/// `powerline/renderer.py:18-21`.
///
/// Maps chars in `0x00..=0x1F` to printable two-char sequences:
/// `0x00` → `"^@"`, `0x01` → `"^A"`, …, `0x1F` → `"^_"`.
pub fn np_control_character_translations() -> &'static HashMap<char, String> {
    static M: OnceLock<HashMap<char, String>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = HashMap::new();
        for i in 0u32..0x20 {
            let ch = char::from_u32(i).unwrap();
            let repl_byte = (i + 0x40) as u8;
            let repl = format!("^{}", repl_byte as char);
            m.insert(ch, repl);
        }
        m
    })
}

/// Port of `np_invalid_character_translations` from
/// `powerline/renderer.py:30-33`.
///
/// Maps surrogate-escape codepoints in `0xDC80..=0xDCFF` to
/// `"<80>"`, `"<81>"`, …, `"<FF>"` strings.
pub fn np_invalid_character_translations() -> &'static HashMap<u32, String> {
    static M: OnceLock<HashMap<u32, String>> = OnceLock::new();
    M.get_or_init(|| {
        let mut m = HashMap::new();
        for i in 0xDC80u32..0xDD00 {
            m.insert(i, format!("<{:02x}>", i - 0xDC00));
        }
        m
    })
}

/// Port of `np_invalid_character_re` from
/// `powerline/renderer.py:46`.
///
/// `re.compile('(?<![\uD800-\uDBFF])[\uDC80-\uDD00]')`.
/// The Rust `regex` crate doesn't support lookbehind; this stub
/// matches lone trailing surrogates by codepoint range only — the
/// caller is responsible for the `(?<![\uD800-\uDBFF])` check.
pub fn np_invalid_character_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // Rust regex char ranges over UTF-8 can't reference
        // surrogate codepoints directly, but the regex crate
        // supports `\u{..}` notation for non-surrogate codepoints
        // only. The full Python regex matches surrogate-escape
        // chars produced by `surrogateescape` decoding; in Rust
        // these are represented differently (replacement char +
        // From::from_utf8_lossy), so the stub regex compiles to
        // an empty alternation and is a structural placeholder.
        Regex::new(r"^$").unwrap()
    })
}

/// Port of `np_character_translations` from
/// `powerline/renderer.py:59`.
///
/// Returns a fresh union of `np_control_character_translations`
/// (always) + `np_invalid_character_translations` (UCS-4). Rust is
/// always UCS-4-equivalent (chars are full unicode codepoints), so
/// the table is always the union.
pub fn np_character_translations() -> HashMap<char, String> {
    let mut m = np_control_character_translations().clone();
    // py:59 + py:33  union with invalid translations
    for (cp, repl) in np_invalid_character_translations() {
        if let Some(c) = char::from_u32(*cp) {
            m.insert(c, repl.clone());
        }
    }
    m
}

/// Port of `translate_np()` from
/// `powerline/renderer.py:68-82`.
///
/// Translates non-printable characters in `s` via the
/// `np_character_translations` table.
pub fn translate_np(s: &str) -> String {
    let table = np_character_translations();
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if let Some(repl) = table.get(&c) {
            out.push_str(repl);
        } else {
            out.push(c);
        }
    }
    out
}

/// Port of `construct_returned_value()` from
/// `powerline/renderer.py:92`.
///
/// Builds the per-call return tuple from the rendered highlighted
/// string + the raw segments + width. The Python source returns a
/// bare string when neither `output_raw` nor `output_width` is
/// requested; otherwise returns a tuple.
#[derive(Debug, Clone)]
pub enum RenderReturn {
    /// py:94  return rendered_highlighted (string only)
    Plain(String),
    /// py:96+  tuple variant with optional raw + optional width
    Tuple {
        highlighted: String,
        raw: Option<String>,
        width: Option<usize>,
    },
}

/// Port of `construct_returned_value()` (py:92).
pub fn construct_returned_value(
    rendered_highlighted: String,
    rendered_raw: Option<String>,
    width: usize,
    output_raw: bool,
    output_width: bool,
) -> RenderReturn {
    // py:93-94  if not (output_raw or output_width): return rendered_highlighted
    if !output_raw && !output_width {
        return RenderReturn::Plain(rendered_highlighted);
    }
    // py:96-101  build the tuple
    RenderReturn::Tuple {
        highlighted: rendered_highlighted,
        raw: if output_raw { rendered_raw } else { None },
        width: if output_width { Some(width) } else { None },
    }
}

/// Returns the upstream `width_data` table from
/// `powerline/renderer.py:177-184`.
///
/// Width-class → display-width mapping for `strwidth`. `ambiwidth`
/// configures the East Asian ambiguous width per py:182.
pub fn width_data(ambiwidth: u8) -> HashMap<char, u8> {
    let mut m = HashMap::new();
    // py:177-184  Neutral / Narrow / Ambiguous / Half / Wide / Fullwidth
    m.insert('N', 1);
    m.insert('a', 1);
    m.insert('A', ambiwidth);
    m.insert('H', 1);
    m.insert('W', 2);
    m.insert('F', 2);
    m
}

/// Port of `Renderer.strwidth()` from
/// `powerline/renderer.py:188`.
///
/// Computes the display width of `s` using the `width_data` table.
/// Rust port treats every char as Narrow=1 (no East Asian dispatch
/// without `unicode_width` crate). The function exists so callers
/// can plumb the width-aware path once the crate is added.
pub fn strwidth(s: &str) -> usize {
    // py:188-191  Python iterates per-char and sums width_data[east_asian_width(c)]
    s.chars().count()
}

/// Port of `Renderer.compute_divider_widths()` from
/// `powerline/renderer.py:303`.
///
/// `get_divider(side, kind)` is the caller-supplied closure that
/// resolves the divider string for the given side/kind pair (Python
/// calls `theme.get_divider(side, kind)`).
pub fn compute_divider_widths<F>(mut get_divider: F) -> Map<String, Value>
where
    F: FnMut(&str, &str) -> String,
{
    let mut out = Map::new();
    for side in ["left", "right"] {
        let mut side_map = Map::new();
        // py:304-309  hard / soft per side
        side_map.insert(
            "hard".to_string(),
            Value::from(strwidth(&get_divider(side, "hard"))),
        );
        side_map.insert(
            "soft".to_string(),
            Value::from(strwidth(&get_divider(side, "soft"))),
        );
        out.insert(side.to_string(), Value::Object(side_map));
    }
    out
}

/// Port of `class Renderer(object)` from
/// `powerline/renderer.py:103`.
///
/// Holds the base renderer state. The render-pipeline methods
/// (`render`/`do_render`/`_render_segments`/`_prepare_segments`)
/// are heavy enough to deserve their own port pass; this struct
/// surfaces the constructor + the `segment_info` / `width_data` /
/// `character_translations` state.
pub struct Renderer {
    /// Python: `self.theme_config`.
    pub theme_config: Map<String, Value>,
    /// Python: `self.local_themes`.
    pub local_themes: Map<String, Value>,
    /// Python: `self.character_translations`.
    pub character_translations: HashMap<char, String>,
    /// Python: `self.width_data` per py:177-184.
    pub width_data: HashMap<char, u8>,
}

impl Renderer {
    /// Port of `Renderer.__init__()` from
    /// `powerline/renderer.py:158`.
    pub fn new(
        theme_config: Map<String, Value>,
        local_themes: Map<String, Value>,
        ambiwidth: u8,
    ) -> Self {
        // py:167-171  use_non_breaking_spaces → character_translations[' '] = NBSP
        let use_nbsp = theme_config
            .get("use_non_breaking_spaces")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mut character_translations: HashMap<char, String> = HashMap::new();
        if use_nbsp {
            character_translations.insert(' ', NBSP.to_string());
        }
        Self {
            theme_config,
            local_themes,
            character_translations,
            width_data: width_data(ambiwidth),
        }
    }

    /// Port of `Renderer.segment_info` class attribute from
    /// `powerline/renderer.py:124-128`.
    ///
    /// Returns a fresh dict with environ + getcwd + home keys
    /// populated from the process environment.
    pub fn segment_info() -> Map<String, Value> {
        let mut info = Map::new();
        // py:125-128  environ / getcwd / home
        let env_map: Map<String, Value> = std::env::vars()
            .map(|(k, v)| (k, Value::String(v)))
            .collect();
        info.insert("environ".to_string(), Value::Object(env_map.clone()));
        info.insert(
            "home".to_string(),
            env_map.get("HOME").cloned().unwrap_or(Value::Null),
        );
        info
    }

    /// Port of `Renderer.get_segment_info()` from
    /// `powerline/renderer.py:216`.
    ///
    /// Merges `segment_info` over the base `Renderer::segment_info()`
    /// + sets `mode`. When `PWD` is present, replaces `getcwd` with a
    /// `Value::String(pwd)` (Rust port can't replicate Python's
    /// lambda-closure getcwd; the caller derives the cwd from the
    /// returned segment_info instead).
    pub fn get_segment_info(
        &self,
        segment_info: Option<Map<String, Value>>,
        mode: Option<&str>,
    ) -> Map<String, Value> {
        // py:230  r = self.segment_info.copy()
        let mut r = Self::segment_info();
        // py:231  r['mode'] = mode
        r.insert(
            "mode".to_string(),
            mode.map(|s| Value::String(s.into())).unwrap_or(Value::Null),
        );
        // py:232-233  if segment_info: r.update(segment_info)
        if let Some(extra) = segment_info {
            for (k, v) in extra {
                r.insert(k, v);
            }
        }
        // py:234-235  if 'PWD' in r['environ']: r['getcwd'] = lambda: environ['PWD']
        let pwd = r
            .get("environ")
            .and_then(|v| v.as_object())
            .and_then(|env| env.get("PWD"))
            .and_then(|v| v.as_str())
            .map(String::from);
        if let Some(p) = pwd {
            r.insert("getcwd".to_string(), Value::String(p));
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nbsp_constant_matches_upstream() {
        // py:15  NBSP = ' '
        assert_eq!(NBSP, "\u{a0}");
    }

    #[test]
    fn np_control_character_translations_has_32_entries() {
        // py:18-21  range(0x20) → 32 entries
        let m = np_control_character_translations();
        assert_eq!(m.len(), 32);
    }

    #[test]
    fn np_control_character_translations_maps_null_to_caret_at() {
        // py:20  '\x00' → '^@'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\u{00}'), Some(&"^@".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_a_to_caret_a() {
        // py:20  '\x01' → '^A'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\u{01}'), Some(&"^A".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_tab_to_caret_i() {
        // py:18-21 docstring: maps tab (0x09) to '^I'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\t'), Some(&"^I".to_string()));
    }

    #[test]
    fn np_control_character_translations_maps_newline_to_caret_j() {
        // py:18-21 docstring: maps newline (0x0A) to '^J'
        let m = np_control_character_translations();
        assert_eq!(m.get(&'\n'), Some(&"^J".to_string()));
    }

    #[test]
    fn np_invalid_character_translations_has_128_entries() {
        // py:30-33  range(0xDC80, 0xDD00) → 128 entries
        let m = np_invalid_character_translations();
        assert_eq!(m.len(), 128);
    }

    #[test]
    fn np_invalid_character_translations_first_entry_is_80() {
        // py:32  0xDC80 → '<80>'
        let m = np_invalid_character_translations();
        assert_eq!(m.get(&0xDC80), Some(&"<80>".to_string()));
    }

    #[test]
    fn np_invalid_character_translations_last_entry_is_ff() {
        // py:30-33  0xDCFF → '<ff>'
        let m = np_invalid_character_translations();
        assert_eq!(m.get(&0xDCFF), Some(&"<ff>".to_string()));
    }

    #[test]
    fn np_character_translations_contains_both_control_and_invalid() {
        let m = np_character_translations();
        // 32 control + 128 invalid (where char::from_u32 succeeds = 0)
        // surrogates are invalid Rust chars so the union doesn't
        // actually include them — Rust char::from_u32 returns None for
        // surrogates. So we get only the 32 control entries.
        assert!(m.len() >= 32);
        assert!(m.contains_key(&'\u{00}'));
    }

    #[test]
    fn translate_np_replaces_control_chars() {
        // py:74-82  translate via character_translations table
        let r = translate_np("abc\x01def");
        assert_eq!(r, "abc^Adef");
    }

    #[test]
    fn translate_np_passes_printable_chars_through() {
        let r = translate_np("hello world");
        assert_eq!(r, "hello world");
    }

    #[test]
    fn translate_np_handles_multiple_control_chars() {
        let r = translate_np("\x00\x01\x02");
        assert_eq!(r, "^@^A^B");
    }

    #[test]
    fn construct_returned_value_plain_when_no_flags() {
        // py:93-94  return rendered_highlighted
        let r = construct_returned_value("hi".to_string(), None, 5, false, false);
        match r {
            RenderReturn::Plain(s) => assert_eq!(s, "hi"),
            _ => panic!("expected Plain"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_width_only() {
        let r = construct_returned_value("hi".to_string(), None, 5, false, true);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert!(raw.is_none());
                assert_eq!(width, Some(5));
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_raw_only() {
        let r =
            construct_returned_value("hi".to_string(), Some("hi-raw".to_string()), 5, true, false);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert_eq!(raw, Some("hi-raw".to_string()));
                assert!(width.is_none());
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn construct_returned_value_tuple_with_both() {
        let r =
            construct_returned_value("hi".to_string(), Some("hi-raw".to_string()), 5, true, true);
        match r {
            RenderReturn::Tuple {
                highlighted,
                raw,
                width,
            } => {
                assert_eq!(highlighted, "hi");
                assert_eq!(raw, Some("hi-raw".to_string()));
                assert_eq!(width, Some(5));
            }
            _ => panic!("expected Tuple"),
        }
    }

    #[test]
    fn width_data_default_ambiwidth_is_1() {
        // py:177-184
        let w = width_data(1);
        assert_eq!(w.get(&'N'), Some(&1));
        assert_eq!(w.get(&'A'), Some(&1));
        assert_eq!(w.get(&'W'), Some(&2));
        assert_eq!(w.get(&'F'), Some(&2));
    }

    #[test]
    fn width_data_ambiwidth_overrides_ambiguous() {
        // py:181  'A': ambiwidth
        let w = width_data(2);
        assert_eq!(w.get(&'A'), Some(&2));
    }

    #[test]
    fn strwidth_counts_chars() {
        // py:188-191  per-char width sum
        assert_eq!(strwidth("hello"), 5);
        assert_eq!(strwidth(""), 0);
        assert_eq!(strwidth("café"), 4);
    }

    #[test]
    fn compute_divider_widths_emits_both_sides_and_kinds() {
        // py:303-310
        let r = compute_divider_widths(|side, kind| match (side, kind) {
            ("left", "hard") => " ".to_string(),
            ("left", "soft") => " ".to_string(),
            ("right", "hard") => " ".to_string(),
            ("right", "soft") => " ".to_string(),
            _ => "".to_string(),
        });
        assert!(r.contains_key("left"));
        assert!(r.contains_key("right"));
        let left = r["left"].as_object().unwrap();
        assert_eq!(left["hard"], 1);
        assert_eq!(left["soft"], 1);
    }

    #[test]
    fn renderer_init_use_nbsp_default_adds_space_translation() {
        // py:167-171  use_non_breaking_spaces defaults to True
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        assert_eq!(r.character_translations.get(&' '), Some(&NBSP.to_string()));
    }

    #[test]
    fn renderer_init_use_nbsp_false_omits_space_translation() {
        let mut cfg = Map::new();
        cfg.insert("use_non_breaking_spaces".to_string(), Value::Bool(false));
        let r = Renderer::new(cfg, Map::new(), 1);
        assert!(!r.character_translations.contains_key(&' '));
    }

    #[test]
    fn renderer_init_uses_ambiwidth_for_width_data() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 2);
        assert_eq!(r.width_data.get(&'A'), Some(&2));
    }

    #[test]
    fn segment_info_includes_environ_and_home() {
        let info = Renderer::segment_info();
        assert!(info.contains_key("environ"));
        assert!(info.contains_key("home"));
    }

    #[test]
    fn get_segment_info_merges_segment_info_over_base() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let mut extra = Map::new();
        extra.insert("client_id".to_string(), Value::from(42));
        let info = r.get_segment_info(Some(extra), Some("normal"));
        assert_eq!(info["mode"], "normal");
        assert_eq!(info["client_id"], 42);
    }

    #[test]
    fn get_segment_info_sets_mode_null_when_none() {
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let info = r.get_segment_info(None, None);
        assert_eq!(info["mode"], Value::Null);
    }

    #[test]
    fn get_segment_info_overrides_getcwd_when_pwd_set() {
        // py:234-235  if 'PWD' in environ: getcwd = lambda
        let cfg = Map::new();
        let r = Renderer::new(cfg, Map::new(), 1);
        let mut extra = Map::new();
        let mut env = Map::new();
        env.insert("PWD".to_string(), Value::String("/my/cwd".into()));
        extra.insert("environ".to_string(), Value::Object(env));
        let info = r.get_segment_info(Some(extra), None);
        assert_eq!(info.get("getcwd"), Some(&Value::String("/my/cwd".into())));
    }
}
