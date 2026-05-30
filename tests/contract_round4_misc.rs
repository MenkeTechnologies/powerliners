// vim:fileencoding=utf-8:noet
//! Round 4 contract tests for previously-uncovered surfaces.
//!
//! Targets:
//!   - `mergedicts(remove=true)` with TOP-LEVEL REMOVE_THIS_KEY deletes a
//!     top-level key (depth-0 case — earlier rounds covered depth >= 2).
//!   - `mergedicts_copy(d1, empty_map)` returns an exact clone of d1 (empty
//!     overlay must not lose any keys or mutate structure).
//!   - `humanize_bytes(1023.0, "B", false)` returns "1023 B" — boundary just
//!     below the 1024 KiB threshold pins that the unit choice uses log(1024)
//!     correctly (1023.log(1024) < 1).
//!   - `humanize_bytes` with very large input (≥ 1024^6) clamps to the largest
//!     unit (PiB) — pins the `.min(unit_list.len() - 1)` cap.
//!   - `urllib_urlencode` with empty input → empty string (no iterations,
//!     no trailing separator).
//!   - `urllib_urlencode` with embedded `&` and `=` in value: each must be
//!     percent-encoded so the encoded pair round-trips unambiguously.
//!   - `add_spaces_left(2, seg)` puts exactly 2 spaces on the LEFT of contents.
//!   - `add_spaces_right(2, seg)` puts exactly 2 spaces on the RIGHT of contents.
//!   - `Theme::get_divider` with empty dividers returns None for any
//!     (side, type) pair — pins the empty-map early return.
//!
//! Earlier rounds pinned:
//!   - mergedicts(remove=true) at depth 2; mergedicts(remove=false) sentinel
//!     stays literal; mergedicts_copy non-mutation with overlapping nested keys
//!     (NOT top-level REMOVE_THIS_KEY; NOT empty-overlay clone)
//!   - humanize_bytes zero / 1 KiB / 1 MiB / 1 GiB (NOT 1023 B boundary;
//!     NOT > 1024^5 clamp)
//!   - urllib_urlencode simple / spaces / safe-chars (NOT empty input;
//!     NOT embedded `&`/`=` in value)
//!   - add_spaces_center (NOT left/right with non-zero amount)
//!   - get_divider nested lookup happy path (NOT empty-map for-any return)
//!
//! These tests pin DIFFERENT surfaces.

use powerliners::lib::dict::{mergedicts, mergedicts_copy, REMOVE_THIS_KEY};
use powerliners::lib::humanize_bytes::humanize_bytes;
use powerliners::lib::url::urllib_urlencode;
use powerliners::theme::{add_spaces_left, add_spaces_right, Theme};
use serde_json::{json, Map, Value};

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => panic!("not an object: {v:?}"),
    }
}

fn seg(contents: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("contents".to_string(), Value::String(contents.to_string()));
    m
}

/// `mergedicts(remove=true)` with REMOVE_THIS_KEY at the TOP level (depth 0)
/// must delete that key. Earlier rounds covered depth 2 only.
#[test]
fn test_mergedicts_remove_this_key_at_top_level_deletes_key() {
    let mut d1 = obj(json!({"keep": 1, "kill": 2, "also_keep": 3}));
    let mut d2 = Map::new();
    d2.insert("kill".to_string(), REMOVE_THIS_KEY());
    mergedicts(&mut d1, d2, true);
    assert_eq!(
        Value::Object(d1),
        json!({"keep": 1, "also_keep": 3}),
        "REMOVE_THIS_KEY at top level must delete the targeted key"
    );
}

/// `mergedicts_copy(d1, empty_map)` returns a clone of d1: empty overlay
/// must not alter any structure.
#[test]
fn test_mergedicts_copy_empty_overlay_returns_clone_of_d1() {
    let d1 = obj(json!({"a": 1, "b": {"nested": true}, "c": [1, 2, 3]}));
    let d1_before = d1.clone();
    let empty: Map<String, Value> = Map::new();
    let merged = mergedicts_copy(&d1, empty);
    assert_eq!(d1, d1_before, "d1 must not be mutated by empty overlay");
    assert_eq!(
        Value::Object(merged),
        Value::Object(d1),
        "empty overlay must produce a structural clone of d1"
    );
}

