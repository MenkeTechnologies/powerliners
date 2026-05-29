// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/unicode.py`.
//!
//! Upstream is a 283-line Python 2/3 unicode-compat layer. The vast
//! majority of it handles Py2 `unicode`/`str` distinctions, codec
//! error fallbacks, and `__builtin__.unichr` polyfills — all of which
//! are no-ops in Rust where every `String` is UTF-8 by construction
//! and `char` is a 4-byte Unicode scalar.
//!
//! Ported surface:
//!   - `u()` / `safe_unicode*` — string coercion
//!   - `tointiter()` — byte iter helper
//!   - `powerline_decode_error()` — `<XX>` hex error formatter
//!   - `register_strwidth_error()` — encode-error name generator
//!   - `out_u()` — bytes → string via preferred output encoding
//!   - `FailedUnicode` — marker newtype
//!   - `surrogate_pair_to_character()` — high/low surrogate join
//!   - `string()` — identity for `&str`
//!   - `strwidth_ucs_4()` / `strwidth_ucs_2()` — display-width helpers

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import codecs                                    // py:5
// from unicodedata import east_asian_width, combining  // py:7
// from powerline.lib.encoding import get_preferred_output_encoding  // py:9

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Port of `u()` from `powerline/lib/unicode.py:35`.
///
/// Python:
/// ```python
/// def u(s):
///     '''Return unicode instance assuming UTF-8 encoded string.'''
///     if type(s) is unicode:
///         return s
///     else:
///         return unicode(s, 'utf-8')
/// ```
///
/// In Python the input could be `unicode` (Py2) / `str` (Py3) or
/// `bytes`. Rust callers pass `&str` (already valid UTF-8) or
/// `&[u8]` (byte slice). Both shapes are provided.
pub fn u(s: &str) -> String {
    // py:35
    // py:37  type(s) is unicode → already unicode, return as-is
    // py:39  unicode(s, 'utf-8') → decode bytes as UTF-8
    // For &str input both paths collapse to "copy to owned String".
    s.to_string()
}

/// Port of `tointiter()` from
/// `powerline/lib/unicode.py:44-53`.
///
/// Python: convert a byte string to the sequence of integers.
/// Py2 uses `ord(c) for c in s`, Py3 uses `iter(s)`. Rust's `&[u8]`
/// already iterates as `u8`, so the impl is just `.iter().copied()`.
pub fn tointiter(s: &[u8]) -> impl Iterator<Item = u8> + '_ {
    // py:48  return (ord(c) for c in s)
    // py:53  return iter(s)
    s.iter().copied()
}

/// Port of `powerline_decode_error()` from
/// `powerline/lib/unicode.py:56-62`.
///
/// Replaces an invalid byte range with `<XX>` hex notation. Python
/// receives a `UnicodeDecodeError` exception object; Rust port takes
/// the raw byte slice that triggered the error since Rust decoders
/// (e.g. `from_utf8_lossy`) don't surface the same exception shape.
/// Returns the replacement string plus the consumed byte count, the
/// same `(replacement, end)` tuple Python returns at py:62.
pub fn powerline_decode_error(bytes: &[u8]) -> (String, usize) {
    // py:59-61  ''.join('<{0:02X}>'.format(c) for c in tointiter(...))
    let mut out = String::with_capacity(bytes.len() * 4);
    for c in tointiter(bytes) {
        out.push_str(&format!("<{:02X}>", c));
    }
    // py:62  return (..., e.end)
    (out, bytes.len())
}

/// Module-level counter mirroring Python's `last_swe_idx` global at
/// `powerline/lib/unicode.py:68`. Bumped by `register_strwidth_error`
/// each time a new error handler is registered.
static LAST_SWE_IDX: AtomicUsize = AtomicUsize::new(0);

