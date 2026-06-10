// vim:fileencoding=utf-8:noet
//! Boundary / edge-case tests for `lib::` helpers — covers cases the
//! in-module unit tests do not exercise.
//!
//! Each test below targets ONE specific bug class missing from
//! `src/ported/lib/{humanize_bytes,url,unicode}.rs` `#[cfg(test)]` blocks:
//!
//!   1. `humanize_bytes` exponent saturation — feeding values beyond the
//!      PiB range (1024^7 = 1 ZiB) must NOT panic, overflow, or break
//!      formatting. The exponent computation routes through
//!      `num.log(div) as i64` which can saturate on extreme inputs; the
//!      `.min(unit_list.len() - 1)` clamp is the only defense. Existing
//!      tests only cover up to 1 GiB.
//!
//!   2. `urllib_urlencode` multi-byte UTF-8 — the percent_encode helper
//!      iterates `s.bytes()` (correct) rather than `s.chars()` (would
//!      emit `%xxxx` codepoints instead of UTF-8 byte sequences). Pinning
//!      a non-ASCII string verifies that each UTF-8 byte is percent-
//!      escaped independently (matching `urllib.parse.quote_plus`
//!      semantics). Existing tests only cover ASCII inputs.
//!
//!   3. `surrogate_pair_to_character` upper boundary — the highest valid
//!      surrogate pair is (0xDBFF, 0xDFFF) which must map exactly to
//!      U+10FFFF (the Unicode maximum). Existing tests only cover U+10000
//!      and U+1F600. A regression to the arithmetic (`<< 10` shift count,
//!      base offset, subtraction order) would skew the upper edge
//!      without affecting the mid-range emoji codepoints.

use powerliners::lib::humanize_bytes::humanize_bytes;
use powerliners::lib::unicode::surrogate_pair_to_character;
use powerliners::lib::url::urllib_urlencode;

/// `humanize_bytes` saturates the exponent at PiB instead of panicking
/// or producing invalid prefixes when fed extreme magnitudes. Feeds 1
/// zebibyte (1024^7) — far past the upstream `unit_list`'s top entry
/// (PiB at index 5). The expected output contains `PiB` because the
/// `.min(unit_list.len() - 1)` clamp at
/// `src/ported/lib/humanize_bytes.rs:42` caps `exponent` at 5.
///
/// Bug class caught: an off-by-one in the saturation clamp (e.g.
/// dropping `.min(max)` or replacing it with `< max` would index past
/// the array end and panic) OR a change to `unit_list` length without
/// updating the clamp. Either would surface here. The in-module tests
/// (1024 / 1024² / 1024³) never reach the saturation branch.
#[test]
fn humanize_bytes_saturates_at_pib_for_zib_magnitude() {
    let one_zib = 1024.0_f64.powi(7);
    let out = humanize_bytes(one_zib, "B", false);
    assert!(
        out.contains("PiB"),
        "humanize_bytes(1 ZiB, \"B\", false) = {:?} — expected the \
         exponent to saturate at PiB (index 5) per the \
         min(unit_list.len() - 1) clamp at humanize_bytes.rs:42",
        out
    );
    // Sanity: no panic, no NaN leakage, no "" output.
    assert!(!out.is_empty(), "saturation path produced empty output");
    assert!(
        !out.to_lowercase().contains("nan"),
        "saturation path leaked NaN into output: {:?}",
        out
    );
}

/// `urllib_urlencode` percent-encodes each UTF-8 byte of a multi-byte
/// string independently. The classic regression here is iterating over
/// `chars()` (yields `char`/codepoint) instead of `bytes()` (yields the
/// UTF-8 byte sequence). Iterating chars on `"café"` would attempt to
/// percent-encode `é` as `%E9` (its codepoint U+00E9) — but the correct
/// urlencode of `é` is `%C3%A9` (the two UTF-8 bytes 0xC3 0xA9), which
/// matches `urllib.parse.quote_plus("café")` exactly.
///
/// Bug class caught: a refactor that swaps `s.bytes()` for `s.chars()`
/// in `percent_encode` at `src/ported/lib/url.rs:78`. The in-module
/// tests use only ASCII so the bug would slip through unnoticed.
#[test]
fn urllib_urlencode_percent_encodes_utf8_bytes_not_codepoints() {
    let out = urllib_urlencode(vec![("q", "café")]);
    assert_eq!(
        out, "q=caf%C3%A9",
        "Expected each UTF-8 byte of 'é' (0xC3 0xA9) to be percent-\
         encoded as %C3%A9 — matching Python's urllib.parse.quote_plus. \
         If the test fails with '%E9' the implementation regressed from \
         byte-iteration (correct) to char-iteration (broken)."
    );
}

/// `surrogate_pair_to_character` correctly maps the MAXIMUM valid
/// surrogate pair (0xDBFF, 0xDFFF) to U+10FFFF — the Unicode
/// upper limit.
///
/// Algebraic check from `src/ported/lib/unicode.rs:435`:
///     0x10000 + ((0xDBFF - 0xD800) << 10) + (0xDFFF - 0xDC00)
///   = 0x10000 + (0x3FF << 10) + 0x3FF
///   = 0x10000 + 0xFFC00 + 0x3FF
///   = 0x10FFFF
///
/// Bug class caught: any drift in the arithmetic — wrong shift count
/// (`<< 9` or `<< 11`), wrong base offset (`0x20000`), or subtraction
/// inversion (`0xDC00 - low`) — would all leave the existing tests
/// (U+10000 minimum, U+1F600 mid-range) passing while breaking the
/// upper bound. This test pins the maximum to detect such drift.
#[test]
fn surrogate_pair_to_character_upper_boundary_is_unicode_max() {
    let cp = surrogate_pair_to_character(0xDBFF, 0xDFFF);
    assert_eq!(
        cp, 0x10FFFF,
        "Expected (0xDBFF, 0xDFFF) → U+10FFFF (Unicode maximum); got \
         {:X}. Regression in the (<<10 shift) | (0x10000 base) | \
         (-0xD800/-0xDC00 offsets) arithmetic at unicode.rs:435.",
        cp
    );
    // The result must also be a valid Rust char codepoint (≤ U+10FFFF
    // and not a surrogate). Verify via `char::from_u32`.
    assert!(
        char::from_u32(cp).is_some(),
        "U+10FFFF must be representable as a Rust char; got cp={:X}",
        cp
    );
}
