// vim:fileencoding=utf-8:noet
//! Port of `powerline/colorscheme.py`.
//!
//! Colorscheme resolution — translates highlight-group names into
//! fg/bg/attrs triples that renderers emit as ANSI escape sequences.
//!
//! The full upstream file is ~150 lines; the Rust port retains the
//! same class shape and the 256-entry `cterm_to_hex` table.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

// from copy import copy                              // py:4
// (Rust: explicit .clone() at call sites.)

// from powerline.lib.unicode import unicode          // py:6
// (Rust: all strings are UTF-8 `String`/`&str`; no Py2/Py3 shim needed.)

use serde_json::{Map, Value};

/// Port of module-level binding `DEFAULT_MODE_KEY` from
/// `powerline/colorscheme.py:9`.
///
/// Python: `DEFAULT_MODE_KEY = None` — sentinel used as the dict key
/// for "no specific mode" highlight definitions.
#[allow(non_upper_case_globals)]
pub const DEFAULT_MODE_KEY: Option<&str> = None; // py:9

/// Port of module-level binding `ATTR_BOLD` from `powerline/colorscheme.py:10`.
#[allow(non_upper_case_globals)]
pub const ATTR_BOLD: u32 = 1; // py:10

/// Port of module-level binding `ATTR_ITALIC` from `powerline/colorscheme.py:11`.
#[allow(non_upper_case_globals)]
pub const ATTR_ITALIC: u32 = 2; // py:11

/// Port of module-level binding `ATTR_UNDERLINE` from `powerline/colorscheme.py:12`.
#[allow(non_upper_case_globals)]
pub const ATTR_UNDERLINE: u32 = 4; // py:12

/// Port of `get_attrs_flag()` from `powerline/colorscheme.py:15`.
///
/// Convert an attribute array to a renderer flag.
pub fn get_attrs_flag(attrs: &[String]) -> u32 {
    let mut attrs_flag: u32 = 0; // py:17
    if attrs.iter().any(|a| a == "bold") {
        // py:18  if 'bold' in attrs:
        attrs_flag |= ATTR_BOLD; // py:19
    }
    if attrs.iter().any(|a| a == "italic") {
        // py:20
        attrs_flag |= ATTR_ITALIC; // py:21
    }
    if attrs.iter().any(|a| a == "underline") {
        // py:22
        attrs_flag |= ATTR_UNDERLINE; // py:23
    }
    attrs_flag // py:24
}

/// Port of `pick_gradient_value()` from `powerline/colorscheme.py:27`.
///
/// Given a list of colors and gradient percent, return a color that
/// should be used.
///
/// Note: gradient level is not checked for being inside [0, 100]
/// interval (matches Python behaviour).
///
/// **Banker's rounding**: Python 3's `round()` rounds half-to-even
/// (`round(2.5) == 2`, `round(3.5) == 4`). Rust's `f64::round` rounds
/// half-away-from-zero (`(2.5_f64).round() == 3.0`). To match upstream
/// byte-for-byte we use `round_ties_even` (Rust 1.77+).
pub fn pick_gradient_value(grad_list: &[u64], gradient_level: f64) -> u64 {
    // py:32  grad_list[int(round(gradient_level * (len(grad_list) - 1) / 100))]
    let raw = gradient_level * (grad_list.len() as f64 - 1.0) / 100.0;
    let idx = py_round(raw) as usize;
    grad_list[idx]
}

/// Helper: Python 3 `round()` semantics (banker's rounding / round
/// half to even). Used at every site where the port needs to match
/// `int(round(...))` byte-for-byte against upstream output.
fn py_round(x: f64) -> f64 {
    // Rust 1.77+ stable: f64::round_ties_even
    x.round_ties_even()
}

/// Port of `class Colorscheme` from `powerline/colorscheme.py:35`.
///
/// Holds the parsed colorscheme + colors config, plus computed
/// gradient/group dicts. Per-instance state matches Python's
/// `self.colors`, `self.gradients`, `self.groups`, `self.translations`.
pub struct Colorscheme {
    /// Python: `self.colors` (`dict[str, (int_cterm, int_hex)]`)
    /// — py:38
    pub colors: Map<String, Value>,
    /// Python: `self.gradients` — py:39
    pub gradients: Map<String, Value>,
    /// Python: `self.groups` — py:41 from colorscheme_config['groups']
    pub groups: Map<String, Value>,
    /// Python: `self.translations` — py:42 from
    /// colorscheme_config.get('mode_translations', {})
    pub translations: Map<String, Value>,
}