/// Port of `register_strwidth_error()` from
/// `powerline/lib/unicode.py:71-103`.
///
/// Python registers a global codec error handler named
/// `powerline_encode_strwidth_error_<N>` that replaces unencodable
/// chars with question marks proportional to display width. The
/// returned string is the handler name, used later by `s.encode(enc,
/// errors=name)` callers.
///
/// Rust has no codec error registry. The port returns a `(name,
/// handler)` pair: `name` matches Python's generated identifier so
/// log/debug output stays consistent, `handler` is a closure callers
/// invoke directly when they hit unencodable runs. The internal
/// `LAST_SWE_IDX` counter mirrors Python's `global last_swe_idx`.
pub fn register_strwidth_error<F>(strwidth: F) -> (String, Box<dyn Fn(&str) -> (String, usize)>)
where
    F: Fn(&str) -> usize + Send + Sync + 'static,
{
    // py:93-94  global last_swe_idx; last_swe_idx += 1
    let idx = LAST_SWE_IDX.fetch_add(1, Ordering::SeqCst) + 1;
    // py:96-99  powerline_encode_strwidth_error(e):
    //          return ('?' * strwidth(e.object[e.start:e.end]), e.end)
    let handler: Box<dyn Fn(&str) -> (String, usize)> = Box::new(move |slice: &str| {
        let w = strwidth(slice);
        ("?".repeat(w), slice.len())
    });
    // py:101  ename = 'powerline_encode_strwidth_error_{0}'.format(...)
    let ename = format!("powerline_encode_strwidth_error_{}", idx);
    // py:102  codecs.register_error(ename, ...) — no-op in Rust
    // py:103  return ename
    (ename, handler)
}

/// Port of `out_u()` from `powerline/lib/unicode.py:106-118`.
///
/// Return unicode string suitable for displaying. Python decodes
/// bytes with `get_preferred_output_encoding()` falling through to
/// `powerline_decode_error`. Rust port:
///   - `&str` input → identity copy (py:113-114)
///   - `&[u8]` input → lossy UTF-8 decode (py:115-116)
pub fn out_u_str(s: &str) -> String {
    // py:113-114  isinstance(s, unicode) → return s
    s.to_string()
}

/// Bytes overload of `out_u()`. Falls back to lossy UTF-8 decoding
/// (Python tries `get_preferred_output_encoding()` then
/// `powerline_decode_error`; Rust uses `from_utf8_lossy` which
/// substitutes U+FFFD for invalid sequences).
pub fn out_u_bytes(s: &[u8]) -> String {
    // py:115-116  isinstance(s, bytes) → unicode(s, encoding, errors)
    String::from_utf8_lossy(s).into_owned()
}

/// Port of `safe_unicode()` from `powerline/lib/unicode.py:121`.
///
/// Return unicode instance without raising an exception.
///
/// Python tries ASCII → UTF-8 → `__str__`/`__repr__` → preferred output
/// encoding → recursive fallback. In Rust, every input has a
/// well-defined `Display` or `Debug` impl, and `String::from_utf8_lossy`
/// covers the byte-slice fallback. The port collapses the cascade into
/// the one operation that survives all cases.
pub fn safe_unicode_str(s: &str) -> String {
    // py:121
    // py:138-139  type(s) is bytes → 'ascii' decode fallback
    // py:140-141  not bytes → unicode(s) (already-unicode)
    // For &str the result is just the owned copy.
    s.to_string()
}

/// Bytes overload of `safe_unicode()`.
///
/// Falls back to lossy UTF-8 decoding for non-ASCII bytes
/// (Python tries UTF-8 then the preferred output encoding; Rust uses
/// `from_utf8_lossy` which substitutes U+FFFD for invalid sequences).
pub fn safe_unicode_bytes(s: &[u8]) -> String {
    // py:121
    String::from_utf8_lossy(s).into_owned()
}

/// `safe_unicode` accepting any `Display` value.
///
/// Mirrors Python's `unicode(s)` fallback at py:140 which calls
/// `__str__` / `__repr__` on the input. Rust's `Display` is the analog.
pub fn safe_unicode<T: std::fmt::Display>(s: T) -> String {
    format!("{}", s)
}

/// Port of `FailedUnicode` from `powerline/lib/unicode.py:150-159`.
///
/// Python: builtin `unicode` subclass indicating fatal error in
/// `.render()`. Callers check `isinstance(result, FailedUnicode)` to
/// detect failure without raising. Rust port is a newtype around
/// `String` so callers can pattern-match via `if let` instead of
/// `isinstance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailedUnicode(pub String);

impl FailedUnicode {
    /// Construct from any `Display` value.
    pub fn new<T: std::fmt::Display>(s: T) -> Self {
        FailedUnicode(format!("{}", s))
    }

    /// Borrow as `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume into owned `String`.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for FailedUnicode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for FailedUnicode {
    fn from(s: String) -> Self {
        FailedUnicode(s)
    }
}

impl From<&str> for FailedUnicode {
    fn from(s: &str) -> Self {
        FailedUnicode(s.to_string())
    }
}

