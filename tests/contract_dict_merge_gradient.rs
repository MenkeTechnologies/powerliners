// vim:fileencoding=utf-8:noet
//! Contract tests for previously-uncovered surfaces in dict-merge primitives
//! and Colorscheme gradient resolution.
//!
//! Targets:
//!   - `mergedicts(remove=true)` at DEPTH > 1: REMOVE_THIS_KEY nested two
//!     levels deep correctly deletes the target leaf (recursive descent pin).
//!   - `mergedicts(remove=false)` does NOT delete keys flagged with
//!     REMOVE_THIS_KEY — the sentinel becomes a literal value instead. Pins
//!     the `remove` flag semantics.
//!   - `mergedicts_copy` with overlapping nested keys: both inputs preserved
//!     unchanged, output merged with d2 winning. Pin against accidental
//!     in-place mutation.
//!   - `mergedefaults` 3-level nested merge: d1 wins at every depth where it
//!     has a value; d2 fills only gaps.
//!   - `updated` overwrites existing key (last-write-wins) without mutating
//!     the source dict.
//!   - `add_spaces_center(0, ...)` returns contents unchanged (already pinned)
//!     and `add_spaces_center(4, ...)` puts 2 spaces on each side (no extra).
//!   - `expand_functions('l')` returns the add_spaces_RIGHT fn (NOT left!) —
//!     because padding goes on the right to make text APPEAR left-aligned.
//!     This counter-intuitive mapping is the most likely refactor regression.
//!
//! Earlier rounds pinned:
//!   - pick_gradient_value banker's rounding (NOT mergedicts depth semantics)
//!   - colorscheme.new with empty config (NOT mergedicts_copy non-mutation)
//!   - get_attrs_flag combining all three (NOT expand_functions inverted map)
//!   - add_spaces_center zero/odd amounts (NOT even-amount split)

use powerliners::lib::dict::{
    mergedefaults, mergedicts, mergedicts_copy, updated, REMOVE_THIS_KEY,
};
use powerliners::theme::{add_spaces_center, add_spaces_left, add_spaces_right, expand_functions};
use serde_json::{json, Map, Value};

fn obj(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => panic!("not an object: {v:?}"),
    }
}

/// `mergedicts(remove=true)` with REMOVE_THIS_KEY nested two levels deep
/// must delete the target leaf in the deep child dict.
#[test]
fn test_mergedicts_remove_this_key_at_depth_two_deletes_leaf() {
    let mut d1 = obj(json!({
        "level1": {
            "level2": {"keep": 1, "kill": 2}
        }
    }));
    let mut leaf = Map::new();
    leaf.insert("kill".to_string(), REMOVE_THIS_KEY());
    let mut level2 = Map::new();
    level2.insert("level2".to_string(), Value::Object(leaf));
    let mut d2 = Map::new();
    d2.insert("level1".to_string(), Value::Object(level2));

    mergedicts(&mut d1, d2, true);
    assert_eq!(
        Value::Object(d1),
        json!({"level1": {"level2": {"keep": 1}}}),
        "REMOVE_THIS_KEY at depth 2 must delete only the targeted leaf"
    );
}

/// `mergedicts(remove=false)` does NOT delete: REMOVE_THIS_KEY becomes a
/// literal value (the sentinel JSON object).
#[test]
fn test_mergedicts_remove_false_keeps_sentinel_as_literal_value() {
    let mut d1 = obj(json!({"a": 1, "b": 2}));
    let mut d2 = Map::new();
    d2.insert("b".to_string(), REMOVE_THIS_KEY());
    mergedicts(&mut d1, d2, false);
    let b = d1.get("b").expect("b must still exist when remove=false");
    assert!(
        b.is_object(),
        "REMOVE_THIS_KEY with remove=false stays as sentinel object; got {b:?}"
    );
    // The sentinel object carries the marker.
    assert_eq!(
        b.get("__powerliners_remove_this_key__"),
        Some(&Value::Bool(true)),
        "sentinel marker must be preserved"
    );
}

