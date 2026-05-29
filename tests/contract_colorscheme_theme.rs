//! Contract tests for previously-uncovered powerliners colorscheme + theme surfaces.
//!
//! Targets:
//! - `pick_gradient_value` at banker's-rounding tie boundary matches Python `round()`
//! - `pick_gradient_value` with single-element list returns that element for any %
//! - `Colorscheme::new` with empty colors_config produces empty colors+gradients
//! - `Colorscheme::new` parses a gradient with explicit (cterm_list, hex_list)
//! - `get_attrs_flag` combines all three attrs (bold + italic + underline)
//! - `add_spaces_center` with amount=0 leaves contents unchanged
//! - `add_spaces_center` with odd amount puts extra space on the LEFT
//! - `expand_functions` returns None for unknown alignment char

use powerliners::ported::colorscheme::{
    get_attrs_flag, pick_gradient_value, Colorscheme, ATTR_BOLD, ATTR_ITALIC, ATTR_UNDERLINE,
};
use powerliners::ported::theme::{add_spaces_center, expand_functions};
use serde_json::{json, Map, Value};

#[test]
fn test_pick_gradient_value_bankers_rounding_at_tie() {
    // grad_list of length 3 → indices 0, 1, 2.
    // At gradient_level=25.0: raw = 25 * 2 / 100 = 0.5 → bankers round → 0 (even).
    // At gradient_level=75.0: raw = 75 * 2 / 100 = 1.5 → bankers round → 2 (even).
    // Python `round(0.5) == 0` and `round(1.5) == 2`.
    let grad = vec![10u64, 20, 30];
    assert_eq!(
        pick_gradient_value(&grad, 25.0),
        10,
        "banker's rounding of 0.5 → 0 (even); should return first element"
    );
    assert_eq!(
        pick_gradient_value(&grad, 75.0),
        30,
        "banker's rounding of 1.5 → 2 (even); should return last element"
    );
}

#[test]
fn test_pick_gradient_value_single_element_list_returns_only_element() {
    // grad_list of length 1: raw = level * 0 / 100 = 0 → index 0.
    let grad = vec![42u64];
    for &level in &[0.0, 50.0, 100.0] {
        assert_eq!(
            pick_gradient_value(&grad, level),
            42,
            "single-element list must return that element at level {level}"
        );
    }
}

#[test]
fn test_colorscheme_new_with_empty_colors_config_produces_empty_collections() {
    let cs_config = Map::new();
    let colors_config = Map::new();
    let cs = Colorscheme::new(&cs_config, &colors_config);
    assert!(cs.colors.is_empty(), "no colors_config → empty colors map");
    assert!(
        cs.gradients.is_empty(),
        "no gradients in config → empty gradients map"
    );
    assert!(cs.groups.is_empty(), "no groups → empty");
    assert!(cs.translations.is_empty(), "no translations → empty");
}

#[test]
fn test_colorscheme_new_parses_explicit_gradient_with_hex_list() {
    // gradient with both cterm_list and hex_str_list (2-tuple form).
    let cs_config = json!({"groups": {}}).as_object().unwrap().clone();
    let colors_config = json!({
        "colors": {},
        "gradients": {
            "g": [[1, 2, 3], ["ff0000", "00ff00", "0000ff"]]
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let cs = Colorscheme::new(&cs_config, &colors_config);
    let g = cs
        .gradients
        .get("g")
        .expect("gradient `g` exists")
        .as_array()
        .expect("gradient is an array");
    assert_eq!(
        g.len(),
        2,
        "gradient must be 2-element [cterm_list, hex_list]"
    );
    let hex_list = g[1].as_array().expect("hex list is an array");
    assert_eq!(hex_list.len(), 3, "3 hex values");
    assert_eq!(
        hex_list[0].as_u64(),
        Some(0xff0000),
        "first hex must parse to 0xff0000"
    );
    assert_eq!(hex_list[2].as_u64(), Some(0x0000ff));
}

#[test]
fn test_get_attrs_flag_combines_all_three() {
    let attrs = vec![
        "bold".to_string(),
        "italic".to_string(),
        "underline".to_string(),
    ];
    assert_eq!(
        get_attrs_flag(&attrs),
        ATTR_BOLD | ATTR_ITALIC | ATTR_UNDERLINE,
        "all three attrs must OR together"
    );
}

#[test]
fn test_add_spaces_center_with_zero_amount_returns_contents_unchanged() {
    let mut seg = Map::new();
    seg.insert("contents".to_string(), Value::String("hi".to_string()));
    let out = add_spaces_center(&(), 0, &seg);
    assert_eq!(out, "hi", "amount=0 must not add any spaces; got {out:?}");
}

#[test]
fn test_add_spaces_center_with_odd_amount_puts_extra_space_on_left() {
    // Python: amount, remainder = divmod(3, 2) → (1, 1) → " " * (1+1) + contents + " " * 1
    let mut seg = Map::new();
    seg.insert("contents".to_string(), Value::String("x".to_string()));
    let out = add_spaces_center(&(), 3, &seg);
    assert_eq!(
        out, "  x ",
        "odd amount=3 should yield 2 spaces left + 1 space right; got {out:?}"
    );
}

#[test]
fn test_expand_functions_unknown_alignment_returns_none() {
    // Only 'l', 'r', 'c' are valid; anything else must produce None.
    for ch in ['x', 'y', 'L', 'R', 'C', ' ', '\0'] {
        assert!(
            expand_functions(ch).is_none(),
            "expand_functions({ch:?}) must be None for unknown alignment"
        );
    }
}