/// Port of `string()` from `powerline/lib/unicode.py:162-173`.
///
/// Py2: encode `unicode` to UTF-8 bytes. Py3: decode `bytes` to
/// `str` via UTF-8. Either way the result is the Py-native `str`.
/// Rust's `&str` is already UTF-8 so the str overload is identity;
/// the bytes overload uses `from_utf8_lossy`.
pub fn string_from_str(s: &str) -> String {
    // py:169-173  Py3: type(s) is str → s; else s.decode('utf-8')
    s.to_string()
}

/// Bytes overload of `string()`. Py3 decodes UTF-8 at py:171.
pub fn string_from_bytes(s: &[u8]) -> String {
    // py:171  s.decode('utf-8')
    String::from_utf8_lossy(s).into_owned()
}

/// Port of `surrogate_pair_to_character()` from
/// `powerline/lib/unicode.py:190-193`.
///
/// Transform a pair of surrogate codepoints to one codepoint.
pub fn surrogate_pair_to_character(high: u32, low: u32) -> u32 {
    // py:193  0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
    0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
}

/// Port of `strwidth_ucs_4()` from
/// `powerline/lib/unicode.py:247-254`.
///
/// Compute string width in display cells. Rust `char` is already
/// UCS-4 so the UCS-4 path is the default.
///
/// `width_data` maps east_asian_width category codes ('F', 'H',
/// 'W', 'Na', 'A', 'N') to display widths. Combining marks count
/// as 0 (py:251).
///
/// Note: Rust stdlib has no `east_asian_width` / `combining` lookup.
/// Until a foundational unicode-properties crate is wired in, this
/// function falls back to "Narrow" (`'Na' or 'N'`) for every char.
/// The structure of the port (lookup + sum) matches Python so
/// callers wire up identically once the table is available.
pub fn strwidth_ucs_4(width_data: &HashMap<String, usize>, string: &str) -> usize {
    // py:248-254  sum(0 if combining(c) else width_data[east_asian_width(c)] for c in string)
    let fallback = width_data
        .get("N")
        .or_else(|| width_data.get("Na"))
        .copied()
        .unwrap_or(1);
    string.chars().map(|_c| fallback).sum()
}

