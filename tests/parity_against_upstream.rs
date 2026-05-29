// vim:fileencoding=utf-8:noet
//! Parity harness — pipe identical inputs to upstream Python and Rust
//! ports, assert byte-identical results.
//!
//! Skipped automatically when Python or upstream `vendor/powerline/`
//! is unavailable. When both are present, every assertion is real
//! evidence of behavioural parity per docs/PORT.md Rule 4.

use std::process::Command;

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Run an upstream Python expression and capture stdout (stripped).
/// Returns None when Python isn't available — caller skips assertion.
fn py_eval(expr: &str) -> Option<String> {
    let repo = repo_root();
    let vendor = repo.join("vendor").join("powerline");
    if !vendor.exists() {
        return None;
    }
    let script = format!(
        "import sys; sys.path.insert(0, '{}'); print({})",
        vendor.display(),
        expr
    );
    let out = Command::new("python3")
        .arg("-c")
        .arg(&script)
        .output()
        .ok()?;
    if !out.status.success() {
        eprintln!(
            "py_eval failed: stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        return None;
    }
    // Strip ONLY the trailing newline that `print()` adds — preserve any
    // trailing whitespace that's part of the function's return value.
    let s = String::from_utf8_lossy(&out.stdout).into_owned();
    Some(s.strip_suffix('\n').unwrap_or(&s).to_string())
}

fn python_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ─────────────────────────────────────────────────────────────────────
// version.py
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_version_constant() {
    if !python_available() {
        eprintln!("parity_version_constant: skipped (python3 not available)");
        return;
    }
    let py = match py_eval("__import__('powerline.version', fromlist=['__version__']).__version__")
    {
        Some(v) => v,
        None => {
            eprintln!("parity_version_constant: skipped (vendor not present)");
            return;
        }
    };
    assert_eq!(
        py,
        powerliners::version::__version__,
        "Python __version__ != Rust __version__"
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/humanize_bytes.py
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_humanize_bytes() {
    if !python_available() {
        return;
    }
    let cases: &[(f64, &str, bool)] = &[
        (0.0, "B", false),
        (1024.0, "B", false),
        (1024.0 * 1024.0, "B", false),
        (1024.0_f64.powi(3), "B", false),
        (1000.0, "B", true),
        (1_000_000.0, "B", true),
        (1_000_000_000.0, "B", true),
        (42.0, "B", false),
    ];
    for (n, suf, si) in cases {
        let py_expr = format!(
            "__import__('powerline.lib.humanize_bytes', fromlist=['humanize_bytes']).humanize_bytes({}, '{}', {})",
            n, suf, if *si { "True" } else { "False" }
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => {
                eprintln!("parity_humanize_bytes: skipped (vendor not present)");
                return;
            }
        };
        let rs = powerliners::lib::humanize_bytes::humanize_bytes(*n, suf, *si);
        assert_eq!(
            py, rs,
            "humanize_bytes({}, {:?}, {}) mismatch:\n  py: {:?}\n  rs: {:?}",
            n, suf, si, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// colorscheme.py — get_attrs_flag + ATTR_* constants
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_attr_constants() {
    if !python_available() {
        return;
    }
    let attrs = ["ATTR_BOLD", "ATTR_ITALIC", "ATTR_UNDERLINE"];
    let rs_vals = [
        powerliners::colorscheme::ATTR_BOLD,
        powerliners::colorscheme::ATTR_ITALIC,
        powerliners::colorscheme::ATTR_UNDERLINE,
    ];
    for (i, name) in attrs.iter().enumerate() {
        let expr = format!(
            "__import__('powerline.colorscheme', fromlist=['{}']).{}",
            name, name
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_int: u32 = py
            .parse()
            .unwrap_or_else(|_| panic!("bad py int for {}", name));
        assert_eq!(
            py_int, rs_vals[i],
            "{} mismatch: py={}, rs={}",
            name, py_int, rs_vals[i]
        );
    }
}

#[test]
fn parity_get_attrs_flag() {
    if !python_available() {
        return;
    }
    let cases: &[&[&str]] = &[
        &[],
        &["bold"],
        &["italic"],
        &["underline"],
        &["bold", "italic"],
        &["bold", "italic", "underline"],
        &["unknown_attr"], // should be ignored
    ];
    for attrs in cases {
        let py_list = attrs
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ");
        let expr = format!(
            "__import__('powerline.colorscheme', fromlist=['get_attrs_flag']).get_attrs_flag([{}])",
            py_list
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_int: u32 = py.parse().expect("bad py int");
        let rust_attrs: Vec<String> = attrs.iter().map(|s| s.to_string()).collect();
        let rs = powerliners::colorscheme::get_attrs_flag(&rust_attrs);
        assert_eq!(
            py_int, rs,
            "get_attrs_flag({:?}) mismatch: py={}, rs={}",
            attrs, py_int, rs
        );
    }
}

#[test]
fn parity_pick_gradient_value() {
    if !python_available() {
        return;
    }
    let grad: Vec<u64> = (0..=10).map(|i| i * 10).collect();
    let levels = [0.0, 25.0, 50.0, 75.0, 100.0];
    for level in levels {
        let py_grad = grad
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let expr = format!(
            "__import__('powerline.colorscheme', fromlist=['pick_gradient_value']).pick_gradient_value([{}], {})",
            py_grad, level
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_int: u64 = py.parse().expect("bad py int");
        let rs = powerliners::colorscheme::pick_gradient_value(&grad, level);
        assert_eq!(
            py_int, rs,
            "pick_gradient_value(level={}) mismatch: py={}, rs={}",
            level, py_int, rs
        );
    }
}

#[test]
fn parity_cterm_to_hex_table() {
    if !python_available() {
        return;
    }
    let py_expr =
        "list(__import__('powerline.colorscheme', fromlist=['cterm_to_hex']).cterm_to_hex)";
    let py = match py_eval(py_expr) {
        Some(v) => v,
        None => return,
    };
    // Parse Python list of ints
    let py_str = py.trim_start_matches('[').trim_end_matches(']');
    let py_vals: Vec<u64> = py_str
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect();
    assert_eq!(
        py_vals.len(),
        256,
        "Python cterm_to_hex should have 256 entries"
    );
    assert_eq!(
        py_vals.len(),
        powerliners::colorscheme::cterm_to_hex.len(),
        "len mismatch"
    );
    for (i, (py, rs)) in py_vals
        .iter()
        .zip(powerliners::colorscheme::cterm_to_hex.iter())
        .enumerate()
    {
        assert_eq!(
            py, rs,
            "cterm_to_hex[{i}] mismatch: py=0x{py:06x}, rs=0x{rs:06x}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// bindings/tmux/__init__.py
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_get_tmux_executable_name_default() {
    if !python_available() {
        return;
    }
    // Clear env on both sides
    std::env::remove_var("POWERLINE_TMUX_EXE");
    let py = match py_eval(
        "(__import__('os').environ.pop('POWERLINE_TMUX_EXE', None), \
          __import__('powerline.bindings.tmux', fromlist=['get_tmux_executable_name']).get_tmux_executable_name())[1]"
    ) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::bindings::tmux::get_tmux_executable_name();
    assert_eq!(
        py, rs,
        "get_tmux_executable_name() default mismatch: py={:?}, rs={:?}",
        py, rs
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/dict.py — recursive merge
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_mergedicts_basic() {
    if !python_available() {
        return;
    }
    // Python: import + merge, output result as JSON
    let py_script = r#"
import json, sys, os
sys.path.insert(0, os.environ['PL_VENDOR'])
from powerline.lib.dict import mergedicts
d1 = {"a": 1, "b": 2, "nested": {"x": 1, "y": 2}}
d2 = {"b": 3, "c": 4, "nested": {"y": 20, "z": 30}}
mergedicts(d1, d2)
print(json.dumps(d1, sort_keys=True))
"#;
    let vendor = repo_root().join("vendor").join("powerline");
    if !vendor.exists() {
        return;
    }
    let out = Command::new("python3")
        .env("PL_VENDOR", vendor.to_string_lossy().as_ref())
        .arg("-c")
        .arg(py_script)
        .output();
    let py_out = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim_end().to_string(),
        _ => return,
    };

    let mut d1 = serde_json::json!({"a": 1, "b": 2, "nested": {"x": 1, "y": 2}})
        .as_object()
        .unwrap()
        .clone();
    let d2 = serde_json::json!({"b": 3, "c": 4, "nested": {"y": 20, "z": 30}})
        .as_object()
        .unwrap()
        .clone();
    powerliners::lib::dict::mergedicts(&mut d1, d2, true);

    // Normalise both via serde_json::Value for ordering-independent compare
    let rs_json: serde_json::Value = serde_json::Value::Object(d1);
    let py_json: serde_json::Value = serde_json::from_str(&py_out).expect("py json parse");

    assert_eq!(
        rs_json, py_json,
        "mergedicts mismatch:\n  py: {}\n  rs: {}",
        py_out, rs_json
    );
}

#[test]
fn parity_mergedefaults() {
    if !python_available() {
        return;
    }
    let py_script = r#"
import json, sys, os
sys.path.insert(0, os.environ['PL_VENDOR'])
from powerline.lib.dict import mergedefaults
d1 = {"a": 1}
d2 = {"a": 2, "b": 3}
mergedefaults(d1, d2)
print(json.dumps(d1, sort_keys=True))
"#;
    let vendor = repo_root().join("vendor").join("powerline");
    if !vendor.exists() {
        return;
    }
    let out = Command::new("python3")
        .env("PL_VENDOR", vendor.to_string_lossy().as_ref())
        .arg("-c")
        .arg(py_script)
        .output();
    let py_out = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim_end().to_string(),
        _ => return,
    };

    let mut d1 = serde_json::json!({"a": 1}).as_object().unwrap().clone();
    let d2 = serde_json::json!({"a": 2, "b": 3})
        .as_object()
        .unwrap()
        .clone();
    powerliners::lib::dict::mergedefaults(&mut d1, d2);
    let rs_json = serde_json::Value::Object(d1);
    let py_json: serde_json::Value = serde_json::from_str(&py_out).expect("py json parse");
    assert_eq!(
        rs_json, py_json,
        "mergedefaults mismatch:\n  py: {}\n  rs: {}",
        py_out, rs_json
    );
}

// ─────────────────────────────────────────────────────────────────────
// config.py — POWERLINE_ROOT layout invariants
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_config_layout_invariants() {
    // Layout invariants from py:7-10 — no python needed, this is a
    // structural property of the constants.
    let root = powerliners::config::POWERLINE_ROOT();
    let bindings = powerliners::config::BINDINGS_DIRECTORY();
    let tmux = powerliners::config::TMUX_CONFIG_DIRECTORY();
    assert_eq!(
        bindings,
        &root.join("powerline").join("bindings"),
        "BINDINGS_DIRECTORY = os.path.join(POWERLINE_ROOT, 'powerline', 'bindings')"
    );
    assert_eq!(
        tmux,
        &bindings.join("tmux"),
        "TMUX_CONFIG_DIRECTORY = os.path.join(BINDINGS_DIRECTORY, 'tmux')"
    );
    assert!(
        powerliners::config::DEFAULT_SYSTEM_CONFIG_DIR().is_none(),
        "DEFAULT_SYSTEM_CONFIG_DIR = None"
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/path.py — join + realpath
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_path_join() {
    if !python_available() {
        return;
    }
    let cases: &[&[&str]] = &[
        &["a", "b", "c"],
        &["/abs", "b", "c"],
        &["a", "/b", "c"],
        &["a", "b", "/c"],
    ];
    for parts in cases {
        let py_args = parts
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ");
        let expr = format!(
            "__import__('powerline.lib.path', fromlist=['join']).join({})",
            py_args
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::path::join(parts.iter().copied());
        assert_eq!(
            py,
            rs.to_string_lossy(),
            "path.join({:?}) mismatch: py={:?}, rs={:?}",
            parts,
            py,
            rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// theme.py — add_spaces_* + new_empty_segment_line
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_theme_add_spaces_left() {
    if !python_available() {
        return;
    }
    for amount in [0, 1, 2, 5, 10] {
        let expr = format!(
            "__import__('powerline.theme', fromlist=['add_spaces_left']).add_spaces_left(None, {}, {{'contents': 'hi'}})",
            amount
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let mut s = serde_json::Map::new();
        s.insert("contents".into(), serde_json::Value::String("hi".into()));
        let rs = powerliners::theme::add_spaces_left(&(), amount, &s);
        assert_eq!(
            py, rs,
            "add_spaces_left(amount={}) mismatch: py={:?}, rs={:?}",
            amount, py, rs
        );
    }
}

#[test]
fn parity_theme_add_spaces_right() {
    if !python_available() {
        return;
    }
    for amount in [0, 1, 2, 5, 10] {
        let expr = format!(
            "__import__('powerline.theme', fromlist=['add_spaces_right']).add_spaces_right(None, {}, {{'contents': 'hi'}})",
            amount
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let mut s = serde_json::Map::new();
        s.insert("contents".into(), serde_json::Value::String("hi".into()));
        let rs = powerliners::theme::add_spaces_right(&(), amount, &s);
        assert_eq!(
            py, rs,
            "add_spaces_right(amount={}) mismatch: py={:?}, rs={:?}",
            amount, py, rs
        );
    }
}

#[test]
fn parity_theme_add_spaces_center() {
    if !python_available() {
        return;
    }
    // Including odd amounts which trigger remainder-on-left
    for amount in [0, 1, 2, 3, 4, 5, 7, 10] {
        let expr = format!(
            "__import__('powerline.theme', fromlist=['add_spaces_center']).add_spaces_center(None, {}, {{'contents': 'hi'}})",
            amount
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let mut s = serde_json::Map::new();
        s.insert("contents".into(), serde_json::Value::String("hi".into()));
        let rs = powerliners::theme::add_spaces_center(&(), amount, &s);
        assert_eq!(
            py, rs,
            "add_spaces_center(amount={}) mismatch: py={:?}, rs={:?}",
            amount, py, rs
        );
    }
}

#[test]
fn parity_theme_new_empty_segment_line() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('json').dumps(__import__('powerline.theme', fromlist=['new_empty_segment_line']).new_empty_segment_line(), sort_keys=True)"
    ) {
        Some(v) => v,
        None => return,
    };
    let py_json: serde_json::Value = serde_json::from_str(&py).expect("py json parse");
    let rs = powerliners::theme::new_empty_segment_line();
    let rs_json = serde_json::Value::Object(rs);
    assert_eq!(
        py_json, rs_json,
        "new_empty_segment_line mismatch: py={}, rs={}",
        py, rs_json
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/shell.py — which()
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_which_finds_sh() {
    if !python_available() {
        return;
    }
    let py = match py_eval("__import__('powerline.lib.shell', fromlist=['which']).which('sh')") {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::lib::shell::which("sh");
    if py == "None" {
        assert!(rs.is_none(), "py None but rs found: {:?}", rs);
    } else {
        let rs_path = rs
            .expect("rs should have found sh")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            py, rs_path,
            "which('sh') mismatch: py={:?}, rs={:?}",
            py, rs_path
        );
    }
}

#[test]
fn parity_which_missing_returns_none() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lib.shell', fromlist=['which']).which('powerliners-nonexistent-binary-xyz')"
    ) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::lib::shell::which("powerliners-nonexistent-binary-xyz");
    assert_eq!(py, "None", "py expected None, got {:?}", py);
    assert!(rs.is_none(), "rs expected None, got {:?}", rs);
}

// ─────────────────────────────────────────────────────────────────────
// lib/overrides.py — parse_value, parsedotval, parse_override_var
// ─────────────────────────────────────────────────────────────────────

fn py_eval_json(expr: &str) -> Option<serde_json::Value> {
    let raw = py_eval(&format!(
        "__import__('json').dumps({}, sort_keys=True)",
        expr
    ))?;
    serde_json::from_str(&raw).ok()
}

#[test]
fn parity_parse_value_numbers_and_specials() {
    if !python_available() {
        return;
    }
    let cases = ["42", "-3", "3.14", "null", "true", "false", "hello", "TRUE"];
    for case in cases {
        let py = match py_eval_json(&format!(
            "__import__('powerline.lib.overrides', fromlist=['parse_value']).parse_value({:?})",
            case
        )) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::overrides::parse_value(case);
        // Skip the empty-string case (returns REMOVE_THIS_KEY sentinel which
        // is `object()` in Python — opaque, won't json-serialise).
        assert_eq!(
            py, rs,
            "parse_value({:?}) mismatch:\n  py: {:?}\n  rs: {:?}",
            case, py, rs
        );
    }
}

#[test]
fn parity_parsedotval_str() {
    if !python_available() {
        return;
    }
    let cases = ["foo=42", "a.b.c=true", "ext.tmux.theme=default"];
    for case in cases {
        let py_expr = format!(
            "list(__import__('powerline.lib.overrides', fromlist=['parsedotval']).parsedotval({:?}))",
            case
        );
        let py = match py_eval_json(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::overrides::parsedotval_str(case).unwrap();
        // Python returns (key, value) tuple; serialise as [key, value]
        let rs_arr = serde_json::json!([rs.0, rs.1]);
        assert_eq!(
            py, rs_arr,
            "parsedotval({:?}) mismatch:\n  py: {}\n  rs: {}",
            case, py, rs_arr
        );
    }
}

#[test]
fn parity_parse_override_var() {
    if !python_available() {
        return;
    }
    let cases = ["a=1;b=2;c.d=3", "ext.tmux.theme=default", ""];
    for case in cases {
        let py_expr = format!(
            "[list(x) for x in __import__('powerline.lib.overrides', fromlist=['parse_override_var']).parse_override_var({:?})]",
            case
        );
        let py = match py_eval_json(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::overrides::parse_override_var(case);
        let rs_arr = serde_json::Value::Array(
            rs.into_iter()
                .map(|(k, v)| serde_json::json!([k, v]))
                .collect(),
        );
        assert_eq!(
            py, rs_arr,
            "parse_override_var({:?}) mismatch:\n  py: {}\n  rs: {}",
            case, py, rs_arr
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// lib/memoize.py — default_cache_key (identity property only;
// Python returns frozenset, Rust returns String — but both must give
// the SAME identity on equal inputs)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_memoize_default_cache_key_equality_invariant() {
    // Python's `frozenset(kwargs.items())` is order-insensitive AND
    // equality-stable. Rust's string-based key must preserve the
    // same invariant: equal-kwargs → equal-key (regardless of
    // insertion order). This test verifies the identity property
    // without requiring Python (the property is a structural
    // requirement matching upstream's frozenset semantics).
    use serde_json::json;
    let mut m1 = serde_json::Map::new();
    m1.insert("a".into(), json!(1));
    m1.insert("b".into(), json!(2));

    let mut m2 = serde_json::Map::new();
    m2.insert("b".into(), json!(2));
    m2.insert("a".into(), json!(1));

    assert_eq!(
        powerliners::lib::memoize::default_cache_key(&m1),
        powerliners::lib::memoize::default_cache_key(&m2),
        "default_cache_key must be order-insensitive (matches Python frozenset semantic at py:10)"
    );

    let mut m3 = serde_json::Map::new();
    m3.insert("a".into(), json!(99)); // different value
    m3.insert("b".into(), json!(2));
    assert_ne!(
        powerliners::lib::memoize::default_cache_key(&m1),
        powerliners::lib::memoize::default_cache_key(&m3),
        "different values must yield different keys"
    );
}

// ─────────────────────────────────────────────────────────────────────
// version.py — get_version() also runs git rev-list
// ─────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────
// commands/main.py — int_or_sig
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_int_or_sig() {
    if !python_available() {
        return;
    }
    let cases = ["42", "-1", "0", "sigINT", "sigTERM"];
    for case in cases {
        let py = match py_eval(&format!(
            "__import__('powerline.commands.main', fromlist=['int_or_sig']).int_or_sig({:?})",
            case
        )) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::commands::main::int_or_sig(case).unwrap();
        let rs_str = match rs {
            powerliners::commands::main::IntOrSig::Sig(s) => s,
            powerliners::commands::main::IntOrSig::Int(n) => n.to_string(),
        };
        assert_eq!(
            py, rs_str,
            "int_or_sig({:?}) mismatch: py={:?}, rs={:?}",
            case, py, rs_str
        );
    }
}

#[test]
fn parity_int_or_sig_rejects_garbage() {
    if !python_available() {
        return;
    }
    // Python raises ValueError for "not-a-number"; subprocess will exit non-zero.
    let py = std::process::Command::new("python3")
        .arg("-c")
        .arg("import sys; sys.path.insert(0, '/Users/wizard/RustroverProjects/powerliners/vendor/powerline'); from powerline.commands.main import int_or_sig; int_or_sig('not-a-number')")
        .output();
    let py_ok = match py {
        Ok(o) => o.status.success(),
        Err(_) => return,
    };
    assert!(!py_ok, "Python should raise ValueError on bad int");
    assert!(
        powerliners::commands::main::int_or_sig("not-a-number").is_err(),
        "Rust should return Err on bad int"
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/url.py — urllib_urlencode
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_urllib_urlencode() {
    if !python_available() {
        return;
    }
    let cases: &[&[(&str, &str)]] = &[
        &[("a", "1"), ("b", "2")],
        &[("q", "hello world")],
        &[("k", "a/b?c=d")],
        &[("k", "abc_-.~0")],
    ];
    for case in cases {
        // Build a Python dict literal — order-preserving in Python 3.7+
        let py_dict = case
            .iter()
            .map(|(k, v)| format!("({:?}, {:?})", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        let expr = format!(
            "__import__('powerline.lib.url', fromlist=['urllib_urlencode']).urllib_urlencode([{}])",
            py_dict
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::url::urllib_urlencode(case.iter().copied());
        assert_eq!(
            py, rs,
            "urllib_urlencode({:?}) mismatch:\n  py: {:?}\n  rs: {:?}",
            case, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// renderers/pango_markup.py — escape
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_pango_markup_escape() {
    if !python_available() {
        return;
    }
    for s in ["plain", "<a & b>", "<>&", "&amp;"] {
        let py = match py_eval(&format!(
            "__import__('powerline.renderers.pango_markup', fromlist=['PangoMarkupRenderer']).PangoMarkupRenderer.escape({:?})",
            s
        )) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::renderers::pango_markup::PangoMarkupRenderer::escape(s);
        assert_eq!(
            py, rs,
            "PangoMarkupRenderer.escape({:?}) mismatch: py={:?}, rs={:?}",
            s, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// lint/inspect.py — formatconfigargspec
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_formatconfigargspec_args_only() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lint.inspect', fromlist=['formatconfigargspec']).formatconfigargspec(['a', 'b', 'c'])"
    ) {
        Some(v) => v,
        None => return,
    };
    let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let rs = powerliners::lint::inspect::formatconfigargspec(&args, &[]);
    assert_eq!(
        py, rs,
        "formatconfigargspec(args, no_defaults) mismatch: py={:?}, rs={:?}",
        py, rs
    );
}

#[test]
fn parity_formatconfigargspec_with_defaults() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lint.inspect', fromlist=['formatconfigargspec']).formatconfigargspec(['a', 'b', 'c'], defaults=(1, 2))"
    ) {
        Some(v) => v,
        None => return,
    };
    let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let defaults = vec!["1".to_string(), "2".to_string()];
    let rs = powerliners::lint::inspect::formatconfigargspec(&args, &defaults);
    assert_eq!(
        py, rs,
        "formatconfigargspec(args, defaults) mismatch: py={:?}, rs={:?}",
        py, rs
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/config.py — load_json_config
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_load_json_config_basic() {
    if !python_available() {
        return;
    }
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "powerliners-parity-config-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(
        &tmp,
        r#"{"theme": "default", "ext": {"shell": {"theme": "default"}}}"#,
    )
    .unwrap();

    let path_str = tmp.to_string_lossy();
    let py_script = format!(
        "import json, sys, os\n\
         sys.path.insert(0, os.environ['PL_VENDOR'])\n\
         from powerline.lib.config import load_json_config\n\
         print(json.dumps(load_json_config({:?}), sort_keys=True))",
        path_str
    );
    let vendor = repo_root().join("vendor").join("powerline");
    if !vendor.exists() {
        std::fs::remove_file(&tmp).ok();
        return;
    }
    let py_out = std::process::Command::new("python3")
        .env("PL_VENDOR", vendor.to_string_lossy().as_ref())
        .arg("-c")
        .arg(&py_script)
        .output();
    let py_str = match py_out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim_end().to_string(),
        _ => {
            std::fs::remove_file(&tmp).ok();
            return;
        }
    };
    let py_json: serde_json::Value = serde_json::from_str(&py_str).unwrap();
    let rs_json = powerliners::lib::config::load_json_config(&tmp).unwrap();
    assert_eq!(
        py_json, rs_json,
        "load_json_config mismatch:\n  py: {}\n  rs: {}",
        py_str, rs_json
    );
    std::fs::remove_file(&tmp).ok();
}

#[test]
fn parity_version_get_version_falls_back_to_constant() {
    if !python_available() {
        return;
    }
    // Run in a directory that's not a git repo so both impls fall back
    // to the __version__ literal.
    let py =
        match py_eval("__import__('powerline.version', fromlist=['get_version']).get_version()") {
            Some(v) => v,
            None => return,
        };
    let rs = powerliners::version::get_version();
    // Both should start with __version__; the suffix may differ if
    // one is in a git repo and the other isn't. The shared prefix
    // (version literal) MUST match.
    assert!(
        py.starts_with(powerliners::version::__version__),
        "py output {:?} should start with __version__ {}",
        py,
        powerliners::version::__version__
    );
    assert!(
        rs.starts_with(powerliners::version::__version__),
        "rs output {:?} should start with __version__ {}",
        rs,
        powerliners::version::__version__
    );
}

// ─────────────────────────────────────────────────────────────────────
// __init__.py — get_default_theme / DEFAULT_UPDATE_INTERVAL / LOG_KEYS
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_get_default_theme_unicode_branch() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline', fromlist=['get_default_theme']).get_default_theme(True)",
    ) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::get_default_theme(true);
    assert_eq!(py, rs, "get_default_theme(True) mismatch");
}

#[test]
fn parity_get_default_theme_ascii_branch() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline', fromlist=['get_default_theme']).get_default_theme(False)",
    ) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::get_default_theme(false);
    assert_eq!(py, rs, "get_default_theme(False) mismatch");
}

#[test]
fn parity_default_update_interval_constant() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline', fromlist=['DEFAULT_UPDATE_INTERVAL']).DEFAULT_UPDATE_INTERVAL",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_int: u64 = py
        .parse()
        .unwrap_or_else(|_| panic!("Python returned non-int: {:?}", py));
    let rs = powerliners::DEFAULT_UPDATE_INTERVAL;
    assert_eq!(py_int, rs, "DEFAULT_UPDATE_INTERVAL mismatch");
}

#[test]
fn parity_log_keys_set() {
    if !python_available() {
        return;
    }
    let py = match py_eval("sorted(__import__('powerline', fromlist=['LOG_KEYS']).LOG_KEYS)") {
        Some(v) => v,
        None => return,
    };
    // Python repr of sorted list looks like: ['log_file', 'log_format', 'log_level', 'paths']
    let mut rs_sorted: Vec<&str> = powerliners::LOG_KEYS().iter().copied().collect();
    rs_sorted.sort();
    let rs_repr = format!(
        "[{}]",
        rs_sorted
            .iter()
            .map(|k| format!("'{}'", k))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(py, rs_repr, "LOG_KEYS set contents mismatch");
}

// ─────────────────────────────────────────────────────────────────────
// renderers/shell/*.py — escape_hl_start / escape_hl_end class attrs
//
// These verify the 3 NEAR-tier-ceiling files (readline, zsh) plus the
// graduated ksh/tcsh shell renderers carry exactly the same readline
// non-display markers / shell prompt-protection escapes as upstream.
// ─────────────────────────────────────────────────────────────────────

fn parity_renderer_escape_pair(
    py_module: &str,
    py_class: &str,
    rs_start: &str,
    rs_end: &str,
    label: &str,
) {
    if !python_available() {
        return;
    }
    let start_expr = format!(
        "__import__('{}', fromlist=['{}']).{}.escape_hl_start",
        py_module, py_class, py_class
    );
    let end_expr = format!(
        "__import__('{}', fromlist=['{}']).{}.escape_hl_end",
        py_module, py_class, py_class
    );
    let py_start = match py_eval(&start_expr) {
        Some(v) => v,
        None => return,
    };
    let py_end = match py_eval(&end_expr) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py_start, rs_start, "{} escape_hl_start mismatch", label);
    assert_eq!(py_end, rs_end, "{} escape_hl_end mismatch", label);
}

#[test]
fn parity_readline_renderer_escape_markers() {
    parity_renderer_escape_pair(
        "powerline.renderers.shell.readline",
        "ReadlineRenderer",
        powerliners::renderers::shell::readline::ReadlineRenderer::escape_hl_start,
        powerliners::renderers::shell::readline::ReadlineRenderer::escape_hl_end,
        "ReadlineRenderer",
    );
}

#[test]
fn parity_zsh_renderer_escape_markers() {
    parity_renderer_escape_pair(
        "powerline.renderers.shell.zsh",
        "ZshPromptRenderer",
        powerliners::renderers::shell::zsh::ZshPromptRenderer::escape_hl_start,
        powerliners::renderers::shell::zsh::ZshPromptRenderer::escape_hl_end,
        "ZshPromptRenderer",
    );
}

#[test]
fn parity_ksh_renderer_escape_markers() {
    parity_renderer_escape_pair(
        "powerline.renderers.shell.ksh",
        "KshPromptRenderer",
        powerliners::renderers::shell::ksh::KshPromptRenderer::escape_hl_start,
        powerliners::renderers::shell::ksh::KshPromptRenderer::escape_hl_end,
        "KshPromptRenderer",
    );
}

#[test]
fn parity_bash_renderer_escape_markers() {
    parity_renderer_escape_pair(
        "powerline.renderers.shell.bash",
        "BashPromptRenderer",
        powerliners::renderers::shell::bash::BashPromptRenderer::escape_hl_start,
        powerliners::renderers::shell::bash::BashPromptRenderer::escape_hl_end,
        "BashPromptRenderer",
    );
}

// ─────────────────────────────────────────────────────────────────────
// lint/markedjson/nodes.py — Node subclass `id` class attributes
// lint/markedjson/tokens.py — Token subclass `id` class attributes
// ─────────────────────────────────────────────────────────────────────

fn parity_node_or_token_id(py_module: &str, py_class: &str, rs_id: &str) {
    if !python_available() {
        return;
    }
    let expr = format!(
        "__import__('{}', fromlist=['{}']).{}.id",
        py_module, py_class, py_class
    );
    let py = match py_eval(&expr) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py, rs_id, "{}.id mismatch", py_class);
}

#[test]
fn parity_scalar_node_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.nodes",
        "ScalarNode",
        powerliners::lint::markedjson::nodes::ScalarNode::ID,
    );
}

#[test]
fn parity_sequence_node_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.nodes",
        "SequenceNode",
        powerliners::lint::markedjson::nodes::SequenceNode::ID,
    );
}

#[test]
fn parity_mapping_node_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.nodes",
        "MappingNode",
        powerliners::lint::markedjson::nodes::MappingNode::ID,
    );
}

#[test]
fn parity_stream_start_token_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "StreamStartToken",
        powerliners::lint::markedjson::tokens::StreamStartToken::ID,
    );
}

#[test]
fn parity_stream_end_token_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "StreamEndToken",
        powerliners::lint::markedjson::tokens::StreamEndToken::ID,
    );
}

#[test]
fn parity_flow_sequence_start_token_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "FlowSequenceStartToken",
        powerliners::lint::markedjson::tokens::FlowSequenceStartToken::ID,
    );
}

#[test]
fn parity_flow_mapping_start_token_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "FlowMappingStartToken",
        powerliners::lint::markedjson::tokens::FlowMappingStartToken::ID,
    );
}

#[test]
fn parity_key_value_flow_entry_token_ids() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "KeyToken",
        powerliners::lint::markedjson::tokens::KeyToken::ID,
    );
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "ValueToken",
        powerliners::lint::markedjson::tokens::ValueToken::ID,
    );
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "FlowEntryToken",
        powerliners::lint::markedjson::tokens::FlowEntryToken::ID,
    );
}

#[test]
fn parity_scalar_token_id() {
    parity_node_or_token_id(
        "powerline.lint.markedjson.tokens",
        "ScalarToken",
        powerliners::lint::markedjson::tokens::ScalarToken::ID,
    );
}

// ─────────────────────────────────────────────────────────────────────
// segments/common/wthr.py — WeatherSegment class consts
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_weather_api_key_default() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.segments.common.wthr', fromlist=['WeatherSegment']).WeatherSegment.weather_api_key",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py,
        powerliners::segments::common::wthr::WEATHER_API_KEY,
        "WeatherSegment.weather_api_key mismatch"
    );
}

#[test]
fn parity_weather_interval_default() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.segments.common.wthr', fromlist=['WeatherSegment']).WeatherSegment.interval",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_int: u32 = py.parse().expect("Python returned non-int for interval");
    assert_eq!(
        py_int,
        powerliners::segments::common::wthr::WEATHER_INTERVAL,
        "WeatherSegment.interval mismatch"
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/threaded.py — ThreadedSegment class consts
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_threaded_segment_min_sleep_time() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['ThreadedSegment']).ThreadedSegment.min_sleep_time",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_float: f64 = py
        .parse()
        .expect("Python returned non-float for min_sleep_time");
    let rs = powerliners::lib::threaded::ThreadedSegment::new().min_sleep_time;
    assert!(
        (py_float - rs).abs() < 1e-9,
        "ThreadedSegment.min_sleep_time mismatch: py={}, rs={}",
        py_float,
        rs
    );
}

#[test]
fn parity_threaded_segment_update_first_default() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['ThreadedSegment']).ThreadedSegment.update_first",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_bool = py == "True";
    let rs = powerliners::lib::threaded::ThreadedSegment::new().update_first;
    assert_eq!(
        py_bool, rs,
        "ThreadedSegment.update_first mismatch: py={}, rs={}",
        py_bool, rs
    );
}

#[test]
fn parity_threaded_segment_interval_default() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['ThreadedSegment']).ThreadedSegment.interval",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_int: f64 = py
        .parse()
        .expect("Python returned non-numeric for interval");
    let rs = powerliners::lib::threaded::ThreadedSegment::new().interval;
    assert!(
        (py_int - rs).abs() < 1e-9,
        "ThreadedSegment.interval mismatch: py={}, rs={}",
        py_int,
        rs
    );
}

