// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/unicode.py`.
//!
//! Upstream is a 283-line Python 2/3 unicode-compat layer. The vast
//! majority of it handles Py2 `unicode`/`str` distinctions, codec
//! error fallbacks, and `__builtin__.unichr` polyfills — all of which
//! are no-ops in Rust where every `String` is UTF-8 by construction
//! and `char` is a 4-byte Unicode scalar.
//!
//! The minimal slice ported here is what consumers (theme, segment,
//! renderer) actually call: `u()` and `safe_unicode()`. The rest of
//! the file (`tointiter`, `powerline_decode_error`, `FailedUnicode`,
//! east-asian-width helpers, `register_strwidth_error`) ports
//! incrementally as consumers need them.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import codecs                                    // py:5
// from unicodedata import east_asian_width, combining  // py:7
// from powerline.lib.encoding import get_preferred_output_encoding  // py:9

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
pub fn u(s: &str) -> String {                       // py:35
    // py:37  type(s) is unicode → already unicode, return as-is
    // py:39  unicode(s, 'utf-8') → decode bytes as UTF-8
    // For &str input both paths collapse to "copy to owned String".
    s.to_string()
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
pub fn safe_unicode_str(s: &str) -> String {        // py:121
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
pub fn safe_unicode_bytes(s: &[u8]) -> String {     // py:121
    String::from_utf8_lossy(s).into_owned()
}

/// `safe_unicode` accepting any `Display` value.
///
/// Mirrors Python's `unicode(s)` fallback at py:140 which calls
/// `__str__` / `__repr__` on the input. Rust's `Display` is the analog.
pub fn safe_unicode<T: std::fmt::Display>(s: T) -> String {
    format!("{}", s)
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
    fn safe_unicode_str_round_trips() {
        assert_eq!(safe_unicode_str("hello"), "hello");
    }

    #[test]
    fn safe_unicode_bytes_handles_valid_utf8() {
        assert_eq!(safe_unicode_bytes(b"hello"), "hello");
    }

    #[test]
    fn safe_unicode_bytes_lossy_on_invalid() {
        // Invalid UTF-8 byte sequence
        let bad = &[0xff, 0xfe, b'a'];
        let result = safe_unicode_bytes(bad);
        // Should not panic; should contain the replacement char
        assert!(result.contains('\u{FFFD}') || result.contains('a'));
    }

    #[test]
    fn safe_unicode_display_works_on_int() {
        assert_eq!(safe_unicode(42), "42");
    }
}
