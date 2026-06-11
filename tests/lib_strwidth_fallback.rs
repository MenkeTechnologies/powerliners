// vim:fileencoding=utf-8:noet
//! Edge-case tests for `lib::unicode::strwidth_ucs_4` / `strwidth_ucs_2`
//! covering two code-paths the in-module unit tests do NOT exercise.
//!
//! Background: until a unicode east_asian_width table is wired in, the
//! Rust port treats every char with one fallback width, resolved at
//! `src/ported/lib/unicode.rs:455-459`:
//!
//! ```text
//! let fallback = width_data
//!     .get("N")
//!     .or_else(|| width_data.get("Na"))
//!     .copied()
//!     .unwrap_or(1);
//! string.chars().map(|_c| fallback).sum()
//! ```
//!
//! The in-module tests (`strwidth_ucs_4_sums_with_default_table`,
//! `_uses_table_value`, `_empty_table_falls_back_to_one`) all insert the
//! SAME value for both `"N"` and `"Na"` and feed only ASCII strings. That
//! leaves two distinct bug classes unguarded:
//!
//!   1. Fallback key PRECEDENCE: when `"N"` and `"Na"` disagree, the
//!      `.get("N").or_else(|| .get("Na"))` chain pins `"N"` as the winner.
//!      Every existing test sets them equal, so swapping the two lookups
//!      (a one-token edit) would pass the whole in-module suite while
//!      silently changing which category drives the fallback width.
//!
//!   2. CHAR vs BYTE counting: `string.chars().map(...)` counts Unicode
//!      scalar values. A regression to `.bytes()` or `string.len()` —
//!      the single most common width-calculation bug, and a column-
//!      alignment break for a statusline tool — would inflate the width
//!      of any multibyte string. All existing strwidth tests use ASCII
//!      (1 byte == 1 char), so the bug is invisible to them.

use powerliners::lib::unicode::{strwidth_ucs_2, strwidth_ucs_4};
use std::collections::HashMap;

/// Pins the fallback-key precedence: `"N"` wins over `"Na"` when both are
/// present with different values.
///
/// `strwidth_ucs_4` resolves its per-char width via
/// `width_data.get("N").or_else(|| width_data.get("Na"))`
/// (`src/ported/lib/unicode.rs:455-457`). With `N=3` and `Na=2`, three
/// ASCII chars must measure `3 * 3 = 9`, NOT `3 * 2 = 6`.
///
/// Bug class caught: reordering the `.or_else` chain to prefer `"Na"`
/// (or any future refactor that flattens the two lookups) would yield 6
/// here. The in-module tests can't catch it because they set `N == Na`.
#[test]
fn strwidth_ucs_4_prefers_n_over_na_when_they_differ() {
    let mut width_data = HashMap::new();
    width_data.insert("N".to_string(), 3);
    width_data.insert("Na".to_string(), 2);
    assert_eq!(
        strwidth_ucs_4(&width_data, "abc"),
        9,
        "`N` (=3) must take precedence over `Na` (=2) per the \
         `.get(\"N\").or_else(|| .get(\"Na\"))` chain at unicode.rs:455-457; \
         got a value != 9 means the precedence regressed to `Na`"
    );
}

/// Falls back to `"Na"` only when `"N"` is absent.
///
/// Complements the precedence test: removing `"N"` must route through the
/// `.or_else` arm and use `"Na"`. With only `Na=5`, two chars measure
/// `2 * 5 = 10`.
///
/// Bug class caught: dropping the `.or_else(|| .get("Na"))` arm entirely
/// (so a missing `"N"` collapses straight to the `unwrap_or(1)` default)
/// would yield 2 instead of 10.
#[test]
fn strwidth_ucs_4_falls_back_to_na_when_n_absent() {
    let mut width_data = HashMap::new();
    width_data.insert("Na".to_string(), 5);
    assert_eq!(
        strwidth_ucs_4(&width_data, "ab"),
        10,
        "with `N` absent, the `.or_else(|| .get(\"Na\"))` arm must apply \
         `Na` (=5); got a value != 10 means the `Na` fallback arm was lost"
    );
}

/// Counts Unicode scalar values (chars), NOT UTF-8 bytes.
///
/// `"héllo"` is 5 chars but 6 bytes (`é` = U+00E9 = 0xC3 0xA9). With a
/// per-char fallback width of 1, the display width MUST be 5. A regression
/// to `.bytes()` / `string.len()` would report 6 — a one-cell over-count
/// that breaks statusline column alignment for any non-ASCII segment.
///
/// Bug class caught: byte-vs-char confusion in the `string.chars()` map
/// at `src/ported/lib/unicode.rs:460`. Every existing strwidth test uses
/// pure ASCII where bytes == chars, so this divergence is invisible to
/// the current suite.
#[test]
fn strwidth_ucs_4_counts_chars_not_utf8_bytes() {
    let mut width_data = HashMap::new();
    width_data.insert("N".to_string(), 1);
    width_data.insert("Na".to_string(), 1);
    // "héllo": 5 scalar values, 6 UTF-8 bytes.
    assert_eq!("héllo".chars().count(), 5);
    assert_eq!("héllo".len(), 6);
    assert_eq!(
        strwidth_ucs_4(&width_data, "héllo"),
        5,
        "width must count the 5 chars of \"héllo\", not its 6 UTF-8 bytes; \
         a result of 6 means `.chars()` regressed to byte iteration at \
         unicode.rs:460"
    );
    // ucs_2 delegates to ucs_4 (unicode.rs:471) — same contract.
    assert_eq!(
        strwidth_ucs_2(&width_data, "héllo"),
        5,
        "strwidth_ucs_2 must mirror strwidth_ucs_4 on multibyte input"
    );
}

/// Astral-plane scalar (emoji) counts as ONE char, not its 4 UTF-8 bytes.
///
/// `"😀"` (U+1F600) is a single Rust `char` encoded as 4 UTF-8 bytes.
/// With fallback width 1, the width is 1 — the strongest byte-vs-char
/// discriminator since the byte/char ratio is 4:1 here.
///
/// Bug class caught: same `.chars()` → byte regression as above, but at
/// the maximum byte-width per scalar (4), so a byte-counting bug shows as
/// 4 instead of 1.
#[test]
fn strwidth_ucs_4_astral_char_counts_as_one() {
    let mut width_data = HashMap::new();
    width_data.insert("N".to_string(), 1);
    width_data.insert("Na".to_string(), 1);
    assert_eq!("😀".chars().count(), 1);
    assert_eq!("😀".len(), 4);
    assert_eq!(
        strwidth_ucs_4(&width_data, "😀"),
        1,
        "a single astral scalar (U+1F600) must measure 1, not its 4 UTF-8 \
         bytes; a result of 4 means byte iteration replaced `.chars()`"
    );
}