impl Colorscheme {
    /// Port of `Colorscheme.__init__()` from `powerline/colorscheme.py:36`.
    ///
    /// Initialize a colorscheme.
    pub fn new(
        colorscheme_config: &Map<String, Value>,
        colors_config: &Map<String, Value>,
    ) -> Self {
        let mut colors: Map<String, Value> = Map::new(); // py:38
        let mut gradients: Map<String, Value> = Map::new(); // py:39

        // py:41  self.groups = colorscheme_config['groups']
        let groups = colorscheme_config
            .get("groups")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // py:42  self.translations = colorscheme_config.get('mode_translations', {})
        let translations = colorscheme_config
            .get("mode_translations")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        // py:44  Create a dict of color tuples with both a cterm and hex value
        if let Some(colors_in) = colors_config.get("colors").and_then(|v| v.as_object()) {
            for (color_name, color) in colors_in {
                // py:45
                let pair = match color {
                    // py:46-47  color is iterable: (cterm_int, hex_str)
                    Value::Array(a) if a.len() == 2 => {
                        let cterm = a[0].as_i64().unwrap_or(0);
                        let hex_str = a[1].as_str().unwrap_or("0");
                        let hex =
                            u64::from_str_radix(hex_str.trim_start_matches("0x"), 16).unwrap_or(0);
                        Value::Array(vec![Value::from(cterm), Value::from(hex)])
                    }
                    // py:48-49  TypeError fallback: lookup cterm in cterm_to_hex
                    Value::Number(n) => {
                        let cterm = n.as_u64().unwrap_or(0) as usize;
                        let hex = cterm_to_hex.get(cterm).copied().unwrap_or(0);
                        Value::Array(vec![Value::from(cterm as u64), Value::from(hex)])
                    }
                    _ => Value::Null,
                };
                colors.insert(color_name.clone(), pair); // py:47/49
            }
        }

        // py:54  Create a dict of gradient names with two lists: for cterm and hex
        if let Some(gradients_in) = colors_config.get("gradients").and_then(|v| v.as_object()) {
            for (gradient_name, gradient) in gradients_in {
                // py:54
                let arr = gradient.as_array().cloned().unwrap_or_default();
                let entry: Value = if arr.len() == 2 {
                    // py:55
                    // py:56-57  (cterm_list, hex_str_list)
                    let cterm_list = arr[0].clone();
                    let hex_list: Vec<Value> = arr[1]
                        .as_array()
                        .map(|hl| {
                            hl.iter()
                                .map(|c| {
                                    let s = c.as_str().unwrap_or("0");
                                    Value::from(
                                        u64::from_str_radix(s.trim_start_matches("0x"), 16)
                                            .unwrap_or(0),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Value::Array(vec![cterm_list, Value::Array(hex_list)])
                } else {
                    // py:58-60  single cterm list; derive hex from cterm_to_hex
                    let cterm_list = arr[0].clone();
                    let hex_list: Vec<Value> = cterm_list
                        .as_array()
                        .map(|cl| {
                            cl.iter()
                                .map(|c| {
                                    let i = c.as_u64().unwrap_or(0) as usize;
                                    Value::from(cterm_to_hex.get(i).copied().unwrap_or(0))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    Value::Array(vec![cterm_list, Value::Array(hex_list)])
                };
                gradients.insert(gradient_name.clone(), entry);
            }
        }

        Colorscheme {
            colors,
            gradients,
            groups,
            translations,
        }
    }

    /// Port of `Colorscheme.get_gradient()` from `powerline/colorscheme.py:62`.
    pub fn get_gradient(&self, gradient: &str, gradient_level: f64) -> Value {
        if let Some(g) = self.gradients.get(gradient) {
            // py:63
            // py:64  tuple of pick_gradient_value over each sub-list
            if let Value::Array(pair) = g {
                let mapped: Vec<Value> = pair
                    .iter()
                    .map(|grad_list| {
                        let list: Vec<u64> = grad_list
                            .as_array()
                            .map(|l| l.iter().filter_map(|v| v.as_u64()).collect())
                            .unwrap_or_default();
                        if list.is_empty() {
                            Value::Null
                        } else {
                            Value::from(pick_gradient_value(&list, gradient_level))
                        }
                    })
                    .collect();
                return Value::Array(mapped);
            }
            g.clone()
        } else {
            // py:65-66
            self.colors.get(gradient).cloned().unwrap_or(Value::Null)
        }
    }

    /// Port of `Colorscheme.get_group_props()` from `powerline/colorscheme.py:68`.
    ///
    /// `translate_colors` is forwarded through recursive calls and ultimately
    /// consumed in the non-recursive branch; clippy's `only_used_in_recursion`
    /// fires on the recursive paths but the parameter is real upstream behavior.
    #[allow(clippy::only_used_in_recursion)]
    pub fn get_group_props(
        &self,
        mode: Option<&str>,
        trans: &Map<String, Value>,
        group: &Value,
        translate_colors: bool,
    ) -> Option<Value> {
        match group {
            Value::String(group_name) => {
                // py:69  isinstance(group, str)
                let trans_groups = trans.get("groups").and_then(|v| v.as_object());
                if let Some(g) = trans_groups.and_then(|tg| tg.get(group_name)) {
                    // py:80  return self.get_group_props(mode, trans, group_props, False)
                    self.get_group_props(mode, trans, g, false)
                } else if let Some(g) = self.groups.get(group_name) {
                    // py:78  return self.get_group_props(mode, trans, group_props, True)
                    self.get_group_props(mode, trans, g, true)
                } else {
                    // py:76  return None
                    None
                }
            }
            _ => {
                // py:81
                if translate_colors {
                    // py:82
                    let mut group_props = group.clone(); // py:83  copy(group)
                    if let (Some(ctrans), Value::Object(gp)) = (
                        trans.get("colors").and_then(|v| v.as_object()),
                        &mut group_props,
                    ) {
                        for key in ["fg", "bg"] {
                            // py:89
                            if let Some(cur) = gp.get(key) {
                                if let Some(new) = ctrans.get(cur.as_str().unwrap_or("")) {
                                    gp.insert(key.to_string(), new.clone()); // py:91
                                }
                            }
                        }
                    }
                    Some(group_props) // py:94
                } else {
                    // py:95
                    Some(group.clone()) // py:96
                }
            }
        }
    }

    /// Port of `Colorscheme.get_highlighting()` from `powerline/colorscheme.py:98`.
    pub fn get_highlighting(
        &self,
        groups: &[String],
        mode: Option<&str>,
        gradient_level: Option<f64>,
    ) -> Result<Map<String, Value>, String> {
        // py:99  trans = self.translations.get(mode, {})
        let trans: Map<String, Value> = mode
            .and_then(|m| self.translations.get(m))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let mut group_props: Option<Value> = None; // py:100-103
        for group in groups {
            let gp = self.get_group_props(mode, &trans, &Value::String(group.clone()), true);
            if gp.is_some() {
                group_props = gp;
                break;
            }
        }
        let group_props = group_props.ok_or_else(|| {
            // py:104-105
            format!(
                "Highlighting groups not found in colorscheme: {}",
                groups.join(", ")
            )
        })?;

        let gp_obj = group_props.as_object().cloned().unwrap_or_default();

        // py:107-110  pick_color selection
        let pick = |key: &str| -> Value {
            let color_name = gp_obj.get(key).and_then(|v| v.as_str()).unwrap_or("");
            match gradient_level {
                None => self.colors.get(color_name).cloned().unwrap_or(Value::Null),
                Some(level) => self.get_gradient(color_name, level),
            }
        };

        let attrs_vec: Vec<String> = gp_obj
            .get("attrs")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        // py:112-116  return dict with fg/bg/attrs
        let mut out = Map::new();
        out.insert("fg".to_string(), pick("fg"));
        out.insert("bg".to_string(), pick("bg"));
        out.insert("attrs".to_string(), Value::from(get_attrs_flag(&attrs_vec)));
        Ok(out)
    }
}

/// Port of module-level binding `cterm_to_hex` from
/// `powerline/colorscheme.py:120`.
///
/// 256-entry table mapping 8-bit terminal color indices to 24-bit hex
/// values. Layout exactly mirrors upstream py:120-147 (rows of 10,
/// last row 6 entries).
#[allow(non_upper_case_globals)]
pub const cterm_to_hex: [u64; 256] = [
    // py:121  row 0 — indices 0..10
    0x000000, 0xc00000, 0x008000, 0x804000, 0x0000c0, 0xc000c0, 0x008080, 0xc0c0c0, 0x808080,
    0xff6060, // py:122  row 1
    0x00ff00, 0xffff00, 0x8080ff, 0xff40ff, 0x00ffff, 0xffffff, 0x000000, 0x00005f, 0x000087,
    0x0000af, // py:123  row 2
    0x0000d7, 0x0000ff, 0x005f00, 0x005f5f, 0x005f87, 0x005faf, 0x005fd7, 0x005fff, 0x008700,
    0x00875f, // py:124  row 3
    0x008787, 0x0087af, 0x0087d7, 0x0087ff, 0x00af00, 0x00af5f, 0x00af87, 0x00afaf, 0x00afd7,
    0x00afff, // py:125  row 4
    0x00d700, 0x00d75f, 0x00d787, 0x00d7af, 0x00d7d7, 0x00d7ff, 0x00ff00, 0x00ff5f, 0x00ff87,
    0x00ffaf, // py:126  row 5
    0x00ffd7, 0x00ffff, 0x5f0000, 0x5f005f, 0x5f0087, 0x5f00af, 0x5f00d7, 0x5f00ff, 0x5f5f00,
    0x5f5f5f, // py:127  row 6
    0x5f5f87, 0x5f5faf, 0x5f5fd7, 0x5f5fff, 0x5f8700, 0x5f875f, 0x5f8787, 0x5f87af, 0x5f87d7,
    0x5f87ff, // py:128  row 7
    0x5faf00, 0x5faf5f, 0x5faf87, 0x5fafaf, 0x5fafd7, 0x5fafff, 0x5fd700, 0x5fd75f, 0x5fd787,
    0x5fd7af, // py:129  row 8
    0x5fd7d7, 0x5fd7ff, 0x5fff00, 0x5fff5f, 0x5fff87, 0x5fffaf, 0x5fffd7, 0x5fffff, 0x870000,
    0x87005f, // py:130  row 9
    0x870087, 0x8700af, 0x8700d7, 0x8700ff, 0x875f00, 0x875f5f, 0x875f87, 0x875faf, 0x875fd7,
    0x875fff, // py:131  row 10
    0x878700, 0x87875f, 0x878787, 0x8787af, 0x8787d7, 0x8787ff, 0x87af00, 0x87af5f, 0x87af87,
    0x87afaf, // py:132  row 11
    0x87afd7, 0x87afff, 0x87d700, 0x87d75f, 0x87d787, 0x87d7af, 0x87d7d7, 0x87d7ff, 0x87ff00,
    0x87ff5f, // py:133  row 12
    0x87ff87, 0x87ffaf, 0x87ffd7, 0x87ffff, 0xaf0000, 0xaf005f, 0xaf0087, 0xaf00af, 0xaf00d7,
    0xaf00ff, // py:134  row 13
    0xaf5f00, 0xaf5f5f, 0xaf5f87, 0xaf5faf, 0xaf5fd7, 0xaf5fff, 0xaf8700, 0xaf875f, 0xaf8787,
    0xaf87af, // py:135  row 14
    0xaf87d7, 0xaf87ff, 0xafaf00, 0xafaf5f, 0xafaf87, 0xafafaf, 0xafafd7, 0xafafff, 0xafd700,
    0xafd75f, // py:136  row 15
    0xafd787, 0xafd7af, 0xafd7d7, 0xafd7ff, 0xafff00, 0xafff5f, 0xafff87, 0xafffaf, 0xafffd7,
    0xafffff, // py:137  row 16
    0xd70000, 0xd7005f, 0xd70087, 0xd700af, 0xd700d7, 0xd700ff, 0xd75f00, 0xd75f5f, 0xd75f87,
    0xd75faf, // py:138  row 17
    0xd75fd7, 0xd75fff, 0xd78700, 0xd7875f, 0xd78787, 0xd787af, 0xd787d7, 0xd787ff, 0xd7af00,
    0xd7af5f, // py:139  row 18
    0xd7af87, 0xd7afaf, 0xd7afd7, 0xd7afff, 0xd7d700, 0xd7d75f, 0xd7d787, 0xd7d7af, 0xd7d7d7,
    0xd7d7ff, // py:140  row 19
    0xd7ff00, 0xd7ff5f, 0xd7ff87, 0xd7ffaf, 0xd7ffd7, 0xd7ffff, 0xff0000, 0xff005f, 0xff0087,
    0xff00af, // py:141  row 20
    0xff00d7, 0xff00ff, 0xff5f00, 0xff5f5f, 0xff5f87, 0xff5faf, 0xff5fd7, 0xff5fff, 0xff8700,
    0xff875f, // py:142  row 21
    0xff8787, 0xff87af, 0xff87d7, 0xff87ff, 0xffaf00, 0xffaf5f, 0xffaf87, 0xffafaf, 0xffafd7,
    0xffafff, // py:143  row 22
    0xffd700, 0xffd75f, 0xffd787, 0xffd7af, 0xffd7d7, 0xffd7ff, 0xffff00, 0xffff5f, 0xffff87,
    0xffffaf, // py:144  row 23 — start of greyscale ramp (24 entries)
    0xffffd7, 0xffffff, 0x080808, 0x121212, 0x1c1c1c, 0x262626, 0x303030, 0x3a3a3a, 0x444444,
    0x4e4e4e, // py:145  row 24
    0x585858, 0x626262, 0x6c6c6c, 0x767676, 0x808080, 0x8a8a8a, 0x949494, 0x9e9e9e, 0xa8a8a8,
    0xb2b2b2, // py:146  row 25 (final 6 entries)
    0xbcbcbc, 0xc6c6c6, 0xd0d0d0, 0xdadada, 0xe4e4e4, 0xeeeeee,
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// ATTR_* constants match upstream values.
    #[test]
    fn attr_constants_match_upstream() {
        assert_eq!(ATTR_BOLD, 1);
        assert_eq!(ATTR_ITALIC, 2);
        assert_eq!(ATTR_UNDERLINE, 4);
    }

    /// cterm_to_hex table has exactly 256 entries (8-bit color space).
    #[test]
    fn cterm_to_hex_has_256_entries() {
        assert_eq!(cterm_to_hex.len(), 256);
    }

    /// cterm_to_hex spot-check known values from upstream.
    #[test]
    fn cterm_to_hex_spot_check() {
        // Black at index 0
        assert_eq!(cterm_to_hex[0], 0x000000);
        // White at index 15
        assert_eq!(cterm_to_hex[15], 0xffffff);
        // Final greyscale entry
        assert_eq!(cterm_to_hex[255], 0xeeeeee);
    }

    /// get_attrs_flag combines flags via OR.
    #[test]
    fn get_attrs_flag_combines_bits() {
        let attrs = vec!["bold".to_string(), "underline".to_string()];
        assert_eq!(get_attrs_flag(&attrs), ATTR_BOLD | ATTR_UNDERLINE);
    }

    /// get_attrs_flag empty list returns 0.
    #[test]
    fn get_attrs_flag_empty_is_zero() {
        let attrs: Vec<String> = vec![];
        assert_eq!(get_attrs_flag(&attrs), 0);
    }

    /// pick_gradient_value at 0% returns first, at 100% returns last.
    #[test]
    fn pick_gradient_value_endpoints() {
        let grad = vec![10u64, 20, 30, 40, 50];
        assert_eq!(pick_gradient_value(&grad, 0.0), 10);
        assert_eq!(pick_gradient_value(&grad, 100.0), 50);
    }

    /// Colorscheme::new parses a minimal config.
    #[test]
    fn colorscheme_new_minimal_config() {
        let colorscheme_config = json!({
            "groups": {"information:current": {"fg": "white", "bg": "blue"}}
        })
        .as_object()
        .unwrap()
        .clone();
        let colors_config = json!({
            "colors": {"white": [231, "ffffff"], "blue": [21, "0000ff"]},
            "gradients": {}
        })
        .as_object()
        .unwrap()
        .clone();
        let cs = Colorscheme::new(&colorscheme_config, &colors_config);
        assert_eq!(cs.groups.len(), 1);
        assert_eq!(cs.colors.len(), 2);
        // White color tuple should be (231, 0xffffff)
        let white = cs.colors.get("white").unwrap().as_array().unwrap();
        assert_eq!(white[0].as_u64().unwrap(), 231);
        assert_eq!(white[1].as_u64().unwrap(), 0xffffff);
    }

    /// get_highlighting resolves a group through the standard path.
    #[test]
    fn get_highlighting_resolves_group() {
        let colorscheme_config = json!({
            "groups": {"info": {"fg": "white", "bg": "blue", "attrs": ["bold"]}}
        })
        .as_object()
        .unwrap()
        .clone();
        let colors_config = json!({
            "colors": {"white": [231, "ffffff"], "blue": [21, "0000ff"]},
            "gradients": {}
        })
        .as_object()
        .unwrap()
        .clone();
        let cs = Colorscheme::new(&colorscheme_config, &colors_config);
        let hl = cs
            .get_highlighting(&["info".to_string()], None, None)
            .unwrap();
        assert_eq!(hl.get("attrs").unwrap().as_u64().unwrap(), ATTR_BOLD as u64);
        // fg should be the (cterm, hex) pair for "white"
        let fg = hl.get("fg").unwrap().as_array().unwrap();
        assert_eq!(fg[0].as_u64().unwrap(), 231);
        assert_eq!(fg[1].as_u64().unwrap(), 0xffffff);
    }
}