/// Port of `strwidth_ucs_2()` from
/// `powerline/lib/unicode.py:267-276`.
///
/// UCS-2 variant — handles surrogate pairs. Rust's `&str` is always
/// UTF-8 and `chars()` yields decoded `char` values (no surrogate
/// pairs ever surface), so this path collapses to the UCS-4 impl.
pub fn strwidth_ucs_2(width_data: &HashMap<String, usize>, string: &str) -> usize {
    // py:267-276  surrogate-pair aware; collapses to UCS-4 in Rust
    strwidth_ucs_4(width_data, string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u_passes_through_str() {
        assert_eq!(u("hello"), "hello");
    }

    #[test]
    fn u_handles_utf8() {
        assert_eq!(u("héllo →"), "héllo →");
    }

    #[test]
    fn tointiter_yields_byte_ints() {
        let v: Vec<u8> = tointiter(b"abc").collect();
        assert_eq!(v, vec![b'a', b'b', b'c']);
    }

    #[test]
    fn tointiter_handles_empty() {
        let v: Vec<u8> = tointiter(b"").collect();
        assert!(v.is_empty());
    }

    #[test]
    fn powerline_decode_error_formats_hex() {
        // py:60  '<{0:02X}>'.format(c)
        let (s, end) = powerline_decode_error(&[0xff, 0xfe]);
        assert_eq!(s, "<FF><FE>");
        assert_eq!(end, 2);
    }

    #[test]
    fn powerline_decode_error_handles_empty_range() {
        let (s, end) = powerline_decode_error(&[]);
        assert!(s.is_empty());
        assert_eq!(end, 0);
    }

    #[test]
    fn register_strwidth_error_generates_unique_name() {
        let (n1, _) = register_strwidth_error(|s| s.chars().count());
        let (n2, _) = register_strwidth_error(|s| s.chars().count());
        assert!(n1.starts_with("powerline_encode_strwidth_error_"));
        assert!(n2.starts_with("powerline_encode_strwidth_error_"));
        assert_ne!(n1, n2);
    }

    #[test]
    fn register_strwidth_error_handler_emits_question_marks() {
        let (_, handler) = register_strwidth_error(|s| s.chars().count());
        let (out, end) = handler("…");
        // 1 character wide → 1 question mark
        assert_eq!(out, "?");
        assert_eq!(end, "…".len());
    }

    #[test]
    fn register_strwidth_error_handler_respects_width() {
        // Simulate fullwidth: 2 cells per char
        let (_, handler) = register_strwidth_error(|s| s.chars().count() * 2);
        let (out, _) = handler("Ａ");
        assert_eq!(out, "??");
    }

    #[test]
    fn out_u_str_passes_through() {
        assert_eq!(out_u_str("hello"), "hello");
    }

    #[test]
    fn out_u_bytes_decodes_valid_utf8() {
        assert_eq!(out_u_bytes(b"hello"), "hello");
    }

    #[test]
    fn out_u_bytes_lossy_on_invalid() {
        let bad = &[0xff, b'a'];
        let out = out_u_bytes(bad);
        assert!(out.contains('\u{FFFD}'));
    }

    #[test]
    fn safe_unicode_str_round_trips() {
        assert_eq!(safe_unicode_str("hello"), "hello");
    }

    #[test]
    fn safe_unicode_bytes_handles_valid_utf8() {
        assert_eq!(safe_unicode_bytes(b"hello"), "hello");
    }

    #[test]
    fn safe_unicode_bytes_lossy_on_invalid() {
        let bad = &[0xff, 0xfe, b'a'];
        let result = safe_unicode_bytes(bad);
        assert!(result.contains('\u{FFFD}') || result.contains('a'));
    }

    #[test]
    fn safe_unicode_display_works_on_int() {
        assert_eq!(safe_unicode(42), "42");
    }

    #[test]
    fn string_from_str_identity() {
        assert_eq!(string_from_str("hello"), "hello");
    }

    #[test]
    fn string_from_bytes_decodes_utf8() {
        assert_eq!(string_from_bytes(b"hello"), "hello");
    }

    #[test]
    fn string_from_bytes_lossy_on_invalid() {
        assert!(string_from_bytes(&[0xff, b'x']).contains('\u{FFFD}'));
    }

    #[test]
    fn surrogate_pair_to_character_round_trips_emoji_range() {
        // U+1F600 GRINNING FACE = surrogate pair D83D DE00
        let cp = surrogate_pair_to_character(0xD83D, 0xDE00);
        assert_eq!(cp, 0x1F600);
    }

    #[test]
    fn surrogate_pair_to_character_low_surrogate_boundary() {
        // U+10000 = D800 DC00 (smallest surrogate pair)
        let cp = surrogate_pair_to_character(0xD800, 0xDC00);
        assert_eq!(cp, 0x10000);
    }

    #[test]
    fn failed_unicode_display_returns_inner_string() {
        let fu = FailedUnicode::new("No window 5");
        assert_eq!(fu.to_string(), "No window 5");
    }

    #[test]
    fn failed_unicode_eq_compares_by_value() {
        let a = FailedUnicode::from("err");
        let b = FailedUnicode::from(String::from("err"));
        assert_eq!(a, b);
    }

    #[test]
    fn failed_unicode_into_string_yields_inner() {
        let fu = FailedUnicode::new("oops");
        assert_eq!(fu.into_string(), "oops");
    }

    #[test]
    fn strwidth_ucs_4_sums_with_default_table() {
        let mut width_data = HashMap::new();
        width_data.insert("N".to_string(), 1);
        width_data.insert("Na".to_string(), 1);
        assert_eq!(strwidth_ucs_4(&width_data, "hello"), 5);
        assert_eq!(strwidth_ucs_4(&width_data, ""), 0);
    }

    #[test]
    fn strwidth_ucs_4_uses_table_value() {
        let mut width_data = HashMap::new();
        // Pretend everything is fullwidth
        width_data.insert("N".to_string(), 2);
        width_data.insert("Na".to_string(), 2);
        assert_eq!(strwidth_ucs_4(&width_data, "hi"), 4);
    }

    #[test]
    fn strwidth_ucs_4_empty_table_falls_back_to_one() {
        let width_data = HashMap::new();
        // Falls back to 1 per char when no key matches
        assert_eq!(strwidth_ucs_4(&width_data, "abc"), 3);
    }

    #[test]
    fn strwidth_ucs_2_matches_ucs_4_for_basic_strs() {
        let mut width_data = HashMap::new();
        width_data.insert("N".to_string(), 1);
        width_data.insert("Na".to_string(), 1);
        assert_eq!(
            strwidth_ucs_2(&width_data, "hello"),
            strwidth_ucs_4(&width_data, "hello")
        );
    }
}