/// `humanize_bytes(1023.0, "B", false)` returns "1023 B" (just below the
/// 1024-byte KiB threshold). Pin the log-base-1024 boundary.
#[test]
fn test_humanize_bytes_1023_stays_in_bytes_unit() {
    let s = humanize_bytes(1023.0, "B", false);
    assert_eq!(
        s, "1023 B",
        "1023 bytes must remain in the bytes unit, not jump to KiB; got {s:?}"
    );
}

/// Very large input (above 1024^5 PiB) clamps to PiB (the largest unit in
/// `unit_list`). Pin the `.min(unit_list.len() - 1)` cap so a future enlarged
/// unit_list adds entries safely without breaking the clamp.
#[test]
fn test_humanize_bytes_very_large_clamps_to_largest_unit_pib() {
    // 1024^6 = 1024 PiB → expect "1024.00 PiB" or similar — the unit must be
    // PiB (last unit_list entry), not "EiB" or any larger nonexistent unit.
    let huge = 1024.0_f64.powi(6);
    let s = humanize_bytes(huge, "B", false);
    assert!(
        s.contains("PiB"),
        "input >= 1024^5 must clamp to PiB unit; got {s:?}"
    );
    assert!(
        !s.contains("EiB") && !s.contains("ZiB") && !s.contains("YiB"),
        "must not invent units beyond PiB; got {s:?}"
    );
}

/// `urllib_urlencode([])` → empty string. No trailing `&`, no leading `?`.
#[test]
fn test_urllib_urlencode_empty_iterable_returns_empty_string() {
    let pairs: Vec<(&str, &str)> = vec![];
    let s = urllib_urlencode(pairs);
    assert_eq!(
        s, "",
        "empty input must produce empty output (no trailing separator); got {s:?}"
    );
}

/// `urllib_urlencode` percent-encodes embedded `&` and `=` in values so the
/// encoded pair round-trips unambiguously. Pin against the regression where
/// the encoder would naively pass them through and break key/value splits.
#[test]
fn test_urllib_urlencode_percent_encodes_ampersand_and_equals_in_value() {
    let pairs: Vec<(&str, &str)> = vec![("k", "a&b=c")];
    let s = urllib_urlencode(pairs);
    assert!(
        !s.contains("&b="),
        "embedded & must be percent-encoded, not passed through; got {s:?}"
    );
    assert!(
        s.contains("%26") || s.contains("%26%3D"),
        "ampersand must encode to %26; got {s:?}"
    );
    assert!(
        s.contains("%3D") || s.contains("%26%3D"),
        "equals sign in value must encode to %3D; got {s:?}"
    );
}

/// `add_spaces_left(2, seg)` adds exactly 2 spaces on the LEFT of contents.
#[test]
fn test_add_spaces_left_with_amount_two_pads_two_spaces_left() {
    let s = seg("x");
    let out = add_spaces_left(&(), 2, &s);
    assert_eq!(
        out, "  x",
        "amount=2 must put 2 spaces on the left of 'x'; got {out:?}"
    );
}

/// `add_spaces_right(2, seg)` adds exactly 2 spaces on the RIGHT of contents.
#[test]
fn test_add_spaces_right_with_amount_two_pads_two_spaces_right() {
    let s = seg("x");
    let out = add_spaces_right(&(), 2, &s);
    assert_eq!(
        out, "x  ",
        "amount=2 must put 2 spaces on the right of 'x'; got {out:?}"
    );
}

/// `Theme::get_divider` on a Theme with empty dividers returns None for any
/// (side, kind) request. Pins the empty-dividers early-return path.
#[test]
fn test_theme_get_divider_empty_dividers_returns_none_for_any_side_or_kind() {
    let t = Theme::new();
    // Default Theme::new has empty dividers map.
    for side in ["left", "right", "garbage_side"] {
        for kind in ["soft", "hard", "unknown_kind"] {
            assert!(
                t.get_divider(side, kind).is_none(),
                "empty dividers must yield None for ({side}, {kind})"
            );
        }
    }
}