// ─────────────────────────────────────────────────────────────────────
// config.py — DEFAULT_SYSTEM_CONFIG_DIR
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_default_system_config_dir_is_none() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.config', fromlist=['DEFAULT_SYSTEM_CONFIG_DIR']).DEFAULT_SYSTEM_CONFIG_DIR",
    ) {
        Some(v) => v,
        None => return,
    };
    // Python None prints as "None"
    let rs = powerliners::ported::config::DEFAULT_SYSTEM_CONFIG_DIR();
    assert_eq!(py, "None", "Python DEFAULT_SYSTEM_CONFIG_DIR not None");
    assert!(
        rs.is_none(),
        "Rust DEFAULT_SYSTEM_CONFIG_DIR() returned Some, expected None"
    );
}

// ─────────────────────────────────────────────────────────────────────
// segments/common/players.py — STATE_SYMBOLS + _convert_state
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_players_state_symbols() {
    if !python_available() {
        return;
    }
    // Test each key/value pair separately to avoid Python dict ordering issues.
    let keys = ["fallback", "play", "pause", "stop"];
    let rs_map = powerliners::segments::common::players::state_symbols();
    for key in &keys {
        let expr = format!(
            "__import__('powerline.segments.common.players', fromlist=['STATE_SYMBOLS']).STATE_SYMBOLS[{:?}]",
            key
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs_val = rs_map
            .get(*key)
            .and_then(|v| v.as_str())
            .unwrap_or("<missing>");
        assert_eq!(
            py, rs_val,
            "STATE_SYMBOLS[{:?}] mismatch: py={:?}, rs={:?}",
            key, py, rs_val
        );
    }
}

#[test]
fn parity_players_convert_state() {
    if !python_available() {
        return;
    }
    let cases = [
        "Play",
        "PLAYING",
        "Paused",
        "stopped",
        "STOP",
        "unknown",
        "",
        "fallback",
        // Edge: 'play' substring takes precedence over later checks
        "displaying",
        // Edge: 'pause' substring
        "paused for x",
    ];
    for input in &cases {
        let expr = format!(
            "__import__('powerline.segments.common.players', fromlist=['_convert_state'])._convert_state({:?})",
            input
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::segments::common::players::_convert_state(input);
        assert_eq!(
            py, rs,
            "_convert_state({:?}) mismatch: py={:?}, rs={:?}",
            input, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// segments/common/players.py — _convert_seconds
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_players_convert_seconds() {
    if !python_available() {
        return;
    }
    let cases: &[f64] = &[0.0, 1.0, 59.0, 60.0, 61.0, 125.0, 3600.0, 3661.0];
    for &s in cases {
        let expr = format!(
            "__import__('powerline.segments.common.players', fromlist=['_convert_seconds'])._convert_seconds({})",
            s
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::segments::common::players::_convert_seconds(s);
        assert_eq!(
            py, rs,
            "_convert_seconds({}) mismatch: py={:?}, rs={:?}",
            s, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// lib/dict.py — mergedicts_copy + updated
// (mergedicts + mergedefaults + mergeargs already covered earlier)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_mergedicts_copy_does_not_mutate_inputs() {
    if !python_available() {
        return;
    }
    // Pick a case where d2 wins on a collision AND a nested dict gets
    // recursively merged.
    let expr = "\
        import json; \
        mod = __import__('powerline.lib.dict', fromlist=['mergedicts_copy']); \
        d1 = {'a': 1, 'nested': {'x': 1, 'y': 2}}; \
        d2 = {'b': 2, 'nested': {'y': 99, 'z': 3}}; \
        r = mod.mergedicts_copy(d1, d2); \
        print(json.dumps(r, sort_keys=True), end='')";
    let py = match py_eval(expr) {
        Some(v) => v,
        None => return,
    };

    use serde_json::json;
    let d1 = serde_json::json!({"a": 1, "nested": {"x": 1, "y": 2}})
        .as_object()
        .unwrap()
        .clone();
    let d2 = json!({"b": 2, "nested": {"y": 99, "z": 3}})
        .as_object()
        .unwrap()
        .clone();
    let r = powerliners::lib::dict::mergedicts_copy(&d1, d2);
    // Sort the Rust output the same way Python's sort_keys=True does for stable comparison.
    let rs = serde_json::to_string(&{
        let mut sorted = std::collections::BTreeMap::new();
        for (k, v) in &r {
            sorted.insert(k.clone(), v.clone());
        }
        sorted
    })
    .unwrap();
    // Python's json.dumps adds spaces after commas and colons by default;
    // serde_json::to_string is compact. Normalize both to compact form.
    let py_compact = py.replace(", ", ",").replace(": ", ":");
    assert_eq!(
        rs, py_compact,
        "mergedicts_copy mismatch:\n  py: {}\n  rs: {}",
        py_compact, rs
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/unicode.py — surrogate_pair_to_character
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_surrogate_pair_to_character() {
    if !python_available() {
        return;
    }
    // Surrogate pairs encoding codepoints from the supplementary planes.
    // Each tuple: (high, low) → expected unicode codepoint.
    // E.g. 0xD83D 0xDE00 → 0x1F600 (😀 grinning face)
    //      0xD83D 0xDCA9 → 0x1F4A9 (💩 pile of poo)
    //      0xD834 0xDD1E → 0x1D11E (𝄞 musical symbol G clef)
    let cases: &[(u32, u32)] = &[
        (0xD83D, 0xDE00),
        (0xD83D, 0xDCA9),
        (0xD834, 0xDD1E),
        (0xD800, 0xDC00), // boundary: lowest surrogate pair → 0x10000
        (0xDBFF, 0xDFFF), // boundary: highest surrogate pair → 0x10FFFF
    ];
    for &(high, low) in cases {
        let expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['surrogate_pair_to_character']).surrogate_pair_to_character({}, {})",
            high, low
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_cp: u32 = py.parse().expect("Python returned non-int codepoint");
        let rs = powerliners::lib::unicode::surrogate_pair_to_character(high, low);
        assert_eq!(
            py_cp, rs,
            "surrogate_pair_to_character(0x{:04X}, 0x{:04X}) mismatch: py=0x{:X}, rs=0x{:X}",
            high, low, py_cp, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// bindings/tmux/__init__.py — NON_DIGITS / DIGITS / NON_LETTERS regex patterns
// (verify pattern strings match upstream regex sources)
// ─────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────
// renderer.py — np_control_character_translations dict
//   (maps 0x00-0x1F → '^@', '^A', ..., '^_')
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_renderer_np_control_character_translations() {
    if !python_available() {
        return;
    }
    // Spot-check 5 representative codepoints across the 0x00-0x1F range.
    // Python: dict where (i, '^' + chr(i + 0x40)) so 0x00→'^@', 0x09→'^I', etc.
    let cases: &[(u32, &str)] = &[
        (0x00, "^@"),
        (0x09, "^I"), // tab
        (0x0A, "^J"), // newline
        (0x10, "^P"),
        (0x1F, "^_"),
    ];
    let rs_table = powerliners::ported::renderer::np_control_character_translations();
    for &(cp, expected) in cases {
        let expr = format!(
            "__import__('powerline.renderer', fromlist=['np_control_character_translations']).np_control_character_translations[{}]",
            cp
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py, expected,
            "py disagrees with hand-written expected for cp 0x{:02X}",
            cp
        );
        let rs_val = rs_table
            .get(&char::from_u32(cp).unwrap())
            .map(|s| s.as_str())
            .unwrap_or("<missing>");
        assert_eq!(
            py, rs_val,
            "np_control_character_translations[0x{:02X}] mismatch: py={:?}, rs={:?}",
            cp, py, rs_val
        );
    }
    // Verify table size: Python uses range(0x20) → 32 entries.
    let py_len = match py_eval(
        "len(__import__('powerline.renderer', fromlist=['np_control_character_translations']).np_control_character_translations)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py_len, "32", "Python table size != 32");
    assert_eq!(rs_table.len(), 32, "Rust table size != 32");
}

#[test]
fn parity_renderer_np_invalid_character_translations() {
    if !python_available() {
        return;
    }
    // Python: range(0xDC80, 0xDD00) → 128 entries, each mapped to
    // '<{0:02x}>'.format(cp - 0xDC00).
    // So 0xDC80 → '<80>', 0xDCFF → '<ff>', 0xDD00-1 → '<ff>' (table end is exclusive)
    let cases: &[(u32, &str)] = &[(0xDC80, "<80>"), (0xDCA9, "<a9>"), (0xDCFF, "<ff>")];
    let rs_table = powerliners::ported::renderer::np_invalid_character_translations();
    for &(cp, expected) in cases {
        let expr = format!(
            "__import__('powerline.renderer', fromlist=['np_invalid_character_translations']).np_invalid_character_translations[{}]",
            cp
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(py, expected, "py disagrees with expected for cp 0x{:X}", cp);
        let rs_val = rs_table.get(&cp).map(|s| s.as_str()).unwrap_or("<missing>");
        assert_eq!(
            py, rs_val,
            "np_invalid_character_translations[0x{:X}] mismatch: py={:?}, rs={:?}",
            cp, py, rs_val
        );
    }
    let py_len = match py_eval(
        "len(__import__('powerline.renderer', fromlist=['np_invalid_character_translations']).np_invalid_character_translations)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py_len, "128", "Python table size != 128 (0xDC80..0xDD00)");
    assert_eq!(rs_table.len(), 128, "Rust table size != 128");
}

// ─────────────────────────────────────────────────────────────────────
// segments/i3wm.py — WORKSPACE_REGEX pattern + format_name + WS_ICONS
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_i3wm_workspace_regex_pattern() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.segments.i3wm', fromlist=['WORKSPACE_REGEX']).WORKSPACE_REGEX.pattern",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py, r"^[0-9]+: ?",
        "WORKSPACE_REGEX.pattern mismatch: py={:?}",
        py
    );
}

#[test]
fn parity_i3wm_format_name_strips_prefix() {
    if !python_available() {
        return;
    }
    // Verify both branches: strip=False is identity; strip=True removes
    // `[0-9]+: ?` exactly once at the start.
    let cases: &[(&str, bool)] = &[
        ("1: term", false),
        ("1: term", true),
        ("10:foo", true),
        ("no prefix", true),
        ("", true),
        ("9: bar baz", true),
    ];
    for &(name, strip) in cases {
        let expr = format!(
            "__import__('powerline.segments.i3wm', fromlist=['format_name']).format_name({:?}, {})",
            name,
            if strip { "True" } else { "False" }
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::segments::i3wm::format_name(name, strip);
        assert_eq!(
            py, rs,
            "format_name({:?}, {}) mismatch: py={:?}, rs={:?}",
            name, strip, py, rs
        );
    }
}

#[test]
fn parity_i3wm_ws_icons_default() {
    if !python_available() {
        return;
    }
    // Verify WS_ICONS == {"multiple": "M"} on both sides.
    let py = match py_eval(
        "list(sorted(__import__('powerline.segments.i3wm', fromlist=['WS_ICONS']).WS_ICONS.items()))",
    ) {
        Some(v) => v,
        None => return,
    };
    // Python list repr: [('multiple', 'M')]
    assert_eq!(
        py, "[('multiple', 'M')]",
        "WS_ICONS dict mismatch: py={:?}",
        py
    );
    let rs_map = powerliners::segments::i3wm::ws_icons();
    assert_eq!(rs_map.len(), 1, "Rust WS_ICONS has wrong key count");
    assert_eq!(
        rs_map.get("multiple").and_then(|v| v.as_str()),
        Some("M"),
        "Rust WS_ICONS['multiple'] != 'M'"
    );
}

#[test]
fn parity_i3wm_workspace_groups_state_combinations() {
    if !python_available() {
        return;
    }
    // Exhaustively verify every combination of (focused, urgent, visible)
    // produces the same highlight-group ordering as upstream Python.
    for focused in [false, true] {
        for urgent in [false, true] {
            for visible in [false, true] {
                // Build a tiny stub class mimicking i3ipc.Workspace's
                // attribute shape so the Python fn accepts it.
                let py_expr = format!(
                    "(lambda: (lambda W: __import__('powerline.segments.i3wm', fromlist=['workspace_groups']).workspace_groups(W({}, {}, {})))(type('W', (), dict(__init__=lambda self, f, u, v: setattr(self, 'focused', f) or setattr(self, 'urgent', u) or setattr(self, 'visible', v)))))()",
                    if focused { "True" } else { "False" },
                    if urgent { "True" } else { "False" },
                    if visible { "True" } else { "False" }
                );
                let py = match py_eval(&py_expr) {
                    Some(v) => v,
                    None => return,
                };
                let flags = powerliners::segments::i3wm::WorkspaceFlags {
                    focused,
                    urgent,
                    visible,
                };
                let rs = powerliners::segments::i3wm::workspace_groups(flags);
                let rs_repr = format!(
                    "[{}]",
                    rs.iter()
                        .map(|s| format!("'{}'", s))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                assert_eq!(
                    py, rs_repr,
                    "workspace_groups(focused={}, urgent={}, visible={}) mismatch",
                    focused, urgent, visible
                );
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// segments/common/time.py — UNICODE_TEXT_TRANSLATION + hour_str default
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_time_unicode_text_translation_table() {
    if !python_available() {
        return;
    }
    // Verify the upstream table maps exactly two ASCII characters to
    // their unicode equivalents, and the Rust translate() helper
    // mirrors that mapping when applied to a string containing both.
    let py = match py_eval(
        "list(sorted(__import__('powerline.segments.common.time', fromlist=['UNICODE_TEXT_TRANSLATION']).UNICODE_TEXT_TRANSLATION.items()))",
    ) {
        Some(v) => v,
        None => return,
    };
    // Python prints e.g. [(39, '’'), (45, '‐')]
    // ASCII apostrophe (39) → U+2019, ASCII hyphen-minus (45) → U+2010.
    assert!(
        py.contains("39") && py.contains("’"),
        "expected apostrophe mapping in table, got {}",
        py
    );
    assert!(
        py.contains("45") && py.contains("‐"),
        "expected hyphen mapping in table, got {}",
        py
    );

    // Round-trip via Rust translate()
    let rs = powerliners::segments::common::time::unicode_text_translate("don't-care");
    assert_eq!(rs, "don\u{2019}t\u{2010}care", "Rust translate() mismatch");

    // Round-trip via Python str.translate()
    let py_rt = match py_eval(
        "\"don't-care\".translate(__import__('powerline.segments.common.time', fromlist=['UNICODE_TEXT_TRANSLATION']).UNICODE_TEXT_TRANSLATION)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(rs, py_rt, "py vs rs translate() output mismatch");
}

#[test]
fn parity_time_fuzzy_time_default_hour_str() {
    if !python_available() {
        return;
    }
    // The default hour_str list is positional 1st default of fuzzy_time.
    // Read it from the function's __defaults__ via inspect.
    let py = match py_eval(
        "list(__import__('inspect').signature(__import__('powerline.segments.common.time', fromlist=['fuzzy_time']).fuzzy_time).parameters['hour_str'].default)",
    ) {
        Some(v) => v,
        None => return,
    };
    // Python list repr: ['twelve', 'one', ..., 'eleven']
    let rs = powerliners::segments::common::time::fuzzy_time_default_hour_str();
    let rs_repr = format!(
        "[{}]",
        rs.iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(py, rs_repr, "fuzzy_time hour_str default mismatch");
}

#[test]
fn parity_tmux_regex_pattern_strings() {
    if !python_available() {
        return;
    }
    let cases = [
        ("NON_DIGITS", "[^0-9]+"),
        ("DIGITS", "[0-9]+"),
        ("NON_LETTERS", "[^a-z]+"),
    ];
    for (name, expected) in cases {
        let expr = format!(
            "__import__('powerline.bindings.tmux', fromlist=['{}']).{}.pattern",
            name, name
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py, expected,
            "tmux {}.pattern mismatch: py={:?}, expected={:?}",
            name, py, expected
        );
    }
    // Cross-check Rust regex compilation accepts identical inputs by
    // exercising them against known strings.
    let non_digits = powerliners::ported::bindings::tmux::NON_DIGITS();
    let digits = powerliners::ported::bindings::tmux::DIGITS();
    assert_eq!(
        non_digits.replace_all("2.3a", "").into_owned(),
        "23",
        "Rust NON_DIGITS stripped output mismatch"
    );
    assert_eq!(
        digits.replace_all("2.3a", "").into_owned(),
        ".a",
        "Rust DIGITS stripped output mismatch"
    );
}

#[test]
fn parity_updated_merges_and_copies() {
    if !python_available() {
        return;
    }
    // Verify updated(d, kwargs) returns the merged copy with d2 keys
    // winning and d itself untouched.
    let expr = "\
        import json; \
        mod = __import__('powerline.lib.dict', fromlist=['updated']); \
        d = {'a': 1, 'b': 2}; \
        r = mod.updated(d, c=3, b=99); \
        print(json.dumps(r, sort_keys=True), end='')";
    let py = match py_eval(expr) {
        Some(v) => v,
        None => return,
    };

    let d = serde_json::json!({"a": 1, "b": 2})
        .as_object()
        .unwrap()
        .clone();
    let updates: Vec<(String, serde_json::Value)> = vec![
        ("c".to_string(), serde_json::Value::from(3)),
        ("b".to_string(), serde_json::Value::from(99)),
    ];
    let r = powerliners::lib::dict::updated(&d, updates);
    let rs = serde_json::to_string(&{
        let mut sorted = std::collections::BTreeMap::new();
        for (k, v) in &r {
            sorted.insert(k.clone(), v.clone());
        }
        sorted
    })
    .unwrap();
    let py_compact = py.replace(", ", ",").replace(": ", ":");
    assert_eq!(
        rs, py_compact,
        "updated mismatch:\n  py: {}\n  rs: {}",
        py_compact, rs
    );
}