/// `mergedicts_copy` with overlapping nested keys: neither input mutated;
/// result is the deep-merged tree with d2 winning leaves.
#[test]
fn test_mergedicts_copy_non_mutating_with_overlapping_nested_keys() {
    let d1 = obj(json!({"a": {"x": 1, "y": 2}, "b": 99}));
    let d2 = obj(json!({"a": {"y": 200, "z": 300}, "c": 5}));
    let d1_before = d1.clone();
    let d2_before = d2.clone();
    let merged = mergedicts_copy(&d1, d2.clone());
    assert_eq!(d1, d1_before, "d1 must not be mutated");
    assert_eq!(d2, d2_before, "d2 must not be mutated");
    assert_eq!(
        Value::Object(merged),
        json!({"a": {"x": 1, "y": 200, "z": 300}, "b": 99, "c": 5}),
        "merged result must have d2 winning on overlap, gaps filled from both"
    );
}

/// `mergedefaults` 3-level nested: d1 keeps every value it has; d2 only fills
/// gaps at the deepest level.
#[test]
fn test_mergedefaults_three_level_nested_d1_wins_everywhere() {
    let mut d1 = obj(json!({
        "l1": {"l2": {"l3a": "d1value"}}
    }));
    let d2 = obj(json!({
        "l1": {"l2": {"l3a": "d2value", "l3b": "fill"}, "l2_new": "added"},
        "l1_new": "top_added"
    }));
    mergedefaults(&mut d1, d2);
    assert_eq!(
        Value::Object(d1),
        json!({
            "l1": {
                "l2": {"l3a": "d1value", "l3b": "fill"},
                "l2_new": "added"
            },
            "l1_new": "top_added"
        }),
        "mergedefaults must keep d1's l3a and fill only gaps"
    );
}

/// `updated` overwrites existing key (last-write-wins) without mutating source.
#[test]
fn test_updated_overwrites_existing_key_without_mutating_source() {
    let d = obj(json!({"a": 1, "b": 2}));
    let r = updated(&d, vec![("a".to_string(), json!(99))]);
    assert_eq!(d.get("a"), Some(&json!(1)), "source must be unmutated");
    assert_eq!(
        r.get("a"),
        Some(&json!(99)),
        "result must have overwritten value"
    );
    assert_eq!(r.get("b"), Some(&json!(2)), "untouched key passes through");
}

/// `add_spaces_center(4, ...)` splits 4 evenly: 2 spaces left + 2 spaces right
/// (no remainder).
#[test]
fn test_add_spaces_center_even_amount_splits_evenly_no_remainder() {
    let mut seg = Map::new();
    seg.insert("contents".to_string(), Value::from("AB"));
    let out = add_spaces_center(&(), 4, &seg);
    assert_eq!(
        out, "  AB  ",
        "amount=4 must yield 2 left + 2 right; got {out:?}"
    );
}

/// `expand_functions('l')` returns the right-padding function (inverse mapping
/// per upstream powerline-status). This is the most counter-intuitive part of
/// the alignment system.
#[test]
fn test_expand_functions_l_align_returns_right_padding_function() {
    // 'l' (left-aligned text) needs padding ON THE RIGHT — so the function
    // returned must be add_spaces_right.
    let f_l = expand_functions('l').expect("'l' must map to a function");
    let f_r = expand_functions('r').expect("'r' must map to a function");
    let mut seg = Map::new();
    seg.insert("contents".to_string(), Value::from("X"));
    // f('l') with amount=3 must put 3 spaces after X → "X   ".
    let out_l = f_l(&(), 3, &seg);
    assert_eq!(
        out_l, "X   ",
        "'l' must pad right (X then 3 spaces); got {out_l:?}"
    );
    // f('r') with amount=3 must put 3 spaces before X → "   X".
    let out_r = f_r(&(), 3, &seg);
    assert_eq!(
        out_r, "   X",
        "'r' must pad left (3 spaces then X); got {out_r:?}"
    );
}

/// `add_spaces_left` with missing `contents` key produces only spaces (no
/// panic). Pins the unwrap_or("") fallback in the segment getter.
#[test]
fn test_add_spaces_left_missing_contents_yields_only_spaces_no_panic() {
    let empty: Map<String, Value> = Map::new();
    let out = add_spaces_left(&(), 5, &empty);
    assert_eq!(
        out, "     ",
        "missing contents must yield amount-spaces only; got {out:?}"
    );
    let out_right = add_spaces_right(&(), 3, &empty);
    assert_eq!(
        out_right, "   ",
        "missing contents in add_spaces_right must also yield only spaces"
    );
}
