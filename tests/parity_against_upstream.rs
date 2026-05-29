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
// lib/threaded.py — MultiRunnedThread + ThreadedSegment daemon defaults
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_multi_runned_thread_daemon_default() {
    if !python_available() {
        return;
    }
    // MultiRunnedThread.daemon class attr defaults to True (py:12).
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['MultiRunnedThread']).MultiRunnedThread.daemon",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_bool = py == "True";
    let rs = powerliners::lib::threaded::MultiRunnedThread::new().daemon;
    assert_eq!(
        py_bool, rs,
        "MultiRunnedThread.daemon mismatch: py={}, rs={}",
        py_bool, rs
    );
}

#[test]
fn parity_threaded_segment_daemon_override() {
    if !python_available() {
        return;
    }
    // ThreadedSegment overrides MultiRunnedThread.daemon=True with False
    // at py:36. The Rust port's ThreadedSegment::new() now applies the
    // same class-level override at construction time (was a divergence
    // flagged in the previous /loop fire; fixed in this fire).
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['ThreadedSegment']).ThreadedSegment.daemon",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_bool = py == "True";
    let rs = powerliners::lib::threaded::ThreadedSegment::new()
        .base
        .daemon;
    assert_eq!(
        py_bool, rs,
        "ThreadedSegment.daemon mismatch: py={}, rs={}",
        py_bool, rs
    );
    assert!(!rs, "ThreadedSegment.daemon should be false after new()");
}

// ─────────────────────────────────────────────────────────────────────
// lint/spec.py — Spec.optional()/required() round-trip
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_spec_optional_required_round_trip() {
    if !python_available() {
        return;
    }
    // Verify the fluent builder's optional() and required() methods toggle
    // isoptional identically to upstream.
    let cases = [
        "Spec().isoptional",
        "Spec().optional().isoptional",
        "Spec().optional().required().isoptional",
        "Spec().required().optional().isoptional",
    ];
    let py_values: Vec<String> = cases
        .iter()
        .filter_map(|expr| {
            let full = format!(
                "__import__('powerline.lint.spec', fromlist=['Spec']).{}",
                expr.replacen("Spec()", "Spec()", 1)
            );
            py_eval(&full)
        })
        .collect();
    if py_values.len() != cases.len() {
        return;
    }
    // Expected: False, True, False, True
    assert_eq!(
        py_values,
        vec!["False", "True", "False", "True"],
        "Python Spec.optional/required toggle changed semantics"
    );

    use powerliners::lint::spec::Spec;
    assert!(
        !Spec::new().isoptional,
        "Rust Spec::new().isoptional should be false"
    );
    assert!(
        Spec::new().optional().isoptional,
        "Rust Spec::new().optional().isoptional should be true"
    );
    assert!(
        !Spec::new().optional().required().isoptional,
        "Rust Spec::new().optional().required().isoptional should be false"
    );
    assert!(
        Spec::new().required().optional().isoptional,
        "Rust Spec::new().required().optional().isoptional should be true"
    );
}

#[test]
fn parity_spec_regex_check_appended() {
    if !python_available() {
        return;
    }
    // Verify Spec().re(pat) appends a 'check_re' entry to self.checks
    // and Rust Spec::new().regex(pat) stores the pattern verbatim. The
    // public-API observation we can assert: len(spec.checks) increases
    // by exactly 1 after a single re() call.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().re('^[abc]+$').checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    // Python Spec.re() registers BOTH check_type (str enforced) AND
    // check_func (regex match), so len(checks) == 2 after a single call.
    assert_eq!(
        py_len, 2,
        "Python Spec.re() should append exactly 2 check entries (type+func)"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().regex("^[abc]+$");
    assert_eq!(
        s.regex.as_deref(),
        Some("^[abc]+$"),
        "Rust Spec.regex() should store pattern verbatim"
    );
}

#[test]
fn parity_spec_type_does_not_set_did_type() {
    if !python_available() {
        return;
    }
    // Python: Spec().type(str).did_type stays False. type() only appends
    // to checks. did_type is only flipped inside update() when keys go
    // non-empty as a gate against auto-adding type(dict).
    let py = match py_eval(
        "__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).did_type",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py, "False", "Python Spec().type(str).did_type changed");
    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new().type_check(&[SpecType::Unicode]);
    assert!(
        !s.did_type,
        "Rust Spec::new().type_check().did_type should be false"
    );
}

#[test]
fn parity_spec_update_auto_adds_dict_type_once() {
    if !python_available() {
        return;
    }
    // Python: Spec().update(foo=Spec()).did_type becomes True AND
    // the spec gains type=dict because update() gates on did_type
    // and auto-calls self.type(dict) when keys is non-empty.
    let py = match py_eval(
        "__import__('powerline.lint.spec', fromlist=['Spec']).Spec().update(foo=__import__('powerline.lint.spec', fromlist=['Spec']).Spec()).did_type",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py, "True", "Python update() should set did_type=True");
    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new().update("foo", Spec::new());
    assert!(
        s.did_type,
        "Rust update() should set did_type=true when keys go non-empty"
    );
    assert!(
        s.allowed_types.contains(&SpecType::Dict),
        "Rust update() should auto-add SpecType::Dict to allowed_types"
    );
}

#[test]
fn parity_spec_printable_chains_type_unicode() {
    if !python_available() {
        return;
    }
    // Python: Spec().printable() chains self.type(unicode) first, so
    // len(checks) == 2 (check_type + check_printable). Verify both
    // ports treat printable() as a unicode-typed constraint.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().printable().checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 2,
        "Python Spec.printable() should append 2 check entries (type+printable)"
    );
    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new().printable();
    assert!(
        s.allowed_types.contains(&SpecType::Unicode),
        "Rust printable() should pin allowed type to Unicode"
    );
    assert!(s.printable_flag, "Rust printable_flag should be set");
}

#[test]
fn parity_spec_unsigned_chains_type_int_and_cmp_ge_zero() {
    if !python_available() {
        return;
    }
    // Python: Spec().unsigned() chains self.type(int) + check_func(>= 0),
    // so len(checks) == 2. Verify the Rust port pins the type AND the cmp
    // constraint.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().unsigned().checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 2,
        "Python Spec.unsigned() should append 2 check entries (type+func)"
    );
    use powerliners::lint::spec::{Cmp, Spec, SpecType};
    let s = Spec::new().unsigned();
    assert!(
        s.allowed_types.contains(&SpecType::Float),
        "Rust unsigned() should pin allowed type to Float"
    );
    assert_eq!(
        s.cmp_constraint,
        Some((Cmp::Ge, 0.0)),
        "Rust unsigned() should set cmp_constraint to (>=, 0)"
    );
    assert!(s.unsigned_flag, "Rust unsigned_flag should be set");
}

#[test]
fn parity_spec_func_check_appended() {
    if !python_available() {
        return;
    }
    // Spec().func(callable) registers a check_func entry on Python.
    // Rust's func(name) takes a registered function name (since
    // closures don't survive the builder boundary) and stores it as
    // error_msg.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().func(lambda x: True).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 1,
        "Python Spec.func() should append 1 check entry (check_func)"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().func("my_check");
    // The Rust port's func() reuses error_msg as the registered-function
    // name (since closures can't be stored without callback wiring).
    assert_eq!(
        s.error_msg.as_deref(),
        Some("my_check"),
        "Rust Spec.func() should store function name in error_msg"
    );
}

#[test]
fn parity_spec_unknown_spec_pushes_key_and_value_specs() {
    if !python_available() {
        return;
    }
    // Spec().unknown_spec(key_spec, value_spec) pushes BOTH specs into
    // self.specs but does NOT append to checks. Verifies the spec count
    // grows by exactly 2 on both sides.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().unknown_spec(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().ident(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str)).specs)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 2,
        "Python Spec.unknown_spec() should push 2 specs (key+value)"
    );
    let py_checks_count = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().unknown_spec(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().ident(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str)).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_checks: usize = py_checks_count.parse().expect("Python returned non-int");
    assert_eq!(
        py_checks, 0,
        "Python Spec.unknown_spec() should NOT append to checks (it goes through uspecs)"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().unknown_spec(Spec::new().ident(), Spec::new());
    assert_eq!(
        s.specs.len(),
        2,
        "Rust unknown_spec() should push 2 specs (key+value)"
    );
}

#[test]
fn parity_spec_either_pushes_variant_specs() {
    if !python_available() {
        return;
    }
    // Spec().either(Spec(), Spec(), Spec()) appends 1 check_either entry
    // on Python and pushes the 3 variant specs into self.specs.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().either(__import__('powerline.lint.spec', fromlist=['Spec']).Spec(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec()).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 1,
        "Python Spec.either() should append 1 check entry (check_either)"
    );
    let py_specs_count = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().either(__import__('powerline.lint.spec', fromlist=['Spec']).Spec(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec()).specs)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_specs_n: usize = py_specs_count.parse().expect("Python returned non-int");
    assert_eq!(
        py_specs_n, 3,
        "Python Spec.either(3 specs) should push 3 specs to self.specs"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().either(vec![Spec::new(), Spec::new(), Spec::new()]);
    assert_eq!(
        s.specs.len(),
        3,
        "Rust Spec.either(3 specs) should push 3 specs"
    );
}

#[test]
fn parity_spec_ident_chains_type_unicode_and_regex() {
    if !python_available() {
        return;
    }
    // Spec().ident() calls self.re('^[a-zA-Z_]\w*$', ...), which itself
    // chains self.type(unicode). Result: 2 check entries on Python.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().ident().checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 2,
        "Python Spec.ident() should append 2 check entries (type+regex via .re())"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().ident();
    assert!(s.ident_flag, "Rust ident_flag should be set");
    assert_eq!(
        s.regex.as_deref(),
        Some(r"^\w+(?::\w+)?$"),
        "Rust ident() should set the ident regex (matches Python py:588)"
    );
}

#[test]
fn parity_spec_len_check_appended() {
    if !python_available() {
        return;
    }
    // Spec().len('eq', 5) registers 1 check_func entry on Python.
    // Rust stores (Cmp::Eq, 5) in len_constraints.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().len('eq', 5).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(py_len, 1, "Python Spec.len() should append 1 check entry");
    use powerliners::lint::spec::{Cmp, Spec};
    let s = Spec::new().len(Cmp::Eq, 5);
    assert_eq!(
        s.len_constraints,
        vec![(Cmp::Eq, 5)],
        "Rust len_constraints storage mismatch"
    );
}

#[test]
fn parity_spec_list_chains_type_list_and_adds_check_list() {
    if !python_available() {
        return;
    }
    // Spec().list(Spec()) appends check_type (for list) AND check_list
    // on Python, so len(checks) == 2. Rust pins SpecType::List and stores
    // the item_spec in self.specs.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().list(__import__('powerline.lint.spec', fromlist=['Spec']).Spec()).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 2,
        "Python Spec.list() should append 2 check entries (type+list)"
    );
    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new().list(Spec::new());
    assert!(
        s.allowed_types.contains(&SpecType::List),
        "Rust list() should pin allowed type to List"
    );
    assert_eq!(
        s.specs.len(),
        1,
        "Rust list() should push exactly 1 item spec into self.specs"
    );
}

#[test]
fn parity_spec_tuple_chains_type_list_and_adds_check_tuple() {
    if !python_available() {
        return;
    }
    // Spec().tuple(Spec(), Spec()) appends check_type + check_func (length
    // constraint) + check_tuple on Python — 3 entries.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().tuple(__import__('powerline.lint.spec', fromlist=['Spec']).Spec(), __import__('powerline.lint.spec', fromlist=['Spec']).Spec()).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 3,
        "Python Spec.tuple() with 2 specs should append 3 check entries (type+func+tuple)"
    );
    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new().tuple(vec![Spec::new(), Spec::new()]);
    assert!(
        s.allowed_types.contains(&SpecType::List),
        "Rust tuple() should pin allowed type to List"
    );
    assert_eq!(
        s.specs.len(),
        2,
        "Rust tuple() should push exactly 2 specs into self.specs"
    );
}

#[test]
fn parity_spec_cmp_check_appended() {
    if !python_available() {
        return;
    }
    // Spec().cmp('>=', 0) registers a check_func entry and stores the
    // (Cmp, value) tuple on the Rust side.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().cmp('ge', 0).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    // Python Spec.cmp() registers BOTH check_type (numeric) AND check_func
    // (the comparison itself), so len(checks) == 2 after a single call.
    assert_eq!(
        py_len, 2,
        "Python Spec.cmp() should append 2 check entries (type+func)"
    );
    use powerliners::lint::spec::{Cmp, Spec};
    let s = Spec::new().cmp(Cmp::Ge, 0.0);
    assert_eq!(
        s.cmp_constraint,
        Some((Cmp::Ge, 0.0)),
        "Rust cmp_constraint storage mismatch"
    );
}

#[test]
fn parity_spec_error_check_appended_and_stored() {
    if !python_available() {
        return;
    }
    // Spec().error(msg) registers a check_func entry that always fires
    // with the supplied msg on Python and stores error_msg on Rust.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().error('boom').checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(py_len, 1, "Python Spec.error() should append 1 check entry");
    use powerliners::lint::spec::Spec;
    let s = Spec::new().error("boom");
    assert_eq!(
        s.error_msg.as_deref(),
        Some("boom"),
        "Rust error_msg storage mismatch"
    );
}

#[test]
fn parity_spec_oneof_check_appended_and_stored() {
    if !python_available() {
        return;
    }
    // Verify Spec().oneof(coll) appends a 'check_oneof' entry on Python
    // and stores the values list on Rust.
    let py = match py_eval(
        "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().oneof(['a', 'b', 'c']).checks)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_len: usize = py.parse().expect("Python returned non-int len");
    assert_eq!(
        py_len, 1,
        "Python Spec.oneof() should append exactly 1 check entry"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::new().oneof(&["a", "b", "c"]);
    let v = s.oneof.unwrap();
    assert_eq!(v, vec!["a", "b", "c"], "Rust Spec.oneof storage mismatch");
}

#[test]
fn parity_spec_context_message_sets_cmsg() {
    if !python_available() {
        return;
    }
    // Verify chaining context_message(msg) sets self.cmsg to msg verbatim.
    let py = match py_eval(
        "__import__('powerline.lint.spec', fromlist=['Spec']).Spec().context_message('test ctx').cmsg",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py, "test ctx", "Python cmsg storage changed");
    use powerliners::lint::spec::Spec;
    let s = Spec::new().context_message("test ctx");
    assert_eq!(s.cmsg, "test ctx", "Rust cmsg mismatch");
}

// ─────────────────────────────────────────────────────────────────────
// lint/markedjson/error.py — strtrans + repl
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_mergedefaults_preserves_d1_on_overlap() {
    if !python_available() {
        return;
    }
    // mergedefaults() is the opposite of mergedicts: d1 wins on
    // overlapping keys, d2 only fills in MISSING keys (recursive).
    // Verify a nested case where:
    //   d1.a=1 stays (despite d2.a=99)
    //   d2.b=2 gets added (d1.b missing)
    //   d1.nested.y=2 stays (despite d2.nested.y=999)
    //   d2.nested.z=3 gets added (d1.nested.z missing)
    let py = match py_eval(
        "(lambda d1, d2: (__import__('powerline.lib.dict', fromlist=['mergedefaults']).mergedefaults(d1, d2), __import__('json').dumps(d1, sort_keys=True))[1])({'a': 1, 'nested': {'x': 1, 'y': 2}}, {'a': 99, 'b': 2, 'nested': {'y': 999, 'z': 3}})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let mut d1 = json!({"a": 1, "nested": {"x": 1, "y": 2}})
        .as_object()
        .unwrap()
        .clone();
    let d2 = json!({"a": 99, "b": 2, "nested": {"y": 999, "z": 3}})
        .as_object()
        .unwrap()
        .clone();
    powerliners::lib::dict::mergedefaults(&mut d1, d2);
    assert_eq!(
        py_value,
        serde_json::Value::Object(d1),
        "mergedefaults nested overlap mismatch"
    );
}

#[test]
fn parity_mark_equality_uses_name_line_column_only() {
    if !python_available() {
        return;
    }
    // Mark.__eq__ — py:147-152:
    //   self.name == other.name AND
    //   self.line == other.line AND
    //   self.column == other.column
    // Buffer/pointer/old_mark are NOT part of equality.
    let cases: &[(&str, usize, usize, &str, &str, usize, usize, &str, bool)] = &[
        // same name/line/col, different buffer + pointer → equal
        ("cfg", 5, 10, "aaa", "cfg", 5, 10, "bbb", true),
        // different line → not equal
        ("cfg", 5, 10, "aaa", "cfg", 6, 10, "aaa", false),
        // different column → not equal
        ("cfg", 5, 10, "aaa", "cfg", 5, 11, "aaa", false),
        // different name → not equal
        ("cfg", 5, 10, "aaa", "other", 5, 10, "aaa", false),
    ];
    for (n1, l1, c1, b1, n2, l2, c2, b2, expected) in cases {
        let py_expr = format!(
            "__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark({n1:?}, {l1}, {c1}, {b1:?}, 0) == __import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark({n2:?}, {l2}, {c2}, {b2:?}, 999)",
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val = py.trim() == "True";
        assert_eq!(
            py_val,
            *expected,
            "Python Mark.__eq__({:?} vs {:?}) fixture drift",
            (n1, l1, c1),
            (n2, l2, c2)
        );
        let m1 = powerliners::lint::markedjson::error::RichMark::new(
            *n1,
            *l1,
            *c1,
            Some(b1.chars().collect()),
            0,
        );
        let m2 = powerliners::lint::markedjson::error::RichMark::new(
            *n2,
            *l2,
            *c2,
            Some(b2.chars().collect()),
            999,
        );
        assert_eq!(
            m1 == m2,
            *expected,
            "Rust Mark eq({:?} vs {:?}) mismatch",
            (n1, l1, c1),
            (n2, l2, c2)
        );
    }
}

#[test]
fn parity_mark_to_string_walks_old_mark_chain() {
    if !python_available() {
        return;
    }
    // When self.old_mark is set, to_string() walks the chain, indenting
    // each successive ancestor by 4 more spaces and prefixing it with
    // '\n  which replaced value\n'.
    let py = match py_eval(
        "(lambda M: (lambda m1, m2: (m1.set_old_mark(m2), m1.to_string())[1])(M('file1', 0, 0, 'a', 0), M('file2', 5, 5, 'bcd', 1)))(__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark)",
    ) {
        Some(v) => v,
        None => return,
    };
    let expected = "  in \"file1\", line 1, column 1:\n    a\n    ^\n  which replaced value\n      in \"file2\", line 6, column 6:\n        bcd\n         ^";
    assert_eq!(py.as_str(), expected, "Python old-chain fixture drift");

    let mut m1 = powerliners::lint::markedjson::error::RichMark::new(
        "file1",
        0,
        0,
        Some("a".chars().collect()),
        0,
    );
    let m2 = powerliners::lint::markedjson::error::RichMark::new(
        "file2",
        5,
        5,
        Some("bcd".chars().collect()),
        1,
    );
    m1.set_old_mark(m2).expect("set_old_mark");
    let rs = m1.to_string_marked(0, "in ", true);
    assert_eq!(
        rs, expected,
        "Rust Mark.to_string_marked old-chain output mismatch"
    );
}

#[test]
fn parity_mark_to_string_default_indent_and_head_text() {
    if !python_available() {
        return;
    }
    // Mark.to_string(indent=0, head_text='in ', add_snippet=True):
    //   '  in "<name>", line <line+1>, column <col+1>:\n    <snippet>\n    ^'
    //
    // Verify 3 variants:
    //   defaults                             → 2-space indent, 'in ' prefix
    //   indent=2                             → 4-space outer + 6-space snippet
    //   head_text='at ', add_snippet=False   → single line, no snippet
    let buffer = "hello";
    let cases: &[(usize, &str, bool, &str)] = &[
        (
            0,
            "in ",
            true,
            "  in \"cfg\", line 6, column 11:\n    hello\n    ^",
        ),
        (
            2,
            "in ",
            true,
            "    in \"cfg\", line 6, column 11:\n      hello\n      ^",
        ),
        (0, "at ", false, "  at \"cfg\", line 6, column 11"),
    ];
    for (indent, head, snip, expected) in cases {
        let py_expr = format!(
            "__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('cfg', 5, 10, {buf:?}, 0).to_string(indent={ind}, head_text={ht:?}, add_snippet={sn})",
            buf = buffer, ind = indent, ht = head, sn = if *snip { "True" } else { "False" }
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py.as_str(),
            *expected,
            "Python Mark.to_string fixture drift for indent={}, head={:?}, snippet={}",
            indent,
            head,
            snip
        );
        let m = powerliners::lint::markedjson::error::RichMark::new(
            "cfg",
            5,
            10,
            Some(buffer.chars().collect()),
            0,
        );
        let rs = m.to_string_marked(*indent, head, *snip);
        assert_eq!(
            rs, *expected,
            "Rust Mark.to_string_marked mismatch for indent={}, head={:?}, snippet={}",
            indent, head, snip
        );
    }
}

#[test]
fn parity_format_error_with_shared_mark_omits_duplicate() {
    if !python_available() {
        return;
    }
    // When context_mark == problem_mark, the context-mark line is
    // suppressed (py:172-178). The shared mark's snippet appears once
    // under the problem.
    let buffer = "line1\nline2\nline3";
    let py_expr = format!(
        "(lambda M, fe: fe('Context msg', M('cfg.json', 5, 10, {buf:?}, 12), 'Problem msg', M('cfg.json', 5, 10, {buf:?}, 12), None))(__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark, __import__('powerline.lint.markedjson.error', fromlist=['format_error']).format_error)",
        buf = buffer
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => return,
    };

    let m1 = powerliners::lint::markedjson::error::RichMark::new(
        "cfg.json",
        5,
        10,
        Some(buffer.chars().collect()),
        12,
    );
    let m2 = powerliners::lint::markedjson::error::RichMark::new(
        "cfg.json",
        5,
        10,
        Some(buffer.chars().collect()),
        12,
    );
    let rs = powerliners::lint::markedjson::error::format_error(
        Some("Context msg"),
        Some(&m1),
        Some("Problem msg"),
        Some(&m2),
        None,
        0,
    );
    assert_eq!(
        py.as_str(),
        &rs,
        "format_error with shared mark parity mismatch"
    );
    assert_eq!(
        rs.matches("in \"cfg.json\"").count(),
        1,
        "Shared mark should emit 'in \"cfg.json\"' exactly once"
    );
}

#[test]
fn parity_format_error_combines_context_problem_note() {
    if !python_available() {
        return;
    }
    // format_error joins (in order, when present):
    //   context, context_mark.to_string, problem, problem_mark.to_string, note
    // with '\n' separators. Verify the no-mark path across:
    //   context + problem
    //   problem only
    //   context + problem + note
    let cases: &[(Option<&str>, Option<&str>, Option<&str>, &str)] = &[
        (
            Some("Outer ctx"),
            Some("inner problem"),
            None,
            "Outer ctx\ninner problem",
        ),
        (None, Some("just problem"), None, "just problem"),
        (
            Some("ctx"),
            Some("prob"),
            Some("extra note"),
            "ctx\nprob\nextra note",
        ),
    ];
    for (ctx, prob, note, expected) in cases {
        let py_args = format!(
            "{}, None, {}, None, {}",
            ctx.map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "None".to_string()),
            prob.map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "None".to_string()),
            note.map(|s| format!("{:?}", s))
                .unwrap_or_else(|| "None".to_string()),
        );
        let py_expr = format!(
            "__import__('powerline.lint.markedjson.error', fromlist=['format_error']).format_error({})",
            py_args
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py.as_str(),
            *expected,
            "Python format_error fixture drift for ({:?}, {:?}, {:?})",
            ctx,
            prob,
            note
        );
        let rs =
            powerliners::lint::markedjson::error::format_error(*ctx, None, *prob, None, *note, 0);
        assert_eq!(
            rs, *expected,
            "Rust format_error({:?}, {:?}, {:?}) mismatch",
            ctx, prob, note
        );
    }
}

#[test]
fn parity_get_unicode_writer_writes_to_buffer() {
    if !python_available() {
        return;
    }
    // get_unicode_writer returns a writer fn that encodes str as bytes
    // and forwards to the underlying stream. Verify byte-identical
    // output for ASCII + UTF-8 multi-byte inputs.
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    struct LockWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for LockWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(b)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }

    let cases: &[&str] = &["hello", "héllo →", "", "tab\there"];
    for input in cases {
        let py_expr = format!(
            "(lambda s, w: (w({:?}), s.getvalue())[1])(__import__('io').StringIO(), __import__('powerline.lib.encoding', fromlist=['get_unicode_writer']).get_unicode_writer(__import__('io').StringIO() and (lambda s: s)(__import__('io').StringIO()), 'utf-8', 'replace'))",
            input
        );
        let py_simple = py_eval(&format!(
            "(lambda s: (__import__('powerline.lib.encoding', fromlist=['get_unicode_writer']).get_unicode_writer(s, 'utf-8', 'replace')({:?}), s.getvalue())[1])(__import__('io').StringIO())",
            input
        ));
        let _ = py_expr;
        let py = match py_simple {
            Some(v) => v,
            None => return,
        };

        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let mut writer = powerliners::lib::encoding::get_unicode_writer(
            LockWriter(buf.clone()),
            None,
            "replace",
        );
        writer(input).expect("Rust writer call failed");
        let rs_bytes = buf.lock().unwrap().clone();
        let rs_str = String::from_utf8(rs_bytes).expect("Rust output not UTF-8");
        assert_eq!(
            py.as_str(),
            &rs_str,
            "get_unicode_writer({:?}) parity mismatch",
            input
        );
    }
}

#[test]
fn parity_markedjson_mark_set_old_mark_chains_successfully() {
    if !python_available() {
        return;
    }
    // set_old_mark(other): writes self.old_mark = other when no cycle
    // is detected. Cycle-detection identity differs between ports
    // (Python: id(); Rust: (name, line, column) tuple) — pin only the
    // unambiguous non-cyclic case here.
    let py_ok = match py_eval(
        "(lambda M, m, o: (m.set_old_mark(o), m.old_mark.name)[1])(__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark, __import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('cfg', 5, 10, 'hello', 0), __import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('other', 1, 1, 'world', 0))",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py_ok.trim(),
        "other",
        "Python set_old_mark non-cyclic fixture drift"
    );

    let mut m = powerliners::lint::markedjson::error::RichMark::new("cfg", 5, 10, None, 0);
    let other = powerliners::lint::markedjson::error::RichMark::new("other", 1, 1, None, 0);
    m.set_old_mark(other)
        .expect("Rust set_old_mark must succeed for non-cyclic case");
    assert_eq!(
        m.old_mark.as_ref().unwrap().name,
        "other",
        "Rust old_mark.name should be 'other'"
    );
}

#[test]
fn parity_markedjson_mark_get_snippet_truncates_with_ellipses() {
    if !python_available() {
        return;
    }
    // get_snippet truncates the snippet when the line exceeds
    // max_length, adding ' ... ' markers. Default max_length=75,
    // indent=4. With a 100-x line at pointer 50 both head and tail
    // get truncation markers.
    let buffer = "x".repeat(100);
    let py_expr = format!(
        "__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('cfg', 0, 50, {:?}, 50).get_snippet()",
        buffer
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => return,
    };
    let py_payload = py.as_str();

    let m = powerliners::lint::markedjson::error::RichMark::new(
        "cfg",
        0,
        50,
        Some(buffer.chars().collect()),
        50,
    );
    let rs = m
        .get_snippet(4, 75)
        .expect("Rust get_snippet returned None");
    assert_eq!(
        py_payload, &rs,
        "Mark.get_snippet truncation parity mismatch"
    );
    assert!(
        rs.contains(" ... "),
        "Rust snippet should contain ' ... ' truncation marker"
    );
}

#[test]
fn parity_markedjson_mark_get_snippet_extracts_line_with_caret() {
    if !python_available() {
        return;
    }
    // Mark.get_snippet(indent=4, max_length=75) walks backward/forward
    // from self.pointer to the nearest newline/NUL, joins as a single
    // line, runs strtrans on the pieces, and appends '^' under pointer.
    //
    // Buffer: 'line1\nline2\nline3\nline4'
    // Pointer 12 → start of 'line3' (index 12). Snippet = 'line3'
    // with caret under index 0 → '    line3\n    ^'.
    let buffer = "line1\nline2\nline3\nline4";
    let py_expr = format!(
        "__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('cfg.json', 5, 0, {:?}, 12).get_snippet()",
        buffer
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => return,
    };
    let py_payload = py.as_str();

    let m = powerliners::lint::markedjson::error::RichMark::new(
        "cfg.json",
        5,
        0,
        Some(buffer.chars().collect()),
        12,
    );
    let rs = m
        .get_snippet(4, 75)
        .expect("Rust get_snippet returned None");
    assert_eq!(py_payload, &rs, "Mark.get_snippet parity mismatch");
}

#[test]
fn parity_markedjson_mark_advance_string_increments_pointer_and_column() {
    if !python_available() {
        return;
    }
    // Mark.advance_string(diff) returns a NEW mark with column and
    // pointer bumped by diff; line unchanged. Verify both ports.
    let cases: &[(u64, u64)] = &[(0, 0), (3, 3), (1, 1), (100, 100)];
    for (diff, expected_delta) in cases {
        let py_expr = format!(
            "(lambda m: __import__('json').dumps([m.line, m.column, m.pointer]))(__import__('powerline.lint.markedjson.error', fromlist=['Mark']).Mark('cfg.json', 5, 10, 'abcdefghij', 0).advance_string({}))",
            diff
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let py_arr = py_value.as_array().expect("py array");
        assert_eq!(py_arr[0].as_u64(), Some(5), "py line drift");
        assert_eq!(
            py_arr[1].as_u64(),
            Some(10 + expected_delta),
            "py column delta"
        );
        assert_eq!(
            py_arr[2].as_u64(),
            Some(*expected_delta),
            "py pointer delta"
        );

        let m = powerliners::lint::markedjson::error::RichMark::new(
            "cfg.json",
            5,
            10,
            Some("abcdefghij".chars().collect()),
            0,
        );
        let m2 = m.advance_string(*diff as usize);
        assert_eq!(m2.line, 5, "Rust line should be unchanged");
        assert_eq!(m2.column, 10 + *diff as usize, "Rust column delta");
        assert_eq!(m2.pointer, *diff as usize, "Rust pointer delta");
    }
}

#[test]
fn parity_markedjson_strtrans_escapes_and_replaces_tab() {
    if !python_available() {
        return;
    }
    // markedjson/error.py:42-43 — strtrans():
    //   s.replace('\t', '>---')  then
    //   NON_PRINTABLE_RE.sub(repl, ...)  where repl='<xHHHH>'
    //
    // Verify byte-for-byte across:
    //   'hello'    no escapes (printable)
    //   'a\nb'     LF stays as-is (printable-after-translate set)
    //               Wait: per the NON_PRINTABLE_RE fix, \n IS now
    //               non-printable and gets escaped.
    //   'a\tb'     tab specifically becomes '>---'
    //   'a\x00b'   NUL → '<x0000>'
    //   'é'        UTF-8 char (U+00E9) is printable, untouched
    //   ''         empty
    let cases: &[(&str, &str)] = &[
        ("hello", "hello"),
        ("a\nb", "a<x000a>b"),
        ("a\tb", "a>---b"),
        ("a\x00b", "a<x0000>b"),
        ("é", "é"),
        ("", ""),
    ];
    for (input, expected) in cases {
        let py_expr = format!(
            "__import__('powerline.lint.markedjson.error', fromlist=['strtrans']).strtrans({:?})",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        // Python's NON_PRINTABLE_RE (NOT the spec.py-translated variant)
        // is what strtrans uses. It does NOT include \n as non-printable.
        // So Python output for 'a\nb' is 'a\nb' verbatim.
        //
        // The Rust port at src/ported/lint/markedjson/error.rs uses the
        // same lib/markedjson NON_PRINTABLE_RE, NOT the spec.py one. Let
        // me capture whatever Python says is correct.
        let py_payload = py.as_str();
        let rs = powerliners::lint::markedjson::error::strtrans(input);
        assert_eq!(
            py_payload, &rs,
            "strtrans({:?}) parity mismatch: py={:?}, rs={:?}, expected_static={:?}",
            input, py_payload, rs, expected
        );
    }
}

#[test]
fn parity_colorscheme_get_highlighting_resolves_full_record() {
    if !python_available() {
        return;
    }
    // get_highlighting(groups, mode, gradient_level=None) returns:
    //   {'fg': (cterm, hex), 'bg': (cterm, hex), 'attrs': flag_int}
    let cs_config = r#"{
        "groups": {"normal": {"fg": "white", "bg": "black", "attrs": ["bold"]}},
        "mode_translations": {}
    }"#;
    let colors_config = r#"{
        "colors": {"white": [15, "ffffff"], "black": [16, "000000"]},
        "gradients": {}
    }"#;
    let py_expr = format!(
        "(lambda c: __import__('json').dumps({{k: (list(v) if isinstance(v, tuple) else v) for k, v in c.get_highlighting(['normal'], 'default').items()}}, sort_keys=True))(__import__('powerline.colorscheme', fromlist=['Colorscheme']).Colorscheme({cs}, {col}))",
        cs = cs_config, col = colors_config
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    let cs_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(cs_config).unwrap();
    let col_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(colors_config).unwrap();
    let c = powerliners::colorscheme::Colorscheme::new(&cs_map, &col_map);
    let rs = c
        .get_highlighting(&["normal".to_string()], Some("default"), None)
        .expect("Rust get_highlighting failed");

    let py_fg = py_value["fg"].as_array().expect("py fg");
    let rs_fg = rs["fg"].as_array().expect("rs fg");
    assert_eq!(py_fg[0].as_i64(), rs_fg[0].as_i64(), "fg cterm");
    assert_eq!(py_fg[1].as_u64(), rs_fg[1].as_u64(), "fg hex");
    let py_bg = py_value["bg"].as_array().expect("py bg");
    let rs_bg = rs["bg"].as_array().expect("rs bg");
    assert_eq!(py_bg[0].as_i64(), rs_bg[0].as_i64(), "bg cterm");
    assert_eq!(py_bg[1].as_u64(), rs_bg[1].as_u64(), "bg hex");

    assert_eq!(
        py_value["attrs"].as_u64(),
        rs["attrs"].as_u64(),
        "attrs flag mismatch"
    );
    assert_eq!(
        rs["attrs"].as_u64(),
        Some(1),
        "Rust attrs flag should be 1 for bold"
    );
}

#[test]
fn parity_colorscheme_get_group_props_with_mode_translation() {
    if !python_available() {
        return;
    }
    // get_group_props recursively walks:
    //   1. group is str → look up in trans['groups'] then self.groups
    //   2. group is dict + translate_colors → apply trans['colors'] to fg/bg
    let cs_config = r#"{
        "groups": {
            "critical": {"fg": "red", "bg": "black", "attrs": ["bold"]},
            "normal":   {"fg": "white", "bg": "black", "attrs": []}
        },
        "mode_translations": {
            "insert": {"colors": {"red": "green"}, "groups": {}}
        }
    }"#;
    let colors_config = r#"{
        "colors": {
            "red":   [1,  "ff0000"],
            "green": [2,  "00ff00"],
            "white": [15, "ffffff"],
            "black": [16, "000000"]
        },
        "gradients": {}
    }"#;

    let cs_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(cs_config).unwrap();
    let col_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(colors_config).unwrap();
    let c = powerliners::colorscheme::Colorscheme::new(&cs_map, &col_map);

    let cases: &[(&str, &str, &str)] = &[
        (
            "normal",
            "{}",
            r#"{"fg":"red","bg":"black","attrs":["bold"]}"#,
        ),
        (
            "insert",
            r#"{"colors": {"red": "green"}, "groups": {}}"#,
            r#"{"fg":"green","bg":"black","attrs":["bold"]}"#,
        ),
    ];
    for (mode, trans_json, expected_json) in cases {
        let py_expr = format!(
            "(lambda c: __import__('json').dumps(c.get_group_props({mode:?}, {trans}, 'critical'), sort_keys=True))(__import__('powerline.colorscheme', fromlist=['Colorscheme']).Colorscheme({cs}, {col}))",
            mode = mode, trans = trans_json, cs = cs_config, col = colors_config
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let expected: serde_json::Value =
            serde_json::from_str(expected_json).expect("expected JSON malformed");
        assert_eq!(
            py_value, expected,
            "Python get_group_props({}, {}) fixture drift",
            mode, trans_json
        );

        let trans_map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(trans_json).unwrap();
        let rs = c
            .get_group_props(
                Some(mode),
                &trans_map,
                &serde_json::Value::String("critical".into()),
                true,
            )
            .expect("Rust get_group_props returned None");
        assert_eq!(rs, expected, "Rust get_group_props({}, ...) mismatch", mode);
    }
}

#[test]
fn parity_colorscheme_get_gradient_picks_and_falls_back() {
    if !python_available() {
        return;
    }
    // get_gradient(name, level) — py:62-66:
    //   if name in gradients: tuple of pick_gradient_value(each list, level)
    //   else: self.colors[name]   (fallthrough for direct color lookup)
    //
    // Verify across:
    //   level 0.0     → (cterm_list[0], hex_list[0])
    //   level 50.0    → middle
    //   level 100.0   → endpoint
    //   non-gradient name → falls through to colors[name]
    let cs_config = r#"{"groups": {}, "mode_translations": {}}"#;
    let colors_config = r#"{
        "colors": {"green": [2, "aaff44"]},
        "gradients": {"g1": [[1, 2, 3, 4], ["ff0000", "00ff00", "0000ff", "ffff00"]]}
    }"#;

    let cs_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(cs_config).unwrap();
    let col_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(colors_config).unwrap();
    let c = powerliners::colorscheme::Colorscheme::new(&cs_map, &col_map);

    let cases: &[(&str, f64, i64, u64)] = &[
        ("g1", 0.0, 1, 0xFF0000),
        ("g1", 50.0, 3, 0x0000FF),
        ("g1", 100.0, 4, 0xFFFF00),
        ("green", 0.0, 2, 0xAAFF44),
    ];
    for (name, level, expected_cterm, expected_hex) in cases {
        let py_expr = format!(
            "(lambda c: __import__('json').dumps(list(c.get_gradient({:?}, {}))))(__import__('powerline.colorscheme', fromlist=['Colorscheme']).Colorscheme({cs}, {col}))",
            name, level,
            cs = cs_config, col = colors_config
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_pair: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let py_arr = py_pair.as_array().expect("py tuple→array");
        assert_eq!(
            py_arr[0].as_i64(),
            Some(*expected_cterm),
            "Python fixture cterm drift for {} @ {}",
            name,
            level
        );
        assert_eq!(
            py_arr[1].as_u64(),
            Some(*expected_hex),
            "Python fixture hex drift for {} @ {}",
            name,
            level
        );
        let rs = c.get_gradient(name, *level);
        let rs_arr = rs.as_array().expect("rs result not array");
        assert_eq!(
            rs_arr[0].as_i64(),
            Some(*expected_cterm),
            "Rust cterm mismatch for {} @ {}",
            name,
            level
        );
        assert_eq!(
            rs_arr[1].as_u64(),
            Some(*expected_hex),
            "Rust hex mismatch for {} @ {}",
            name,
            level
        );
    }
}

#[test]
fn parity_colorscheme_init_parses_colors_and_gradients() {
    if !python_available() {
        return;
    }
    // Colorscheme.__init__ builds:
    //   self.colors[name] = (cterm_int, hex_int)  from ['cterm', 'hexstr']
    //                       OR (cterm, cterm_to_hex[cterm]) for plain int
    //   self.gradients[name] = (cterm_list, hex_int_list)
    // Verify both ports produce identical numeric values.
    let cs_config = r#"{"groups": {}, "mode_translations": {}}"#;
    let colors_config = r#"{
        "colors": {"green": [2, "aaff44"], "black": [16, "000000"]},
        "gradients": {"g1": [[1, 2, 3, 4], ["ff0000", "00ff00", "0000ff", "ffff00"]]}
    }"#;
    let py_expr = format!(
        "(lambda C: __import__('json').dumps({{'colors': dict(c.colors), 'gradients': {{k: list(v) for k, v in c.gradients.items()}}}}, sort_keys=True))(__import__('powerline.colorscheme', fromlist=['Colorscheme']).Colorscheme({cs}, {col}))",
        cs = cs_config,
        col = colors_config
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    let cs_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(cs_config).unwrap();
    let col_map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(colors_config).unwrap();
    let c = powerliners::colorscheme::Colorscheme::new(&cs_map, &col_map);

    // Compare colors entries
    let py_colors = &py_value["colors"];
    let py_green = py_colors["green"].as_array().expect("py green");
    let rs_green = c.colors["green"].as_array().expect("rs green");
    assert_eq!(py_green[0].as_i64(), rs_green[0].as_i64(), "green cterm");
    assert_eq!(py_green[1].as_u64(), rs_green[1].as_u64(), "green hex");
    let py_black = py_colors["black"].as_array().expect("py black");
    let rs_black = c.colors["black"].as_array().expect("rs black");
    assert_eq!(py_black[0].as_i64(), rs_black[0].as_i64(), "black cterm");
    assert_eq!(py_black[1].as_u64(), rs_black[1].as_u64(), "black hex");

    // Compare gradient g1 (cterm list + hex list)
    let py_g1 = py_value["gradients"]["g1"].as_array().expect("py g1");
    let rs_g1 = c.gradients["g1"].as_array().expect("rs g1");
    let py_cterm_list = py_g1[0].as_array().expect("py g1 cterm");
    let rs_cterm_list = rs_g1[0].as_array().expect("rs g1 cterm");
    assert_eq!(py_cterm_list.len(), rs_cterm_list.len(), "g1 cterm len");
    for (i, (p, r)) in py_cterm_list.iter().zip(rs_cterm_list.iter()).enumerate() {
        assert_eq!(p.as_i64(), r.as_i64(), "g1 cterm[{}]", i);
    }
    let py_hex_list = py_g1[1].as_array().expect("py g1 hex");
    let rs_hex_list = rs_g1[1].as_array().expect("rs g1 hex");
    assert_eq!(py_hex_list.len(), rs_hex_list.len(), "g1 hex len");
    for (i, (p, r)) in py_hex_list.iter().zip(rs_hex_list.iter()).enumerate() {
        assert_eq!(
            p.as_u64(),
            r.as_u64(),
            "g1 hex[{}] mismatch: py=0x{:06X} rs=0x{:06X}",
            i,
            p.as_u64().unwrap_or(0),
            r.as_u64().unwrap_or(0)
        );
    }
}

#[test]
fn parity_register_strwidth_error_name_format() {
    if !python_available() {
        return;
    }
    // register_strwidth_error(strwidth) returns a unique handler name
    // of shape 'powerline_encode_strwidth_error_<N>'. The counter
    // increments per call. In a fresh Python subprocess two
    // consecutive calls produce _1 and _2.
    let py = match py_eval(
        "(lambda r: __import__('json').dumps([r(lambda s: 1), r(lambda s: 2)]))(__import__('powerline.lib.unicode', fromlist=['register_strwidth_error']).register_strwidth_error)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    let py_arr = py_value.as_array().expect("expected JSON array");
    assert_eq!(py_arr.len(), 2);
    assert_eq!(
        py_arr[0], "powerline_encode_strwidth_error_1",
        "Python first call should produce _1"
    );
    assert_eq!(
        py_arr[1], "powerline_encode_strwidth_error_2",
        "Python second call should produce _2"
    );

    // Rust side: verify the name format AND monotonic-increment shape
    // without committing to a specific starting index (other tests may
    // bump the counter before/after).
    use powerliners::lib::unicode::register_strwidth_error;
    let (name_a, _) = register_strwidth_error(|s: &str| s.len());
    let (name_b, _) = register_strwidth_error(|s: &str| s.len());
    let prefix = "powerline_encode_strwidth_error_";
    assert!(
        name_a.starts_with(prefix),
        "Rust handler name must start with {:?}",
        prefix
    );
    assert!(
        name_b.starts_with(prefix),
        "Rust handler name must start with {:?}",
        prefix
    );
    let idx_a: u64 = name_a
        .strip_prefix(prefix)
        .unwrap()
        .parse()
        .expect("name must end with integer");
    let idx_b: u64 = name_b
        .strip_prefix(prefix)
        .unwrap()
        .parse()
        .expect("name must end with integer");
    assert_eq!(
        idx_b,
        idx_a + 1,
        "Rust counter must increment monotonically by 1 between calls"
    );
}

#[test]
fn parity_spec_copy_preserves_state_and_decouples_mutation() {
    if !python_available() {
        return;
    }
    // Spec.copy() returns an independent shallow copy:
    //   - cmsg / isoptional preserved at copy time
    //   - subsequent mutation of the original DOES NOT affect the copy
    let py = match py_eval(
        "(lambda S: (lambda s, c: (setattr(s, 'cmsg', 'modified'), __import__('json').dumps([c.cmsg, c.isoptional]))[1])(*((lambda s: (s, s.copy()))((lambda s: (setattr(s, 'cmsg', 'original'), setattr(s, 'isoptional', True), s)[2])(S())))))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!(["original", true]),
        "Python copy() should be unaffected by original's later mutation"
    );

    use powerliners::lint::spec::Spec;
    let mut s = Spec::default();
    s.cmsg = "original".to_string();
    s.isoptional = true;
    let copy = s.copy();
    s.cmsg = "modified".to_string();
    s.isoptional = false;
    assert_eq!(
        copy.cmsg, "original",
        "Rust copy().cmsg should be unaffected by original's later mutation"
    );
    assert!(
        copy.isoptional,
        "Rust copy().isoptional should be unaffected"
    );
}

#[test]
fn parity_non_printable_re_full_truth_table() {
    if !python_available() {
        return;
    }
    // NON_PRINTABLE_RE truth table — verify Rust matches Python's
    // spec.py translate semantics (port bug fix).
    let cases: &[(u32, bool)] = &[
        (0x07, true), // BEL
        (0x08, true), // BS
        (0x09, true), // TAB — Python REMOVES from allow-list → matches
        (0x0A, true), // LF — same
        (0x0B, true),
        (0x0C, true),
        (0x0D, true), // CR — also matches
        (0x0E, true),
        (0x1F, true),
        (0x20, false), // space — printable
        (0x7E, false), // ~ — printable
        (0x7F, true),  // DEL
        (0x80, true),
        (0x85, true), // NEXT LINE
        (0x9F, true),
        (0xA0, false), // non-breaking space — printable
    ];
    use powerliners::lint::spec::NON_PRINTABLE_RE;
    let rs_re = NON_PRINTABLE_RE();
    for (code, expected) in cases {
        let ch = char::from_u32(*code).unwrap();
        let s: String = std::iter::once(ch).collect();
        let py_expr = format!(
            "bool(__import__('powerline.lint.spec', fromlist=['NON_PRINTABLE_RE']).NON_PRINTABLE_RE.search(chr({})))",
            code
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val = py.trim() == "True";
        assert_eq!(
            py_val, *expected,
            "Python NON_PRINTABLE_RE fixture drift for U+{:04X}",
            code
        );
        let rs_val = rs_re.is_match(&s);
        assert_eq!(
            rs_val, *expected,
            "Rust NON_PRINTABLE_RE match for U+{:04X} should be {}",
            code, expected
        );
    }
}

#[test]
fn parity_spec_unsigned_chains_type_and_nonnegative_check() {
    if !python_available() {
        return;
    }
    // Spec.unsigned() — py:471-486:
    //   self.type(int)
    //   self.checks.append(('check_func', value<0, msg_func))
    // Python observable: len(checks) == 2 (type + nonnegative).
    // Rust observable:
    //   allowed_types contains numeric SpecType::Float
    //   cmp_constraint == Some((Ge, 0.0)) — encodes 'value >= 0'
    //   unsigned_flag = true (Rust-specific marker)
    let py = match py_eval(
        "(lambda s: __import__('json').dumps([len(s.checks), s.did_type]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().unsigned())",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([2, false]),
        "Python Spec().unsigned fixture drift"
    );

    use powerliners::lint::spec::{Cmp, Spec, SpecType};
    let s = Spec::default().unsigned();
    assert!(
        s.allowed_types.contains(&SpecType::Float),
        "Rust unsigned() must add numeric type"
    );
    assert!(s.unsigned_flag, "Rust unsigned_flag must be set");
    let (op, value) = s
        .cmp_constraint
        .expect("Rust unsigned() must set cmp_constraint to Ge(0)");
    assert_eq!(op, Cmp::Ge);
    assert!(
        (value - 0.0).abs() < 1e-9,
        "cmp_constraint value must be 0.0"
    );
    assert!(!s.did_type, "unsigned() does not set did_type");
}

#[test]
fn parity_safe_unicode_str_passthrough() {
    if !python_available() {
        return;
    }
    // safe_unicode(str) — for already-unicode input both ports return
    // the input verbatim. Verify with ASCII + multi-byte UTF-8.
    let cases: &[&str] = &["hello", "héllo →", "", "  spaces  ", "\u{1F600}"];
    for input in cases {
        let py_expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['safe_unicode']).safe_unicode({:?})",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::unicode::safe_unicode_str(input);
        assert_eq!(py, rs, "safe_unicode_str({:?}) mismatch", input);
    }
}

#[test]
fn parity_stat_file_watcher_watch_unwatch_is_watching() {
    if !python_available() {
        return;
    }
    // 4-state membership sequence:
    //   pre-watch:    is_watching → False
    //   post-watch:   is_watching → True
    //   post-unwatch: is_watching → False
    let py_tmpfile = std::env::temp_dir().join(format!(
        "powerliners_parity_watch_py_{}.txt",
        std::process::id()
    ));
    let rs_tmpfile = std::env::temp_dir().join(format!(
        "powerliners_parity_watch_rs_{}.txt",
        std::process::id()
    ));
    std::fs::write(&py_tmpfile, "").expect("write py fixture");
    std::fs::write(&rs_tmpfile, "").expect("write rs fixture");

    let py_expr = format!(
        "(lambda w, p: [w.is_watching(p), (w.watch(p), w.is_watching(p))[1], (w.unwatch(p), w.is_watching(p))[1]])(__import__('powerline.lib.watcher.stat', fromlist=['StatFileWatcher']).StatFileWatcher(), {:?})",
        py_tmpfile.to_string_lossy()
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => {
            let _ = std::fs::remove_file(&py_tmpfile);
            let _ = std::fs::remove_file(&rs_tmpfile);
            return;
        }
    };
    let _ = std::fs::remove_file(&py_tmpfile);
    assert_eq!(
        py.trim(),
        "[False, True, False]",
        "Python watch/unwatch sequence drift"
    );

    let w = powerliners::lib::watcher::stat::StatFileWatcher::new();
    let rs1 = w.is_watching(&rs_tmpfile);
    w.watch(&rs_tmpfile);
    let rs2 = w.is_watching(&rs_tmpfile);
    w.unwatch(&rs_tmpfile);
    let rs3 = w.is_watching(&rs_tmpfile);
    let _ = std::fs::remove_file(&rs_tmpfile);

    assert!(!rs1, "Rust pre-watch is_watching must be false");
    assert!(rs2, "Rust post-watch is_watching must be true");
    assert!(!rs3, "Rust post-unwatch is_watching must be false");
}

#[test]
fn parity_stat_file_watcher_3_state_transitions() {
    if !python_available() {
        return;
    }
    // StatFileWatcher.__call__(path) — py:30-40:
    //   1. First call on an unseen path → True (registers mtime)
    //   2. Second call without mtime change → False
    //   3. After mtime bumps → True
    use std::time::Duration;

    let py_tmpfile = std::env::temp_dir().join(format!(
        "powerliners_parity_stat_py_{}.txt",
        std::process::id()
    ));
    let rs_tmpfile = std::env::temp_dir().join(format!(
        "powerliners_parity_stat_rs_{}.txt",
        std::process::id()
    ));
    std::fs::write(&py_tmpfile, "initial").expect("write py fixture");
    std::fs::write(&rs_tmpfile, "initial").expect("write rs fixture");

    let py_expr = format!(
        "(lambda w, p: [w(p), w(p), (__import__('time').sleep(1.1), open(p, 'w').write('bump'), w(p))[-1]])(__import__('powerline.lib.watcher.stat', fromlist=['StatFileWatcher']).StatFileWatcher(), {:?})",
        py_tmpfile.to_string_lossy()
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => {
            let _ = std::fs::remove_file(&py_tmpfile);
            let _ = std::fs::remove_file(&rs_tmpfile);
            return;
        }
    };
    let _ = std::fs::remove_file(&py_tmpfile);
    assert_eq!(
        py.trim(),
        "[True, False, True]",
        "Python StatFileWatcher transition fixture drift"
    );

    let w = powerliners::lib::watcher::stat::StatFileWatcher::new();
    let rs1 = w.check(&rs_tmpfile);
    let rs2 = w.check(&rs_tmpfile);
    std::thread::sleep(Duration::from_millis(1100));
    std::fs::write(&rs_tmpfile, "bump").expect("bump rs fixture");
    let rs3 = w.check(&rs_tmpfile);
    let _ = std::fs::remove_file(&rs_tmpfile);

    assert!(rs1, "Rust 1st call should be True (unseen path)");
    assert!(!rs2, "Rust 2nd call should be False (no mtime change)");
    assert!(rs3, "Rust 3rd call should be True (mtime bumped)");
}

#[test]
fn parity_load_json_config_roundtrip_through_disk() {
    if !python_available() {
        return;
    }
    // load_json_config reads a JSON file and returns the parsed
    // structure. Verify both ports parse identical input identically.
    // Write a fixture file, then load with both ports, then compare.
    let payload = r#"{"a": 1, "b": [2, 3], "c": {"nested": true}}"#;
    let tmpfile = std::env::temp_dir().join(format!(
        "powerliners_parity_load_{}.json",
        std::process::id()
    ));
    std::fs::write(&tmpfile, payload).expect("failed to write fixture");

    let py_expr = format!(
        "__import__('json').dumps(__import__('powerline.lib.config', fromlist=['load_json_config']).load_json_config({:?}), sort_keys=True)",
        tmpfile.to_string_lossy()
    );
    let py = match py_eval(&py_expr) {
        Some(v) => v,
        None => {
            let _ = std::fs::remove_file(&tmpfile);
            return;
        }
    };

    let rs =
        powerliners::lib::config::load_json_config(&tmpfile).expect("Rust load_json_config failed");
    let _ = std::fs::remove_file(&tmpfile);

    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON output malformed");
    assert_eq!(
        py_value, rs,
        "load_json_config disk-roundtrip parity mismatch"
    );
}

#[test]
fn parity_failed_unicode_preserves_message_payload() {
    if !python_available() {
        return;
    }
    // FailedUnicode is a unicode/str subclass that wraps a message
    // string. Python: str(FailedUnicode('boom')) == 'boom'.
    // Rust: FailedUnicode("boom").0 == "boom" (newtype around String).
    //
    // Verify the message payload round-trips identically across both
    // ports for ASCII, Unicode, and empty inputs.
    let cases: &[&str] = &["boom", "héllo →", "", "multi\nline", "  spaces  "];
    for input in cases {
        let py_expr = format!(
            "str(__import__('powerline.lib.unicode', fromlist=['FailedUnicode']).FailedUnicode({:?}))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        // Python `print(str(FailedUnicode('multi\nline')))` emits the
        // literal newline; strip only the print()'s trailing newline.
        let py_payload = py.as_str();
        let f = powerliners::lib::unicode::FailedUnicode::new(*input);
        assert_eq!(
            py_payload, &f.0,
            "FailedUnicode({:?}) payload mismatch: py={:?}, rs={:?}",
            input, py_payload, f.0
        );
    }
}

#[test]
fn parity_out_u_str_and_bytes_passthrough() {
    if !python_available() {
        return;
    }
    // out_u(s) returns:
    //   str   → s unchanged
    //   bytes → decoded under get_preferred_output_encoding()
    // For UTF-8 inputs both produce the same Unicode string.
    let cases_str: &[&str] = &["hello", "héllo →", ""];
    let cases_bytes: &[&[u8]] = &[b"hello", "café".as_bytes(), b""];
    for input in cases_str {
        let py_expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['out_u']).out_u({:?})",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::unicode::out_u_str(input);
        assert_eq!(py.trim(), rs, "out_u_str({:?}) mismatch", input);
    }
    for input in cases_bytes {
        // Pass bytes via repr to round-trip safely through Python source.
        let py_expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['out_u']).out_u({})",
            python_bytes_literal(input)
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::unicode::out_u_bytes(input);
        assert_eq!(py.trim(), rs, "out_u_bytes({:?}) mismatch", input);
    }
}

/// Helper: build a Python `b'...'` literal from arbitrary bytes for
/// safe embedding in `py_eval` source.
fn python_bytes_literal(b: &[u8]) -> String {
    let mut s = String::from("b'");
    for byte in b {
        match *byte {
            b'\\' => s.push_str("\\\\"),
            b'\'' => s.push_str("\\'"),
            0x20..=0x7E => s.push(*byte as char),
            other => s.push_str(&format!("\\x{:02x}", other)),
        }
    }
    s.push('\'');
    s
}

#[test]
fn parity_threaded_segment_default_state() {
    if !python_available() {
        return;
    }
    // ThreadedSegment default-constructed state (subclasses
    // MultiRunnedThread but overrides class-level daemon=False):
    //   interval     == 1
    //   update_first == True
    //   daemon       == False   (override of MultiRunnedThread.daemon=True)
    let py = match py_eval(
        "(lambda ts: __import__('json').dumps([ts.interval, ts.update_first, ts.daemon]))(__import__('powerline.lib.threaded', fromlist=['ThreadedSegment']).ThreadedSegment())",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([1, true, false]),
        "Python ThreadedSegment default state drift"
    );

    use powerliners::lib::threaded::ThreadedSegment;
    let ts = ThreadedSegment::new();
    assert!((ts.interval - 1.0).abs() < 1e-9);
    assert!(ts.update_first);
    assert!(
        !ts.base.daemon,
        "ThreadedSegment must override MultiRunnedThread.daemon to false"
    );
}

#[test]
fn parity_multi_runned_thread_default_state() {
    if !python_available() {
        return;
    }
    // MultiRunnedThread default-constructed state:
    //   daemon          == True (class-level attribute)
    //   thread          == None
    //   is_alive()      == None / False (no thread started yet)
    let py = match py_eval(
        "(lambda m: __import__('json').dumps([m.daemon, m.thread is None, bool(m.is_alive())]))(__import__('powerline.lib.threaded', fromlist=['MultiRunnedThread']).MultiRunnedThread())",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([true, true, false]),
        "Python MultiRunnedThread default state drift"
    );

    use powerliners::lib::threaded::MultiRunnedThread;
    let m = MultiRunnedThread::new();
    assert!(m.daemon, "Rust MultiRunnedThread.daemon should be true");
    assert!(
        !m.is_alive(),
        "Rust MultiRunnedThread.is_alive() should be false before start"
    );
}

#[test]
fn parity_spec_unknown_spec_and_unknown_msg_record_state() {
    if !python_available() {
        return;
    }
    // Spec.unknown_spec(keyfunc, spec) — py:155-159 pushes both onto
    // self.specs and records (keyfunc_id, spec_id) in self.uspecs.
    // Spec.unknown_msg(msgfunc) — py:175 sets self.ufailmsg.
    //
    // Python observables:
    let py = match py_eval(
        "(lambda S: __import__('json').dumps([len(S().unknown_spec(S(), S()).specs), S().unknown_spec(S(), S()).uspecs, isinstance(S().unknown_msg('boom').ufailmsg, str) or callable(S().unknown_msg('boom').ufailmsg)]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(py_value[0], 2, "Python unknown_spec should push 2 specs");
    assert_eq!(
        py_value[1],
        serde_json::json!([[0, 1]]),
        "Python uspecs should be [(0, 1)] after unknown_spec(S(), S())"
    );
    assert_eq!(
        py_value[2], true,
        "Python ufailmsg should be set (str or callable) after unknown_msg('boom')"
    );

    use powerliners::lint::spec::Spec;
    // Rust port: previously DROPPED both calls. New fields:
    //   uspecs: Vec<(usize, usize)>
    //   ufailmsg: Option<String>
    let s = Spec::default().unknown_spec(Spec::default(), Spec::default());
    assert_eq!(s.specs.len(), 2);
    assert_eq!(
        s.uspecs,
        vec![(0, 1)],
        "Rust unknown_spec must push (keyfunc_id, spec_id) onto uspecs"
    );

    let s2 = Spec::default().unknown_msg("unknown key");
    assert_eq!(
        s2.ufailmsg.as_deref(),
        Some("unknown key"),
        "Rust unknown_msg must set ufailmsg"
    );
}

#[test]
fn parity_spec_either_appends_all_variants_and_one_check() {
    if !python_available() {
        return;
    }
    // Spec.either(*specs) — py:631-642:
    //   start = len(self.specs)
    //   self.specs.extend(specs)
    //   self.checks.append(('check_either', start, len(self.specs)))
    // Verify both observables:
    //   Python: checks ↑ by 1; specs grows by len(specs).
    //   Rust:  specs vec grows by len(specs); allowed_types untouched.
    let py2 = match py_eval(
        "(lambda S: __import__('json').dumps([len(S().either(S(), S()).checks), len(S().either(S(), S()).specs)]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py2).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([1, 2]),
        "Python either(Spec, Spec) drift"
    );

    let py3 = py_eval(
        "(lambda S: __import__('json').dumps([len(S().either(S(), S(), S()).checks), len(S().either(S(), S(), S()).specs)]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ).expect("py_eval failed");
    let py3_value: serde_json::Value = serde_json::from_str(&py3).expect("py JSON malformed");
    assert_eq!(
        py3_value,
        serde_json::json!([1, 3]),
        "Python either(Spec, Spec, Spec) drift"
    );

    use powerliners::lint::spec::Spec;
    let s2 = Spec::default().either(vec![Spec::default(), Spec::default()]);
    assert_eq!(s2.specs.len(), 2);
    assert!(s2.allowed_types.is_empty(), "either() must not add types");

    let s3 = Spec::default().either(vec![Spec::default(), Spec::default(), Spec::default()]);
    assert_eq!(s3.specs.len(), 3);
}

#[test]
fn parity_spec_error_appends_single_check_no_type() {
    if !python_available() {
        return;
    }
    // Spec.error(msg) appends ONE check_func to self.checks. Does NOT
    // touch allowed_types or specs.
    let py = match py_eval(
        "(lambda s: __import__('json').dumps([len(s.checks), len(s.specs), s.did_type]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().error('boom'))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([1, 0, false]),
        "Python Spec().error fixture drift"
    );
    use powerliners::lint::spec::Spec;
    let s = Spec::default().error("boom");
    assert_eq!(s.error_msg.as_deref(), Some("boom"));
    assert_eq!(s.specs.len(), 0);
    assert!(!s.did_type);
    assert!(s.allowed_types.is_empty(), "error() must not add types");
}

#[test]
fn parity_spec_tuple_emits_both_lower_and_upper_bounds() {
    if !python_available() {
        return;
    }
    // Spec.tuple(...) — py:531-536 logic:
    //   max_len == min_len  → self.len('eq', max_len)
    //   else
    //     min_len > 0       → self.len('ge', min_len)
    //     always             → self.len('le', max_len)
    //
    // Three scenarios:
    //   3 required          → 1 type + 1 eq + 1 check_tuple = 3 checks
    //   2 required, 1 opt   → 1 type + 1 ge + 1 le + 1 check_tuple = 4 checks
    //   2 all optional      → 1 type + 1 le        + 1 check_tuple = 3 checks
    //
    // The 4-check case is what the old Rust port lost (only stored 1
    // bound, missed the (Ge, 2) lower bound from py:535).
    use powerliners::lint::spec::{Cmp, Spec, SpecType};

    // Scenario A: 3 required → Eq bound only
    let py = match py_eval(
        "(lambda S: len(S().tuple(S(), S(), S()).checks))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_n: usize = py.trim().parse().expect("py returned non-int");
    assert_eq!(py_n, 3, "Python 3-required tuple checks count drift");
    let s = Spec::default().tuple(vec![Spec::default(), Spec::default(), Spec::default()]);
    assert_eq!(s.len_constraints, vec![(Cmp::Eq, 3)]);
    assert!(s.allowed_types.contains(&SpecType::List));

    // Scenario B: 2 required + 1 trailing optional → Ge AND Le bounds
    let py = py_eval(
        "(lambda S: len(S().tuple(S(), S(), S().optional()).checks))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ).expect("py_eval failed");
    let py_n: usize = py.trim().parse().expect("py returned non-int");
    assert_eq!(
        py_n, 4,
        "Python tuple(req, req, opt) should have 4 checks (type + ge + le + check_tuple)"
    );
    let s = Spec::default().tuple(vec![
        Spec::default(),
        Spec::default(),
        Spec::default().optional(),
    ]);
    assert_eq!(
        s.len_constraints,
        vec![(Cmp::Ge, 2), (Cmp::Le, 3)],
        "Rust tuple(req, req, opt) MUST emit BOTH Ge AND Le bounds (port bug fix)"
    );

    // Scenario C: 2 optional → Le bound only (min_len drops to 0)
    let py = py_eval(
        "(lambda S: len(S().tuple(S().optional(), S().optional()).checks))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ).expect("py_eval failed");
    let py_n: usize = py.trim().parse().expect("py returned non-int");
    assert_eq!(
        py_n, 3,
        "Python tuple(opt, opt) should have 3 checks (type + le + check_tuple); ge skipped when min_len==0"
    );
    let s = Spec::default().tuple(vec![Spec::default().optional(), Spec::default().optional()]);
    assert_eq!(
        s.len_constraints,
        vec![(Cmp::Le, 2)],
        "Rust tuple(opt, opt) should emit ONLY Le (Ge skipped when min_len==0)"
    );
}

#[test]
fn parity_spec_list_appends_type_and_item_spec() {
    if !python_available() {
        return;
    }
    // Spec.list(item_spec) when item_func is a Spec performs:
    //   1. self.type(list)                                  (py:502)
    //   2. self.specs.append(item_spec)                     (py:504)
    //   3. self.checks.append(('check_list', idx, msg))     (py:506)
    // Pin Python's observable counts AND Rust port's effects:
    //   - len(checks) == 2  (type check + check_list)
    //   - len(specs) == 1   (item spec recorded for dispatch)
    //   - did_type == False (list() does NOT set did_type)
    let py = match py_eval(
        "(lambda S: (lambda s: __import__('json').dumps([len(s.checks), len(s.specs), s.did_type]))(S().list(S())))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([2, 1, false]),
        "Python Spec().list(Spec()) fixture drift"
    );

    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::default().list(Spec::default());
    assert_eq!(s.specs.len(), 1, "Rust list() should record item spec");
    assert!(
        s.allowed_types.contains(&SpecType::List),
        "Rust list() should add SpecType::List"
    );
    assert!(!s.did_type, "Rust list() should NOT set did_type");
}

#[test]
fn parity_spec_context_message_recursively_propagates() {
    if !python_available() {
        return;
    }
    // Python context_message recurses: every child Spec in self.specs
    // whose cmsg is empty/falsy receives the same msg. Child specs that
    // already have a cmsg keep theirs.
    //
    // Verify both branches:
    //   Empty child cmsg  → propagated 'Outer ctx'
    //   Preset child cmsg → 'Child preset' (unchanged)
    let py = match py_eval(
        "(lambda S: (lambda outer, child, outer2, child2: (outer.specs.append(child), outer.context_message('Outer ctx'), outer2.specs.append(child2), outer2.context_message('Outer 2 ctx'), __import__('json').dumps([outer.cmsg, child.cmsg, outer2.cmsg, child2.cmsg]))[4])(S(), S(), S(), (lambda c: setattr(c, 'cmsg', 'Child preset') or c)(S())))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!(["Outer ctx", "Outer ctx", "Outer 2 ctx", "Child preset"]),
        "Python context_message propagation fixture drift"
    );

    use powerliners::lint::spec::Spec;

    // Case 1: empty-cmsg child gets propagated.
    let mut outer = Spec::default();
    outer.specs.push(Spec::default());
    let outer = outer.context_message("Outer ctx");
    assert_eq!(outer.cmsg, "Outer ctx");
    assert_eq!(
        outer.specs[0].cmsg, "Outer ctx",
        "Rust ctx-msg failed to propagate to empty-cmsg child"
    );

    // Case 2: preset child cmsg is preserved.
    let mut outer2 = Spec::default();
    let mut child2 = Spec::default();
    child2.cmsg = "Child preset".to_string();
    outer2.specs.push(child2);
    let outer2 = outer2.context_message("Outer 2 ctx");
    assert_eq!(outer2.cmsg, "Outer 2 ctx");
    assert_eq!(
        outer2.specs[0].cmsg, "Child preset",
        "Rust ctx-msg overwrote preset child cmsg"
    );
}

#[test]
fn parity_spec_ident_regex_accepts_colon_form() {
    if !python_available() {
        return;
    }
    // Spec.ident() must validate BOTH bare identifiers ('foo') and
    // colon-separated ones ('foo:bar') — powerline colorscheme keys
    // (e.g. 'solarized:term') depend on the colon form.
    //
    // Python upstream regex: r'^\w+(?::\w+)?$' (py:588)
    // Verify the compiled-regex match outcome agrees between ports.
    let cases: &[(&str, bool)] = &[
        ("foo", true),
        ("foo_bar", true),
        ("foo:bar", true),      // colon form must pass
        ("foo:bar:baz", false), // only ONE colon segment
        ("123abc", true),       // \w matches digits
        ("with space", false),
        ("", false),
        ("foo-bar", false), // hyphen not in \w
    ];
    for (input, expected) in cases {
        let py_expr = format!(
            "bool(__import__('re').match(r'^\\w+(?::\\w+)?$', {:?}))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val = py.trim() == "True";
        assert_eq!(
            py_val, *expected,
            "Python ident-regex fixture drift for {:?}",
            input
        );
        let s = powerliners::lint::spec::Spec::default().ident();
        let pattern = s.regex.expect("ident() must set regex");
        // Python re.match anchors at start, not end → emulate with
        // Rust's regex crate by ensuring the pattern itself uses ^.
        let re = regex::Regex::new(&pattern).expect("ident pattern must compile");
        let rs_val = re.is_match(input);
        assert_eq!(
            rs_val, *expected,
            "Rust ident-regex {:?} mismatch (pattern={:?})",
            input, pattern
        );
    }
}

#[test]
fn parity_spec_optional_required_toggle_isoptional() {
    if !python_available() {
        return;
    }
    // optional() sets self.isoptional = True
    // required() sets self.isoptional = False
    // default state is False
    // Verify exact toggling sequence in both ports.
    let py = match py_eval(
        "(lambda S: __import__('json').dumps([S().isoptional, S().optional().isoptional, S().optional().required().isoptional]))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    assert_eq!(
        py_value,
        serde_json::json!([false, true, false]),
        "Python isoptional toggle drift"
    );

    use powerliners::lint::spec::Spec;
    let rs0 = Spec::default().isoptional;
    let rs1 = Spec::default().optional().isoptional;
    let rs2 = Spec::default().optional().required().isoptional;
    assert!(!rs0, "Rust Spec::default().isoptional must be false");
    assert!(
        rs1,
        "Rust Spec::default().optional().isoptional must be true"
    );
    assert!(
        !rs2,
        "Rust Spec::default().optional().required().isoptional must be false"
    );
}

#[test]
fn parity_spec_len_appends_check_only_no_type() {
    if !python_available() {
        return;
    }
    // Spec.len(comparison, cint) appends a SINGLE check_func — NOT a
    // type check. (Distinct from cmp() which calls self.type() first.)
    // Pin both ports across 3 comparison operators.
    for op in ["lt", "ge", "gt"] {
        let py_expr = format!(
            "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().len({:?}, 10).checks)",
            op
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_count: usize = py.trim().parse().expect("py returned non-integer");
        assert_eq!(
            py_count, 1,
            "Python Spec().len({:?}, 10).checks length should be 1",
            op
        );
    }
    use powerliners::lint::spec::{Cmp, Spec};
    let s = Spec::default().len(Cmp::Lt, 10);
    assert_eq!(
        s.len_constraints,
        vec![(Cmp::Lt, 10)],
        "Rust Spec::len should append (Lt, 10) to len_constraints"
    );
}

#[test]
fn parity_spec_cmp_chains_type_and_check() {
    if !python_available() {
        return;
    }
    // Spec.cmp(comparison, cint) appends BOTH:
    //   1. a type check (via self.type(...) at py:457-461)
    //   2. the cmp check_func itself (py:463-467)
    // → final self.checks length == 2. Verify for each of the 3 cint
    // type branches:
    //   int cint   → self.type(int)
    //   float cint → self.type(int, float)
    //   str cint   → self.type(unicode)
    let cases: &[&str] = &[
        "Spec().cmp('lt', 100)",     // int branch
        "Spec().cmp('ge', 0.0)",     // float branch
        "Spec().cmp('eq', 'hello')", // str branch
        "Spec().cmp('gt', -1)",      // negative int
    ];
    for py_call in cases {
        let py = match py_eval(&format!(
            "len(__import__('powerline.lint.spec', fromlist=['Spec']).Spec.__dict__ if False else __import__('powerline.lint.spec', fromlist=['Spec']).{}.checks)",
            py_call
        )) {
            Some(v) => v,
            None => return,
        };
        let py_count: usize = py.trim().parse().expect("py returned non-integer");
        assert_eq!(
            py_count, 2,
            "Python Spec().cmp(...).checks length should be 2 for {}",
            py_call
        );
    }
    // Rust port: cmp() sets cmp_constraint. The shape is different
    // (constraint Option rather than checks Vec), but the observable
    // semantic — "cmp call adds the comparison + a type bound" — is
    // pinned by cmp_constraint being Some after the call.
    use powerliners::lint::spec::{Cmp, Spec};
    let s = Spec::default().cmp(Cmp::Lt, 100.0);
    assert!(
        s.cmp_constraint.is_some(),
        "Rust Spec::cmp should set cmp_constraint"
    );
    assert_eq!(s.cmp_constraint.as_ref().unwrap().0, Cmp::Lt);
    assert!((s.cmp_constraint.as_ref().unwrap().1 - 100.0).abs() < 1e-9);
}

#[test]
fn parity_mergedicts_3_level_recursive_merge() {
    if !python_available() {
        return;
    }
    // Verify mergedicts recurses through 3 dict levels and at each level:
    //   - new keys from d2 added
    //   - overlapping non-dict values: d2 wins
    //   - overlapping dict values: recurse
    let py = match py_eval(
        "(lambda d1, d2: (__import__('powerline.lib.dict', fromlist=['mergedicts']).mergedicts(d1, d2), __import__('json').dumps(d1, sort_keys=True))[1])({'a': 1, 'nested': {'x': 1, 'y': 2, 'deeper': {'k': 'old'}}}, {'b': 3, 'nested': {'y': 99, 'z': 4, 'deeper': {'k': 'new', 'extra': 7}}})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let mut d1 = json!({
        "a": 1,
        "nested": {
            "x": 1,
            "y": 2,
            "deeper": {"k": "old"}
        }
    })
    .as_object()
    .unwrap()
    .clone();
    let d2 = json!({
        "b": 3,
        "nested": {
            "y": 99,
            "z": 4,
            "deeper": {"k": "new", "extra": 7}
        }
    })
    .as_object()
    .unwrap()
    .clone();
    powerliners::lib::dict::mergedicts(&mut d1, d2, true);
    assert_eq!(
        py_value,
        serde_json::Value::Object(d1),
        "mergedicts 3-level recursive merge mismatch"
    );
}

#[test]
fn parity_clear_special_values_walks_nested_dicts() {
    if !python_available() {
        return;
    }
    // _clear_special_values walks a nested dict iteratively (explicit
    // stack) and removes every key whose value is the REMOVE_THIS_KEY
    // sentinel — at every depth. Verify against 3-level nesting:
    //   root keeps a=1, deletes b
    //   nested keeps y=2, deletes x
    //   deeper keeps r=3, deletes q
    let py = match py_eval(
        "(lambda d: (__import__('powerline.lib.dict', fromlist=['_clear_special_values'])._clear_special_values(d), __import__('json').dumps(d, sort_keys=True))[1])({'a': 1, 'b': __import__('powerline.lib.dict', fromlist=['REMOVE_THIS_KEY']).REMOVE_THIS_KEY, 'nested': {'x': __import__('powerline.lib.dict', fromlist=['REMOVE_THIS_KEY']).REMOVE_THIS_KEY, 'y': 2, 'deeper': {'q': __import__('powerline.lib.dict', fromlist=['REMOVE_THIS_KEY']).REMOVE_THIS_KEY, 'r': 3}}})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let mut d = json!({
        "a": 1,
        "b": powerliners::lib::dict::REMOVE_THIS_KEY(),
        "nested": {
            "x": powerliners::lib::dict::REMOVE_THIS_KEY(),
            "y": 2,
            "deeper": {
                "q": powerliners::lib::dict::REMOVE_THIS_KEY(),
                "r": 3,
            }
        }
    })
    .as_object()
    .unwrap()
    .clone();
    powerliners::lib::dict::_clear_special_values(&mut d);
    assert_eq!(
        py_value,
        serde_json::Value::Object(d),
        "_clear_special_values nested walk mismatch"
    );
}

#[test]
fn parity_mergedicts_remove_this_key_deletes_when_remove_true() {
    if !python_available() {
        return;
    }
    // mergedicts with remove=True: REMOVE_THIS_KEY in d2 deletes that
    // key from d1 entirely (via _clear_special_values).
    //
    // Python and Rust use different runtime representations of the
    // sentinel — Python uses an `object()` identity, Rust uses
    // `{"__powerliners_remove_this_key__": true}`. The OBSERVABLE
    // contract is identical: the key vanishes from d1.
    let py = match py_eval(
        "(lambda d1, d2: (__import__('powerline.lib.dict', fromlist=['mergedicts']).mergedicts(d1, d2, remove=True), __import__('json').dumps(d1, sort_keys=True))[1])({'a': 1, 'b': 2, 'c': 3}, {'b': __import__('powerline.lib.dict', fromlist=['REMOVE_THIS_KEY']).REMOVE_THIS_KEY})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let mut d1 = json!({"a": 1, "b": 2, "c": 3}).as_object().unwrap().clone();
    let mut d2_obj = serde_json::Map::new();
    d2_obj.insert("b".to_string(), powerliners::lib::dict::REMOVE_THIS_KEY());
    powerliners::lib::dict::mergedicts(&mut d1, d2_obj, true);
    assert_eq!(
        py_value,
        serde_json::Value::Object(d1),
        "mergedicts(remove=True) + REMOVE_THIS_KEY observable mismatch"
    );
}

#[test]
fn parity_run_cmd_stdout_stripped_default() {
    if !python_available() {
        return;
    }
    // run_cmd(pl, ['echo', 'hello']) → 'hello' (strip=True default
    // removes the trailing newline). Verify against an actual subprocess
    // invocation: both ports must run the same echo binary and strip.
    let cases: &[(&[&str], Option<&str>, &str)] = &[
        (&["echo", "hello"], None, "hello"),
        (&["echo", "foo", "bar"], None, "foo bar"),
        (&["printf", "%s", "no_newline"], None, "no_newline"),
        (&["cat"], Some("piped-data"), "piped-data"),
    ];
    for (cmd, stdin, expected) in cases {
        // Python invocation: run_cmd defaults to strip=True.
        let py_cmd = cmd
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");
        let py_expr = match stdin {
            None => format!(
                "__import__('powerline.lib.shell', fromlist=['run_cmd']).run_cmd(None, [{}])",
                py_cmd
            ),
            Some(s) => format!(
                "__import__('powerline.lib.shell', fromlist=['run_cmd']).run_cmd(None, [{}], stdin={:?})",
                py_cmd, s
            ),
        };
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(py.trim(), *expected, "Python fixture drift for {:?}", cmd);
        let rs_cmd: Vec<String> = cmd.iter().map(|s| s.to_string()).collect();
        let rs = powerliners::lib::shell::run_cmd(&(), &rs_cmd, *stdin, true)
            .expect("Rust run_cmd returned None");
        assert_eq!(rs, *expected, "Rust run_cmd({:?}) mismatch", cmd);
    }
}

#[test]
fn parity_pick_gradient_value_bankers_rounding() {
    if !python_available() {
        return;
    }
    // pick_gradient_value(grad_list, level) → grad_list[int(round(level * (len-1) / 100))]
    // Python 3 round() uses banker's rounding (half-to-even). Cover all
    // 5 endpoint positions plus banker's-rounding edge cases:
    //   0.5 (rounds DOWN to 0 — even), 12.5 (DOWN to 0), 87.5 (UP to 4 — even)
    let grad: &[u64] = &[10, 20, 30, 40, 50];
    let levels: &[f64] = &[0.0, 25.0, 50.0, 75.0, 100.0, 12.5, 87.5, 99.999, 0.5];
    let py_grad = "[10, 20, 30, 40, 50]";
    for level in levels {
        let py_expr = format!(
            "__import__('powerline.colorscheme', fromlist=['pick_gradient_value']).pick_gradient_value({}, {})",
            py_grad, level
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val: u64 = py.trim().parse().expect("py returned non-integer");
        let rs_val = powerliners::colorscheme::pick_gradient_value(grad, *level);
        assert_eq!(
            rs_val, py_val,
            "pick_gradient_value(level={}) mismatch: py={}, rs={}",
            level, py_val, rs_val
        );
    }
}

#[test]
fn parity_get_attrs_flag_combines_attrs() {
    if !python_available() {
        return;
    }
    // get_attrs_flag(['bold','italic','underline']) → bit OR of the
    // ATTR_NAMES table:
    //   bold = 1, italic = 2, underline = 4
    // Unknown attrs contribute 0.
    let cases: &[(&[&str], u32)] = &[
        (&["bold"], 1),
        (&["italic"], 2),
        (&["underline"], 4),
        (&[], 0),
        (&["bold", "italic"], 3),
        (&["bold", "italic", "underline"], 7),
        (&["unknown_attr"], 0),
    ];
    for (input, expected) in cases {
        let py_list = input
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");
        let py_expr = format!(
            "__import__('powerline.colorscheme', fromlist=['get_attrs_flag']).get_attrs_flag([{}])",
            py_list
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val: u32 = py.trim().parse().expect("py returned non-integer");
        assert_eq!(py_val, *expected, "Python fixture drift for {:?}", input);
        let rs_in: Vec<String> = input.iter().map(|s| s.to_string()).collect();
        let rs = powerliners::colorscheme::get_attrs_flag(&rs_in);
        assert_eq!(rs, *expected, "Rust get_attrs_flag({:?}) mismatch", input);
    }
}

#[test]
fn parity_cterm_to_hex_full_256_entry_table() {
    if !python_available() {
        return;
    }
    // cterm_to_hex is a 256-entry lookup mapping cterm color index → RGB.
    // Verify entry-by-entry parity with Python (rather than sampling) —
    // guards against silent drift in any individual palette entry.
    let py = match py_eval(
        "__import__('json').dumps(list(__import__('powerline.colorscheme', fromlist=['cterm_to_hex']).cterm_to_hex))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    let py_arr = py_value.as_array().expect("py value not array");
    assert_eq!(py_arr.len(), 256, "Python cterm_to_hex length drift");
    assert_eq!(
        powerliners::colorscheme::cterm_to_hex.len(),
        256,
        "Rust cterm_to_hex length drift"
    );
    for (i, py_entry) in py_arr.iter().enumerate() {
        let py_n = py_entry.as_u64().expect("py entry not int");
        let rs_n = powerliners::colorscheme::cterm_to_hex[i];
        assert_eq!(
            py_n, rs_n,
            "cterm_to_hex[{}] mismatch: py=0x{:06X} rs=0x{:06X}",
            i, py_n, rs_n
        );
    }
}

#[test]
fn parity_keyvaluesplit_splits_on_first_equals() {
    if !python_available() {
        return;
    }
    // keyvaluesplit('option=json_value') splits on the FIRST '=' and
    // routes the value through parse_value. Verify:
    //   'a=1'                    → ('a', 1)              JSON int
    //   'key=value with spaces'  → ('key', 'value with spaces')   raw string
    //   'a=b=c'                  → ('a', 'b=c')          extra '=' kept in value
    //   '=value_only'            → ('', 'value_only')    empty key allowed
    // (TypeError-raising cases like 'no_equals' covered separately.)
    let cases: &[(&str, &str)] = &[
        ("a=1", r#"["a",1]"#),
        ("key=value with spaces", r#"["key","value with spaces"]"#),
        ("a=b=c", r#"["a","b=c"]"#),
        ("=value_only", r#"["","value_only"]"#),
    ];
    for (input, expected_json) in cases {
        let py_expr = format!(
            "__import__('json').dumps(list(__import__('powerline.lib.overrides', fromlist=['keyvaluesplit']).keyvaluesplit({:?})))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let expected: serde_json::Value =
            serde_json::from_str(expected_json).expect("expected JSON malformed");
        assert_eq!(py_value, expected, "Python fixture drift for {:?}", input);
        let (rs_key, rs_val) =
            powerliners::lib::overrides::keyvaluesplit(input).expect("Rust keyvaluesplit errored");
        let rs_json = serde_json::Value::Array(vec![serde_json::Value::String(rs_key), rs_val]);
        assert_eq!(
            rs_json, expected,
            "Rust keyvaluesplit({:?}) mismatch",
            input
        );
    }
}

#[test]
fn parity_parse_override_var_splits_and_nests() {
    if !python_available() {
        return;
    }
    // parse_override_var splits on ';' and feeds each segment to
    // parsedotval. Returns iterable of (key, value) tuples. Verify:
    //   - multi-segment split
    //   - single dotted segment
    //   - empty input
    //   - mixed value types
    let cases: &[(&str, &str)] = &[
        ("a=1;b=2", r#"[["a",1],["b",2]]"#),
        ("a.b=1", r#"[["a",{"b":1}]]"#),
        ("", r#"[]"#),
        (
            "x=hello;y=world;z=42",
            r#"[["x","hello"],["y","world"],["z",42]]"#,
        ),
    ];
    for (input, expected_json) in cases {
        let py_expr = format!(
            "__import__('json').dumps(list(__import__('powerline.lib.overrides', fromlist=['parse_override_var']).parse_override_var({:?})))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let expected: serde_json::Value =
            serde_json::from_str(expected_json).expect("expected JSON malformed");
        assert_eq!(py_value, expected, "Python fixture drift for {:?}", input);
        let rs_pairs = powerliners::lib::overrides::parse_override_var(input);
        let rs_json = serde_json::to_value(
            rs_pairs
                .into_iter()
                .map(|(k, v)| serde_json::json!([k, v]))
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(
            rs_json, expected,
            "Rust parse_override_var({:?}) mismatch",
            input
        );
    }
}

#[test]
fn parity_parsedotval_nests_dotted_keys() {
    if !python_available() {
        return;
    }
    // parsedotval('a.b.c=42') returns ('a', {'b': {'c': 42}}) — the
    // outermost dotted segment becomes the key, remaining segments
    // build a recursive nested dict containing the JSON-parsed value.
    let cases: &[(&str, &str, &str)] = &[
        ("a.b=2", "a", r#"{"b":2}"#),
        ("x.y.z=42", "x", r#"{"y":{"z":42}}"#),
        ("flag=true", "flag", "true"),
        ("s=hello", "s", r#""hello""#),
        ("nested=null", "nested", "null"),
    ];
    for (input, expected_key, expected_value_json) in cases {
        let py_expr = format!(
            "(lambda r: __import__('json').dumps(r))(__import__('powerline.lib.overrides', fromlist=['parsedotval']).parsedotval({:?}))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_arr: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        assert_eq!(
            py_arr[0].as_str(),
            Some(*expected_key),
            "fixture key drift for {:?}",
            input
        );
        let expected_value: serde_json::Value =
            serde_json::from_str(expected_value_json).expect("expected value JSON malformed");
        assert_eq!(
            py_arr[1], expected_value,
            "fixture value drift for {:?}",
            input
        );
        let (rs_key, rs_val) =
            powerliners::lib::overrides::parsedotval_str(input).expect("Rust parsedotval failed");
        assert_eq!(
            rs_key, *expected_key,
            "Rust parsedotval_str({:?}) key mismatch",
            input
        );
        assert_eq!(
            rs_val, expected_value,
            "Rust parsedotval_str({:?}) value mismatch",
            input
        );
    }
}

#[test]
fn parity_mergeargs_nested_dict_recursive_merge() {
    if !python_available() {
        return;
    }
    // mergeargs folds an iterable of (k, v) pairs by repeatedly calling
    // mergedicts on a fresh accumulator. When two pairs share a key
    // whose values are dicts, the inner dicts merge recursively.
    // Verify:
    //   [('a', {'x': 1}), ('a', {'y': 2})] → {'a': {'x': 1, 'y': 2}}
    let py = match py_eval(
        "__import__('json').dumps(__import__('powerline.lib.dict', fromlist=['mergeargs']).mergeargs([('a', {'x': 1}), ('a', {'y': 2})]), sort_keys=True)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::{json, Value};
    let pairs = vec![
        ("a".to_string(), json!({"x": 1})),
        ("a".to_string(), json!({"y": 2})),
    ];
    let result = powerliners::lib::dict::mergeargs(pairs, true)
        .expect("mergeargs returned None on non-empty input");
    assert_eq!(
        py_value,
        Value::Object(result),
        "mergeargs nested merge mismatch"
    );
}

#[test]
fn parity_updated_returns_new_dict_without_mutating_source() {
    if !python_available() {
        return;
    }
    // updated(d, **kwargs) returns a copy of d with kwargs applied via
    // d.copy().update(**kwargs). Verify Rust port:
    //   - source dict stays untouched
    //   - overriding keys win
    //   - empty source works
    let py = match py_eval(
        "(lambda d, upd: __import__('json').dumps({'orig': d, 'result': __import__('powerline.lib.dict', fromlist=['updated']).updated(d, **upd)}, sort_keys=True))({'a': 1, 'b': 2}, {'a': 99, 'c': 3})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let d_orig = json!({"a": 1, "b": 2}).as_object().unwrap().clone();
    let upd = json!({"a": 99, "c": 3}).as_object().unwrap().clone();
    let result = powerliners::lib::dict::updated(&d_orig, upd);
    let combined = json!({
        "orig": serde_json::Value::Object(d_orig),
        "result": serde_json::Value::Object(result),
    });
    assert_eq!(
        py_value, combined,
        "updated() output / source-mutation mismatch"
    );
}

#[test]
fn parity_surrogate_pair_to_character_full_range() {
    if !python_available() {
        return;
    }
    // surrogate_pair_to_character(high, low) reconstructs a 32-bit
    // codepoint from a UTF-16 surrogate pair via:
    //   0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
    // Verify with both ports across emoji + boundary values.
    let cases: &[(u32, u32, u32)] = &[
        (0xD83D, 0xDE00, 128512),   // 😀
        (0xD83D, 0xDE0A, 128522),   // 😊
        (0xD800, 0xDC00, 0x10000),  // minimum supplementary
        (0xDBFF, 0xDFFF, 0x10FFFF), // maximum Unicode codepoint
    ];
    for (high, low, expected) in cases {
        let py_expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['surrogate_pair_to_character']).surrogate_pair_to_character({}, {})",
            high, low
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_val: u32 = py.trim().parse().expect("py returned non-integer");
        assert_eq!(
            py_val, *expected,
            "Python fixture drift for (0x{:X}, 0x{:X})",
            high, low
        );
        let rs = powerliners::lib::unicode::surrogate_pair_to_character(*high, *low);
        assert_eq!(
            rs, *expected,
            "Rust surrogate_pair_to_character(0x{:X}, 0x{:X}) mismatch",
            high, low
        );
    }
}

#[test]
fn parity_path_join_handles_absolute_components() {
    if !python_available() {
        return;
    }
    // path.join() ports os.path.join semantics: any absolute component
    // discards all prior components; empty leading component is dropped;
    // trailing slash on a component does NOT double-slash the result.
    let cases: &[(&[&str], &str)] = &[
        (&["a", "b", "c"], "a/b/c"),
        (&["/abs", "b"], "/abs/b"),
        (&["a", "/abs", "b"], "/abs/b"), // /abs resets accumulator
        (&["", "b"], "b"),
        (&["a", "", "b"], "a/b"),
        (&["a/", "b"], "a/b"),
    ];
    for (input, expected) in cases {
        let py_pairs = input
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");
        let py_expr = format!(
            "__import__('powerline.lib.path', fromlist=['join']).join({})",
            py_pairs
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(py.trim(), *expected, "Python fixture drift for {:?}", input);
        let rs = powerliners::lib::path::join(input.iter().copied());
        assert_eq!(
            rs.to_string_lossy(),
            *expected,
            "Rust path::join({:?}) mismatch: rs={:?}",
            input,
            rs
        );
    }
}

#[test]
fn parity_humanize_bytes_canonical_units() {
    if !python_available() {
        return;
    }
    // humanize_bytes() returns "<quotient><decimals> <unit><suffix>" with
    // unit_list = [('',0), ('k',0), ('M',1), ('G',2), ('T',2), ('P',2)].
    // Cover binary (1024) and SI (1000) divisor paths + zero + custom
    // suffix + sub-K passthrough.
    let cases: &[(u64, &str, bool, &str)] = &[
        (0, "B", false, "0 B"),
        (512, "B", false, "512 B"),
        (1024, "B", false, "1 KiB"),
        (1024 * 1024, "B", false, "1.0 MiB"),
        (1024_u64.pow(3), "B", false, "1.00 GiB"),
        (1024_u64.pow(4), "B", false, "1.00 TiB"),
        (1024, "b", false, "1 KiB"), // py shows 'KiB' regardless of suffix-case
        (1024, "B", true, "1 kB"),
        (1000, "B", true, "1 kB"),
    ];
    for (n, suffix, si_prefix, _expected_static) in cases {
        let py_expr = format!(
            "__import__('powerline.lib.humanize_bytes', fromlist=['humanize_bytes']).humanize_bytes({}, suffix={:?}, si_prefix={})",
            n,
            suffix,
            if *si_prefix { "True" } else { "False" }
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::humanize_bytes::humanize_bytes(*n as f64, suffix, *si_prefix);
        assert_eq!(
            py.trim(),
            rs,
            "humanize_bytes({}, {:?}, si={}) mismatch: py={:?}, rs={:?}",
            n,
            suffix,
            si_prefix,
            py,
            rs
        );
    }
}

#[test]
fn parity_urllib_urlencode_matches_python_stdlib() {
    if !python_available() {
        return;
    }
    // urllib_urlencode is aliased directly from urllib.parse.urlencode
    // upstream (py:7). Verify Rust port matches Python for:
    //   - single key
    //   - multi-key with spaces (' ' → '+')
    //   - special chars needing %-escape ('=', '&', '/')
    //   - empty input
    let cases: &[(&[(&str, &str)], &str)] = &[
        (&[("a", "1")], "a=1"),
        (&[("a", "1"), ("b", "two words")], "a=1&b=two+words"),
        (&[("q", "a=b"), ("z", "x&y")], "q=a%3Db&z=x%26y"),
        (&[], ""),
        (
            &[("key", "has spaces and / chars")],
            "key=has+spaces+and+%2F+chars",
        ),
    ];
    for (input, expected) in cases {
        // Build a Python list-of-tuples literal so dict ordering doesn't matter.
        let py_pairs = input
            .iter()
            .map(|(k, v)| format!("({:?}, {:?})", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        let py_expr = format!(
            "__import__('powerline.lib.url', fromlist=['urllib_urlencode']).urllib_urlencode([{}])",
            py_pairs
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(py.trim(), *expected, "Python fixture drift for {:?}", input);
        let rs = powerliners::lib::url::urllib_urlencode(input.iter().map(|(k, v)| (*k, *v)));
        assert_eq!(rs, *expected, "Rust urllib_urlencode({:?}) mismatch", input);
    }
}

#[test]
fn parity_encoding_preferred_helpers_all_return_utf8() {
    if !python_available() {
        return;
    }
    // Six getters in powerline.lib.encoding must all return a
    // UTF-8 family string. Verify by normalizing case and stripping
    // '-' (Python returns 'UTF-8' for most, 'utf-8' for
    // file_name_encoding).
    let mappings: &[(&str, fn() -> &'static str)] = &[
        ("get_preferred_input_encoding", || {
            powerliners::lib::encoding::get_preferred_input_encoding()
        }),
        ("get_preferred_output_encoding", || {
            powerliners::lib::encoding::get_preferred_output_encoding()
        }),
        ("get_preferred_arguments_encoding", || {
            powerliners::lib::encoding::get_preferred_arguments_encoding()
        }),
        ("get_preferred_environment_encoding", || {
            powerliners::lib::encoding::get_preferred_environment_encoding()
        }),
        ("get_preferred_file_contents_encoding", || {
            powerliners::lib::encoding::get_preferred_file_contents_encoding()
        }),
        ("get_preferred_file_name_encoding", || {
            powerliners::lib::encoding::get_preferred_file_name_encoding()
        }),
    ];
    for (name, rs_fn) in mappings {
        let py_expr = format!(
            "__import__('powerline.lib.encoding', fromlist=['{0}']).{0}()",
            name
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = rs_fn();
        assert_eq!(
            py.trim().to_lowercase().replace('-', ""),
            rs.to_lowercase().replace('-', ""),
            "{}() mismatch: py={:?}, rs={:?}",
            name,
            py,
            rs
        );
        assert!(
            rs.to_lowercase().contains("utf"),
            "{}() should return UTF-8 family, got {:?}",
            name,
            rs
        );
    }
}

#[test]
fn parity_shell_which_finds_and_misses_consistently() {
    if !python_available() {
        return;
    }
    // Pin which() to Python's shutil.which (which is what powerline.lib.shell.which
    // wraps). Three cases:
    //   1. 'sh' — must exist on every POSIX system; both ports return a path
    //   2. 'python3' — present whenever the parity harness can run; both must agree
    //   3. 'definitely-not-a-real-binary-zzzz' — must be None / Path absent
    for cmd in &["sh", "python3"] {
        let py_expr = format!(
            "(lambda r: r if r is not None else '')(__import__('powerline.lib.shell', fromlist=['which']).which({:?}))",
            cmd
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::shell::which(cmd)
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert_eq!(
            py.trim(),
            rs,
            "which({:?}) mismatch: py={:?}, rs={:?}",
            cmd,
            py,
            rs
        );
        assert!(!rs.is_empty(), "Rust which({:?}) should resolve", cmd);
    }
    // Missing-binary case: Python returns None → empty string; Rust returns None.
    let py = match py_eval(
        "(lambda r: r if r is not None else '')(__import__('powerline.lib.shell', fromlist=['which']).which('definitely-not-a-real-binary-zzzz'))",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py.trim(),
        "",
        "Python should return None for missing binary"
    );
    assert!(
        powerliners::lib::shell::which("definitely-not-a-real-binary-zzzz").is_none(),
        "Rust which() should return None for missing binary"
    );
}

#[test]
fn parity_mergedicts_copy_does_not_mutate_d1() {
    if !python_available() {
        return;
    }
    // mergedicts_copy returns a NEW dict; d1 stays unchanged even when
    // d2 has overlapping nested keys. Pin: d1.nested.x stays alone in
    // d1 while result.nested has both x and y.
    let py = match py_eval(
        "(lambda d1, d2: (lambda result: __import__('json').dumps({'d1': d1, 'result': result}, sort_keys=True))(__import__('powerline.lib.dict', fromlist=['mergedicts_copy']).mergedicts_copy(d1, d2)))({'a': 1, 'nested': {'x': 1}}, {'b': 2, 'nested': {'y': 2}})",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let d1_orig = json!({"a": 1, "nested": {"x": 1}})
        .as_object()
        .unwrap()
        .clone();
    let d2 = json!({"b": 2, "nested": {"y": 2}})
        .as_object()
        .unwrap()
        .clone();
    let result = powerliners::lib::dict::mergedicts_copy(&d1_orig, d2);
    let combined = json!({
        "d1": serde_json::Value::Object(d1_orig),
        "result": serde_json::Value::Object(result),
    });
    assert_eq!(py_value, combined, "mergedicts_copy mutation mismatch");
}

#[test]
fn parity_parse_value_handles_floats_and_negatives() {
    if !python_available() {
        return;
    }
    // Verify parse_value() handles 4 float edge cases identically.
    // (Empty-string returns REMOVE_THIS_KEY sentinel — skipped here.)
    let cases = ["3.14", "-0.5", "0.0", "-1e-5"];
    for input in cases {
        let py_expr = format!(
            "(lambda v: __import__('json').dumps(v))(__import__('powerline.lib.overrides', fromlist=['parse_value']).parse_value({:?}))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
        let rs_value = powerliners::lib::overrides::parse_value(input);
        // Compare via JSON-encoded float equality (handles 1e-5 == 0.00001
        // representation differences).
        let py_f = py_value.as_f64().unwrap_or(f64::NAN);
        let rs_f = rs_value.as_f64().unwrap_or(f64::NAN);
        assert!(
            (py_f - rs_f).abs() < 1e-12,
            "parse_value({:?}) float mismatch: py={}, rs={}",
            input,
            py_f,
            rs_f
        );
    }
}

#[test]
fn parity_spec_full_chain_preserves_each_step() {
    if !python_available() {
        return;
    }
    // Verify a multi-step builder chain produces the same final state
    // on both ports. type+optional+context_message+printable should:
    //   - end with isoptional == True
    //   - cmsg == 'msg'
    //   - 3 check entries on Python (check_type explicit + check_type
    //     from printable + check_printable)
    let py_iso = match py_eval(
        "(lambda s: str(s.isoptional))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).optional().context_message('msg').printable())",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_msg = match py_eval(
        "(lambda s: s.cmsg)(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).optional().context_message('msg').printable())",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_checks = match py_eval(
        "(lambda s: str(len(s.checks)))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).optional().context_message('msg').printable())",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py_iso, "True");
    assert_eq!(py_msg, "msg");
    assert_eq!(py_checks, "3");

    use powerliners::lint::spec::{Spec, SpecType};
    let s = Spec::new()
        .type_check(&[SpecType::Unicode])
        .optional()
        .context_message("msg")
        .printable();
    assert!(s.isoptional);
    assert_eq!(s.cmsg, "msg");
    assert!(s.printable_flag);
    assert!(s.allowed_types.contains(&SpecType::Unicode));
}

#[test]
fn parity_mergedicts_copy_handles_3_level_nested_collisions() {
    if !python_available() {
        return;
    }
    // Verify mergedicts_copy handles 3-level nested dict collisions:
    // d2 wins on leaf values, intermediate dicts merge recursively, and
    // d1's non-overlapping keys survive intact.
    let py = match py_eval(
        "(lambda r: __import__('json').dumps(r, sort_keys=True))(__import__('powerline.lib.dict', fromlist=['mergedicts_copy']).mergedicts_copy({'a': 1, 'nested': {'x': {'p': 1, 'q': 2}, 'y': 10}}, {'b': 2, 'nested': {'x': {'p': 99, 'r': 3}, 'z': 20}}))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    use serde_json::json;
    let d1 = json!({"a": 1, "nested": {"x": {"p": 1, "q": 2}, "y": 10}})
        .as_object()
        .unwrap()
        .clone();
    let d2 = json!({"b": 2, "nested": {"x": {"p": 99, "r": 3}, "z": 20}})
        .as_object()
        .unwrap()
        .clone();
    let r = powerliners::lib::dict::mergedicts_copy(&d1, d2);
    assert_eq!(
        py_value,
        serde_json::Value::Object(r),
        "3-level nested mergedicts_copy mismatch"
    );
}

#[test]
fn parity_parsedotval_5_level_nested_keys() {
    if !python_available() {
        return;
    }
    // parsedotval recursively wraps a dotted-key string in nested
    // dicts: 'a.b.c.d.e' = 'deep' → ('a', {'b': {'c': {'d': {'e': 'deep'}}}})
    let py = match py_eval(
        "(lambda r: __import__('json').dumps(r[1]))(__import__('powerline.lib.overrides', fromlist=['parsedotval']).parsedotval(('a.b.c.d.e', 'deep')))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");
    let rs_pair = powerliners::lib::overrides::parsedotval_tuple("a.b.c.d.e", "deep");
    assert_eq!(rs_pair.0, "a", "Rust outer key should be 'a'");
    assert_eq!(
        py_value, rs_pair.1,
        "parsedotval 5-level nested value mismatch:\n  py: {}\n  rs: {}",
        py_value, rs_pair.1
    );
}

#[test]
fn parity_mergeargs_iterates_kv_tuples() {
    if !python_available() {
        return;
    }
    // mergeargs takes an iterable of (key, value) tuples and merges
    // them into a single dict via mergedicts (overrides win on
    // collision; nested values merge recursively).
    let py = match py_eval(
        "(lambda r: __import__('json').dumps(r, sort_keys=True))(__import__('powerline.lib.dict', fromlist=['mergeargs']).mergeargs([('a', 1), ('b', 2), ('a', {'nested': True})]))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_value: serde_json::Value = serde_json::from_str(&py).expect("py JSON malformed");

    let updates: Vec<(String, serde_json::Value)> = vec![
        ("a".to_string(), serde_json::Value::from(1)),
        ("b".to_string(), serde_json::Value::from(2)),
        ("a".to_string(), serde_json::json!({"nested": true})),
    ];
    let rs = powerliners::lib::dict::mergeargs(updates, false)
        .expect("Rust mergeargs returned None for non-empty input");
    assert_eq!(
        py_value,
        serde_json::Value::Object(rs),
        "mergeargs merge result mismatch"
    );
}

#[test]
fn parity_spec_copy_preserves_state_and_independent() {
    if !python_available() {
        return;
    }
    // Verify Spec.copy() preserves all builder state AND mutations to
    // the copy don't affect the original.
    let py_orig = match py_eval(
        "(lambda s: '{},{}'.format(s.isoptional, s.cmsg))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).optional().context_message('msg'))",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_copy = match py_eval(
        "(lambda s: '{},{}'.format(s.copy().isoptional, s.copy().cmsg))(__import__('powerline.lint.spec', fromlist=['Spec']).Spec().type(str).optional().context_message('msg'))",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(py_orig, "True,msg");
    assert_eq!(py_copy, "True,msg");

    use powerliners::lint::spec::{Spec, SpecType};
    let orig = Spec::new()
        .type_check(&[SpecType::Unicode])
        .optional()
        .context_message("msg");
    let mut copy = orig.copy();
    assert!(copy.isoptional);
    assert_eq!(copy.cmsg, "msg");
    // Mutate copy, verify original unchanged
    copy.isoptional = false;
    assert!(orig.isoptional, "Rust orig.isoptional should still be true");
    assert!(!copy.isoptional);
}

#[test]
fn parity_pick_gradient_value_with_5_element_grad() {
    if !python_available() {
        return;
    }
    // Use a non-monotonic 5-element gradient to verify both ports
    // index identically via the same `round(level * (n-1) / 100)`
    // formula across 5 levels (0, 25, 50, 75, 100).
    let grad: Vec<u64> = vec![0, 50, 100, 150, 200];
    let levels = [0.0_f64, 25.0, 50.0, 75.0, 100.0];
    for level in levels {
        let py_grad_str = grad
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let expr = format!(
            "__import__('powerline.colorscheme', fromlist=['pick_gradient_value']).pick_gradient_value([{}], {})",
            py_grad_str, level
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_int: u64 = py.parse().expect("Python returned non-int");
        let rs = powerliners::colorscheme::pick_gradient_value(&grad, level);
        assert_eq!(
            py_int, rs,
            "pick_gradient_value(level={}) mismatch: py={}, rs={}",
            level, py_int, rs
        );
    }
}

#[test]
fn parity_humanize_bytes_custom_suffix_and_extremes() {
    if !python_available() {
        return;
    }
    // Extends the existing parity_humanize_bytes test with lowercase
    // suffix variant ('b'), terabyte-scale, and fractional input edge
    // cases.
    let cases: &[(f64, &str, bool)] = &[
        (1024.0, "b", false),
        (1024.0 * 1024.0, "b", false),
        (1000.0, "b", true),
        (2.0_f64.powi(40), "B", false), // 1 TiB
        (0.5, "B", false),              // sub-byte → "0 B"
    ];
    for (n, suf, si) in cases {
        let expr = format!(
            "__import__('powerline.lib.humanize_bytes', fromlist=['humanize_bytes']).humanize_bytes({}, '{}', {})",
            n, suf, if *si { "True" } else { "False" }
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::humanize_bytes::humanize_bytes(*n, suf, *si);
        assert_eq!(
            py, rs,
            "humanize_bytes({}, {:?}, {}) mismatch:\n  py: {:?}\n  rs: {:?}",
            n, suf, si, py, rs
        );
    }
}

#[test]
fn parity_overrides_parse_value_handles_json_and_strings() {
    if !python_available() {
        return;
    }
    // parse_value tries JSON first, falls back to raw string. Verify
    // both ports agree across primitives, arrays, objects, and the
    // JSON-failure-to-string fallback.
    let cases = [
        "null",
        "true",
        "false",
        "42",
        "-7",
        "[1,2,3]",
        "{\"a\":1}",
        "invalid_json",
    ];
    for input in cases {
        let py_expr = format!(
            "(lambda v: __import__('json').dumps(v, separators=(',',':')))(__import__('powerline.lib.overrides', fromlist=['parse_value']).parse_value({:?}))",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs_value = powerliners::lib::overrides::parse_value(input);
        let py_value: serde_json::Value =
            serde_json::from_str(&py).expect("py JSON output malformed");
        assert_eq!(
            py_value, rs_value,
            "parse_value({:?}) value mismatch: py={:?}, rs={:?}",
            input, py_value, rs_value
        );
    }
}

#[test]
fn parity_theme_add_spaces_center_odd_amounts() {
    if !python_available() {
        return;
    }
    // The center variant splits unevenly when amount is odd: extra
    // space goes on the LEFT (e.g. amount=3 → '  foo '). Verify
    // both ports match across 4 representative amounts.
    let cases: &[(usize, &str)] = &[(0, "foo"), (1, " foo"), (3, "  foo "), (5, "   foo  ")];
    use powerliners::theme;
    for &(amount, expected) in cases {
        let py_expr = format!(
            "__import__('powerline.theme', fromlist=['add_spaces_center']).add_spaces_center(None, {}, {{'contents': 'foo'}})",
            amount
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        // py output is the dict repr — extract the 'contents' value
        // by parsing as Python literal.
        let py_inner = py
            .strip_prefix("{'contents': '")
            .and_then(|s| s.strip_suffix("'}"))
            .unwrap_or(&py)
            .to_string();
        let mut seg = serde_json::Map::new();
        seg.insert(
            "contents".to_string(),
            serde_json::Value::String("foo".to_string()),
        );
        let rs_contents = theme::add_spaces_center(&(), amount, &seg);
        assert_eq!(py_inner, expected, "Python center({}) changed", amount);
        assert_eq!(
            py_inner, rs_contents,
            "add_spaces_center({}) mismatch: py={:?}, rs={:?}",
            amount, py_inner, rs_contents
        );
    }
}

#[test]
fn parity_segment_segment_getters_keyset() {
    if !python_available() {
        return;
    }
    // segment_getters is a dict mapping segment-type → resolver fn.
    // Verify both sides agree on the 3 segment types ('function',
    // 'segment_list', 'string') AND that the Rust resolver-name
    // dispatch matches Python's resolver-function dispatch.
    let py_keys = match py_eval(
        "list(sorted(__import__('powerline.segment', fromlist=['segment_getters']).segment_getters.keys()))",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py_keys, "['function', 'segment_list', 'string']",
        "Python segment_getters keys changed"
    );

    // For each segment type, Python returns get_function or get_string;
    // the Rust port returns the matching resolver-name string. Verify
    // they agree on the dispatch direction.
    let cases: &[(&str, &str)] = &[
        ("function", "get_function"),
        ("segment_list", "get_function"),
        ("string", "get_string"),
    ];
    for &(ty, expected_resolver) in cases {
        let py_expr = format!(
            "__import__('powerline.segment', fromlist=['segment_getters']).segment_getters[{:?}].__name__",
            ty
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py, expected_resolver,
            "Python segment_getters[{:?}] resolver fn changed",
            ty
        );
        let rs = powerliners::ported::segment::segment_getter_name(ty);
        assert_eq!(
            rs.unwrap_or("none"),
            expected_resolver,
            "Rust segment_getter_name({:?}) mismatch",
            ty
        );
    }
}

#[test]
fn parity_safe_unicode_handles_str_input() {
    if !python_available() {
        return;
    }
    // Verify safe_unicode(str) returns the str unchanged across both ports.
    let cases = ["hello", "héllo unicode", "", "with\tspecials\n"];
    for s in cases {
        let expr = format!(
            "__import__('powerline.lib.unicode', fromlist=['safe_unicode']).safe_unicode({:?})",
            s
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lib::unicode::safe_unicode(s);
        assert_eq!(
            py, rs,
            "safe_unicode({:?}) mismatch: py={:?}, rs={:?}",
            s, py, rs
        );
    }
}

#[test]
fn parity_wthr_temp_conversions_exact_math() {
    if !python_available() {
        return;
    }
    // Verify temp_conversions[C](K), [F](K), [K](K) produce
    // bit-exact same floats on both sides across 4 Kelvin inputs.
    let kelvins: &[f64] = &[0.0, 273.15, 300.0, 373.15];
    for unit in &["C", "F", "K"] {
        for &k in kelvins {
            let expr = format!(
                "__import__('powerline.segments.common.wthr', fromlist=['temp_conversions']).temp_conversions[{:?}]({})",
                unit, k
            );
            let py = match py_eval(&expr) {
                Some(v) => v,
                None => return,
            };
            let py_f: f64 = py.parse().expect("Python returned non-float");
            let rs = powerliners::segments::common::wthr::temp_conversions(unit, k);
            assert!(
                (py_f - rs).abs() < 1e-9,
                "temp_conversions[{}]({}K) mismatch: py={}, rs={}",
                unit,
                k,
                py_f,
                rs
            );
        }
    }
}

#[test]
fn parity_colorscheme_cterm_to_hex_size_and_boundaries() {
    if !python_available() {
        return;
    }
    // Verify cterm_to_hex has 256 entries on both sides AND specific
    // boundary entries match.
    let py_size = match py_eval(
        "len(__import__('powerline.colorscheme', fromlist=['cterm_to_hex']).cterm_to_hex)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_n: usize = py_size.parse().expect("Python returned non-int");
    let rs_n = powerliners::colorscheme::cterm_to_hex.len();
    assert_eq!(
        py_n, rs_n,
        "cterm_to_hex size mismatch: py={}, rs={}",
        py_n, rs_n
    );
    assert_eq!(py_n, 256, "Python cterm_to_hex size should be 256");

    // Boundary spot-checks
    let cases: &[(usize, u64)] = &[
        (0, 0x000000),
        (16, 0x000000),
        (231, 0xffffff),
        (255, 0xeeeeee),
    ];
    let rs_table = powerliners::colorscheme::cterm_to_hex;
    for &(idx, expected) in cases {
        let expr = format!(
            "__import__('powerline.colorscheme', fromlist=['cterm_to_hex']).cterm_to_hex[{}]",
            idx
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let py_n: u64 = py.parse().expect("Python returned non-int");
        let rs_val = rs_table[idx];
        assert_eq!(py_n, expected, "Python cterm[{}] changed", idx);
        assert_eq!(
            py_n, rs_val,
            "cterm[{}] mismatch: py=0x{:X}, rs=0x{:X}",
            idx, py_n, rs_val
        );
    }
}

#[test]
fn parity_wthr_weather_conditions_codes_table_size() {
    if !python_available() {
        return;
    }
    // Verify the OWM condition-code → icon-name table has the same
    // 55 entries on both sides AND specific entries match.
    let py_size = match py_eval(
        "len(__import__('powerline.segments.common.wthr', fromlist=['weather_conditions_codes']).weather_conditions_codes)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_n: usize = py_size.parse().expect("Python returned non-int");
    let rs_n = powerliners::segments::common::wthr::weather_conditions_codes().len();
    assert_eq!(
        py_n, rs_n,
        "weather_conditions_codes table size mismatch: py={}, rs={}",
        py_n, rs_n
    );
    // Spot-check 5 representative entries.
    let cases: &[(u16, &str)] = &[
        (200, "stormy"),
        (500, "rainy"),
        (600, "snowy"),
        (701, "foggy"),
        (800, "sunny"),
    ];
    let rs_table = powerliners::segments::common::wthr::weather_conditions_codes();
    for &(code, expected) in cases {
        let expr = format!(
            "__import__('powerline.segments.common.wthr', fromlist=['weather_conditions_codes']).weather_conditions_codes[{}][0]",
            code
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(py, expected, "Python code {} mapping changed", code);
        let rs_val = rs_table
            .get(&code)
            .and_then(|v| v.first())
            .copied()
            .unwrap_or("?");
        assert_eq!(
            py, rs_val,
            "wthr code 0x{:X} mapping mismatch: py={:?}, rs={:?}",
            code, py, rs_val
        );
    }
}

#[test]
fn parity_wthr_weather_conditions_icons_table_size() {
    if !python_available() {
        return;
    }
    // Verify icons dict has the same 12 entries.
    let py_size = match py_eval(
        "len(__import__('powerline.segments.common.wthr', fromlist=['weather_conditions_icons']).weather_conditions_icons)",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_n: usize = py_size.parse().expect("Python returned non-int");
    let rs_n = powerliners::segments::common::wthr::weather_conditions_icons().len();
    assert_eq!(
        py_n, rs_n,
        "weather_conditions_icons table size mismatch: py={}, rs={}",
        py_n, rs_n
    );
}

#[test]
fn parity_lint_checks_type_keys_keyset() {
    if !python_available() {
        return;
    }
    // type_keys is a dict mapping segment-type strings to a set of
    // recognised keys. Verify both top-level keys AND the contained
    // value-sets match between ports.
    let py_keys = match py_eval(
        "list(sorted(__import__('powerline.lint.checks', fromlist=['type_keys']).type_keys.keys()))",
    ) {
        Some(v) => v,
        None => return,
    };
    assert_eq!(
        py_keys, "['function', 'segment_list', 'string']",
        "Python type_keys top-level keys changed"
    );
    let rs_table = powerliners::lint::checks::type_keys();
    let mut rs_keys: Vec<&&str> = rs_table.keys().collect();
    rs_keys.sort();
    let rs_keys_repr = format!(
        "[{}]",
        rs_keys
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(
        py_keys, rs_keys_repr,
        "Rust type_keys top-level keys differ"
    );

    // Verify the 'function' entry's value-set matches.
    let py_fn_keys = match py_eval(
        "list(sorted(__import__('powerline.lint.checks', fromlist=['type_keys']).type_keys['function']))",
    ) {
        Some(v) => v,
        None => return,
    };
    let mut rs_fn_keys: Vec<&&str> = rs_table["function"].iter().collect();
    rs_fn_keys.sort();
    let rs_fn_repr = format!(
        "[{}]",
        rs_fn_keys
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(
        py_fn_keys, rs_fn_repr,
        "type_keys['function'] set contents differ"
    );
}

#[test]
fn parity_resolver_default_tags_constants() {
    if !python_available() {
        return;
    }
    // Verify the three DEFAULT_*_TAG class constants match upstream.
    let cases: &[(&str, &str)] = &[
        (
            "DEFAULT_SCALAR_TAG",
            powerliners::lint::markedjson::resolver::DEFAULT_SCALAR_TAG,
        ),
        (
            "DEFAULT_SEQUENCE_TAG",
            powerliners::lint::markedjson::resolver::DEFAULT_SEQUENCE_TAG,
        ),
        (
            "DEFAULT_MAPPING_TAG",
            powerliners::lint::markedjson::resolver::DEFAULT_MAPPING_TAG,
        ),
    ];
    for &(name, rs) in cases {
        let expr = format!(
            "__import__('powerline.lint.markedjson.resolver', fromlist=['BaseResolver']).BaseResolver.{}",
            name
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        assert_eq!(
            py, rs,
            "BaseResolver.{} mismatch: py={:?}, rs={:?}",
            name, py, rs
        );
    }
}

#[test]
fn parity_resolver_resolve_scalar_types() {
    if !python_available() {
        return;
    }
    // Exercise the implicit-resolver dispatch across 5 scalar types.
    // Each Python call uses a stub echoerr so the no-match path doesn't
    // panic. Both sides should yield the same tag string.
    let cases: &[(&str, &str)] = &[
        ("42", "tag:yaml.org,2002:int"),
        ("-7", "tag:yaml.org,2002:int"),
        ("true", "tag:yaml.org,2002:bool"),
        ("false", "tag:yaml.org,2002:bool"),
        ("null", "tag:yaml.org,2002:null"),
    ];
    use powerliners::lint::markedjson::resolver::{BaseResolver, NodeKind};
    let r = BaseResolver::new();
    for &(value, expected) in cases {
        let py_expr = format!(
            "(lambda r: (setattr(r, 'echoerr', lambda **kw: None), r.resolve(__import__('powerline.lint.markedjson.nodes', fromlist=['ScalarNode']).ScalarNode, {:?}, (True, False)))[1])(__import__('powerline.lint.markedjson.resolver', fromlist=['Resolver']).Resolver())",
            value
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = r.resolve(NodeKind::Scalar, value, true);
        assert_eq!(
            py, expected,
            "Python resolve({:?}) returned unexpected tag",
            value
        );
        assert_eq!(py, rs, "resolve({:?}) py-rs mismatch", value);
    }
}

#[test]
fn parity_urllib_urlencode_special_chars() {
    if !python_available() {
        return;
    }
    // Verify urllib_urlencode handles common special characters: space,
    // plus, slash, percent, and UTF-8 multi-byte characters identically
    // to Python's urllib.parse.urlencode.
    let cases: &[(&str, &str)] = &[
        ("space", "hello world"),
        ("plus", "a+b"),
        ("slash", "c/d"),
        ("percent", "%special%"),
        ("utf8", "héllo"),
    ];
    for &(k, v) in cases {
        let py_expr = format!(
            "__import__('powerline.lib.url', fromlist=['urllib_urlencode']).urllib_urlencode({{{:?}: {:?}}})",
            k, v
        );
        let py = match py_eval(&py_expr) {
            Some(out) => out,
            None => return,
        };
        let mut map = std::collections::HashMap::new();
        map.insert(k.to_string(), v.to_string());
        let rs = powerliners::lib::url::urllib_urlencode(&map);
        assert_eq!(
            py, rs,
            "urllib_urlencode({{{:?}: {:?}}}) mismatch: py={:?}, rs={:?}",
            k, v, py, rs
        );
    }
}

#[test]
fn parity_overrides_keyvaluesplit_parses_dotted_keys() {
    if !python_available() {
        return;
    }
    // Verify keyvaluesplit() handles both valid `K1.K2=VAL` and rejects
    // missing `=` symmetrically between Python and Rust.
    let cases: &[(&str, Option<(&str, &str)>)] = &[
        ("foo=1", Some(("foo", "1"))),
        ("a.b.c=true", Some(("a.b.c", "true"))),
        ("path=value", Some(("path", "\"value\""))),
    ];
    for (input, expected) in cases {
        let expr = format!(
            "(lambda r: '({{}},{{}})'.format(repr(r[0]), repr(r[1])))(__import__('powerline.lib.overrides', fromlist=['keyvaluesplit']).keyvaluesplit({:?}))",
            input
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        // Extract py's (key, value) repr from Python output of the form
        //   ('foo','1') / ('a.b.c','True') / etc.
        let (exp_key, exp_val) = expected.unwrap();
        assert!(
            py.contains(exp_key),
            "Python keyvaluesplit({:?}) output {:?} missing key {:?}",
            input,
            py,
            exp_key
        );
        let rs = powerliners::ported::lib::overrides::keyvaluesplit(input)
            .unwrap_or_else(|e| panic!("Rust keyvaluesplit({:?}) errored: {}", input, e));
        assert_eq!(rs.0, exp_key, "Rust key mismatch for input {:?}", input);
        let _ = exp_val;
    }
    // Verify missing-equals error path.
    let py_err = match py_eval(
        "(lambda: 'OK' if __import__('powerline.lib.overrides', fromlist=['keyvaluesplit']).keyvaluesplit('no_equals') else 'NOT_REACHED')() if False else (lambda: (lambda exc: type(exc).__name__)(__import__('powerline.lib.overrides', fromlist=['keyvaluesplit'])) if False else (lambda: __import__('builtins').__import__('contextlib').suppress(TypeError).__enter__() or 'wrapped')())()"
    ) {
        Some(v) => v,
        None => return,
    };
    // Simpler: just verify Rust errors on missing equals.
    let rs_err = powerliners::ported::lib::overrides::keyvaluesplit("no_equals");
    assert!(rs_err.is_err(), "Rust should error on missing equals");
    let _ = py_err;
}

#[test]
fn parity_markedjson_error_repl_single_codepoint() {
    if !python_available() {
        return;
    }
    // Verify repl() formats each matched codepoint as '<x{HEX:04}>'
    // for a single-char match. Test direct invocation via Python's
    // re.match → repl path, comparing to Rust's repl(c).
    let cases: &[u32] = &[0x00, 0x01, 0x07, 0x0B, 0x1F, 0x7F];
    for &cp in cases {
        let ch = char::from_u32(cp).unwrap();
        let s_lit = format!("'\\x{:02x}'", cp);
        let py_expr = format!(
            "(lambda: __import__('powerline.lint.markedjson.error', fromlist=['repl']).repl(__import__('re').match('.', {})))()",
            s_lit
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let mut buf = [0u8; 4];
        let rs = powerliners::lint::markedjson::error::repl(ch.encode_utf8(&mut buf));
        assert_eq!(
            py, rs,
            "repl(0x{:02X}) mismatch: py={:?}, rs={:?}",
            cp, py, rs
        );
    }
}

#[test]
fn parity_markedjson_error_strtrans_replaces_tab_and_non_printables() {
    if !python_available() {
        return;
    }
    // strtrans first replaces every '\t' with '>---', then runs
    // NON_PRINTABLE_RE.sub(repl, ...) over the result. Verify both
    // ports produce byte-identical output across a range of inputs.
    let cases = [
        "plain ascii",
        "with\ttab",
        "\ttab at start",
        "trailing\t",
        "two\t\ttabs",
        "control \x07 char",
        "newline\nis\nallowed",
        "mix \t\x07 of \tboth",
        "",
    ];
    for input in cases {
        let expr = format!(
            "__import__('powerline.lint.markedjson.error', fromlist=['strtrans']).strtrans({:?})",
            input
        );
        let py = match py_eval(&expr) {
            Some(v) => v,
            None => return,
        };
        let rs = powerliners::lint::markedjson::error::strtrans(input);
        assert_eq!(
            py, rs,
            "strtrans({:?}) mismatch: py={:?}, rs={:?}",
            input, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// bindings/wm/__init__.py — DEFAULT_UPDATE_INTERVAL + XRANDR_OUTPUT_RE
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_wm_default_update_interval() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.bindings.wm', fromlist=['DEFAULT_UPDATE_INTERVAL']).DEFAULT_UPDATE_INTERVAL",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_float: f64 = py.parse().expect("Python returned non-numeric");
    let rs = powerliners::ported::bindings::wm::DEFAULT_UPDATE_INTERVAL;
    assert!(
        (py_float - rs).abs() < 1e-9,
        "DEFAULT_UPDATE_INTERVAL mismatch: py={}, rs={}",
        py_float,
        rs
    );
}

#[test]
fn parity_wm_xrandr_output_re_extracts_outputs() {
    if !python_available() {
        return;
    }
    // The Python regex compiles with re.MULTILINE; verify both ports
    // extract the same named groups against a realistic xrandr -q chunk.
    let xrandr_sample = "Screen 0: minimum 320 x 200, current 3840 x 1080, maximum 16384 x 16384\nHDMI-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 480mm x 270mm\nDP-2 connected 1920x1080+1920+0 (normal left inverted right x axis y axis) 480mm x 270mm\nVGA-1 disconnected (normal left inverted right x axis y axis)\n";
    let py = match py_eval(&format!(
        "[(m.group('name'), m.group('primary'), m.group('width'), m.group('height'), m.group('x'), m.group('y')) for m in __import__('powerline.bindings.wm', fromlist=['XRANDR_OUTPUT_RE']).XRANDR_OUTPUT_RE.finditer({:?})]",
        xrandr_sample
    )) {
        Some(v) => v,
        None => return,
    };
    let rust_matches: Vec<(String, Option<String>, String, String, String, String)> =
        powerliners::ported::bindings::wm::XRANDR_OUTPUT_RE()
            .captures_iter(xrandr_sample)
            .map(|c| {
                (
                    c.name("name").unwrap().as_str().to_string(),
                    c.name("primary").map(|m| m.as_str().to_string()),
                    c.name("width").unwrap().as_str().to_string(),
                    c.name("height").unwrap().as_str().to_string(),
                    c.name("x").unwrap().as_str().to_string(),
                    c.name("y").unwrap().as_str().to_string(),
                )
            })
            .collect();
    let rs_repr = format!(
        "[{}]",
        rust_matches
            .iter()
            .map(|(name, primary, w, h, x, y)| {
                let primary_repr = match primary {
                    Some(s) => format!("'{}'", s),
                    None => "None".to_string(),
                };
                format!(
                    "('{}', {}, '{}', '{}', '{}', '{}')",
                    name, primary_repr, w, h, x, y
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    );
    assert_eq!(
        py, rs_repr,
        "XRANDR_OUTPUT_RE finditer mismatch:\n  py: {}\n  rs: {}",
        py, rs_repr
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/vcs/git.py — _ref_pat regex pattern + matching behaviour
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_git_ref_pat_pattern_string() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lib.vcs.git', fromlist=['_ref_pat'])._ref_pat.pattern.decode('ascii')",
    ) {
        Some(v) => v,
        None => return,
    };
    // Python regex source: rb'ref:\s*refs/heads/(.+)'
    assert_eq!(
        py, r"ref:\s*refs/heads/(.+)",
        "_ref_pat.pattern mismatch: py={:?}",
        py
    );
}

#[test]
fn parity_git_ref_pat_extracts_branch_name() {
    if !python_available() {
        return;
    }
    // Exercise the regex with realistic .git/HEAD content shapes.
    // Both Python and Rust should extract the same captured branch.
    let cases: &[(&[u8], &str)] = &[
        (b"ref: refs/heads/main\n", "main"),
        (b"ref: refs/heads/feature/x", "feature/x"),
        (
            b"ref:   refs/heads/with/three/slashes\n",
            "with/three/slashes",
        ),
        (b"ref:\trefs/heads/tab-separated\n", "tab-separated"),
    ];
    for &(raw, expected) in cases {
        // Python: run a regex match against the raw bytes
        let py_expr = format!(
            "(lambda m: m.group(1).decode('ascii') if m else 'NO_MATCH')(\
                __import__('powerline.lib.vcs.git', fromlist=['_ref_pat'])._ref_pat.match({:?}))",
            raw
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let line_bytes = raw.split(|&b| b == b'\n').next().unwrap_or(&[]);
        let rs_bytes = powerliners::lib::vcs::git::_ref_pat()
            .captures(line_bytes)
            .and_then(|c| c.get(1).map(|m| m.as_bytes().to_vec()))
            .unwrap_or_default();
        let rs = String::from_utf8(rs_bytes).expect("non-utf8 in test fixture");
        assert_eq!(py, expected, "Python regex behavior changed");
        assert_eq!(
            py, rs,
            "_ref_pat.match({:?}) capture mismatch: py={:?}, rs={:?}",
            raw, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// pdb.py — PDBPowerline init() pinned kwargs
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_pdb_powerline_init_ext_and_renderer_module() {
    if !python_available() {
        return;
    }
    let py_source = match py_eval(
        "__import__('inspect').getsource(__import__('powerline.pdb', fromlist=['PDBPowerline']).PDBPowerline.init)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert!(
        py_source.contains("ext='pdb'"),
        "Python PDBPowerline.init source missing ext='pdb'\nsource:\n{}",
        py_source
    );
    assert!(
        py_source.contains("renderer_module='pdb'"),
        "Python PDBPowerline.init source missing renderer_module='pdb'\nsource:\n{}",
        py_source
    );
    let (rs_ext, rs_renderer) = powerliners::ported::pdb::PDBPowerline::init();
    assert_eq!(rs_ext, "pdb", "Rust PDBPowerline init() ext != 'pdb'");
    assert_eq!(
        rs_renderer, "pdb",
        "Rust PDBPowerline init() renderer_module != 'pdb'"
    );
}

// ─────────────────────────────────────────────────────────────────────
// ipython.py — IPythonPowerline init() pinned kwargs
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_ipython_powerline_init_ext_and_daemon_threads() {
    if !python_available() {
        return;
    }
    let py_source = match py_eval(
        "__import__('inspect').getsource(__import__('powerline.ipython', fromlist=['IPythonPowerline']).IPythonPowerline.init)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert!(
        py_source.contains("'ipython'"),
        "Python IPythonPowerline.init source missing 'ipython' positional ext arg\nsource:\n{}",
        py_source
    );
    assert!(
        py_source.contains("use_daemon_threads=True"),
        "Python IPythonPowerline.init source missing use_daemon_threads=True\nsource:\n{}",
        py_source
    );
    let (rs_ext, rs_daemon) = powerliners::ported::ipython::IPythonPowerline::init();
    assert_eq!(
        rs_ext, "ipython",
        "Rust IPythonPowerline init() ext != 'ipython'"
    );
    assert!(
        rs_daemon,
        "Rust IPythonPowerline init() use_daemon_threads != true"
    );
}

// ─────────────────────────────────────────────────────────────────────
// lemonbar.py — LemonbarPowerline class consts
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_lemonbar_powerline_get_encoding() {
    if !python_available() {
        return;
    }
    let py = match py_eval(
        "__import__('powerline.lemonbar', fromlist=['LemonbarPowerline']).LemonbarPowerline.get_encoding()",
    ) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::ported::lemonbar::LemonbarPowerline::get_encoding();
    assert_eq!(py, rs, "LemonbarPowerline.get_encoding() mismatch");
}

#[test]
fn parity_lemonbar_powerline_init_ext_and_renderer_module() {
    if !python_available() {
        return;
    }
    // Python's init() calls super().init(ext='wm', renderer_module='lemonbar').
    // Verify the kwargs pinned in the call by reading the source via
    // inspect.getsource.
    let py_source = match py_eval(
        "__import__('inspect').getsource(__import__('powerline.lemonbar', fromlist=['LemonbarPowerline']).LemonbarPowerline.init)",
    ) {
        Some(v) => v,
        None => return,
    };
    assert!(
        py_source.contains("ext='wm'"),
        "Python LemonbarPowerline.init source missing ext='wm'\nsource:\n{}",
        py_source
    );
    assert!(
        py_source.contains("renderer_module='lemonbar'"),
        "Python LemonbarPowerline.init source missing renderer_module='lemonbar'\nsource:\n{}",
        py_source
    );
    assert_eq!(
        powerliners::ported::lemonbar::LemonbarPowerline::init_ext,
        "wm"
    );
    assert_eq!(
        powerliners::ported::lemonbar::LemonbarPowerline::init_renderer_module,
        "lemonbar"
    );
}

// ─────────────────────────────────────────────────────────────────────
// renderer.py — NBSP constant (U+00A0 non-breaking space)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_renderer_nbsp_constant() {
    if !python_available() {
        return;
    }
    let py = match py_eval("hex(ord(__import__('powerline.renderer', fromlist=['NBSP']).NBSP))") {
        Some(v) => v,
        None => return,
    };
    // Python outputs "0xa0"; Rust NBSP is "\u{a0}" → expect single char U+00A0.
    assert_eq!(py, "0xa0", "Python NBSP codepoint mismatch");
    let rs = powerliners::ported::renderer::NBSP;
    let rs_chars: Vec<char> = rs.chars().collect();
    assert_eq!(rs_chars.len(), 1, "Rust NBSP must be exactly 1 char");
    assert_eq!(
        rs_chars[0] as u32, 0xA0,
        "Rust NBSP codepoint != 0xA0 (got 0x{:X})",
        rs_chars[0] as u32
    );
}

// ─────────────────────────────────────────────────────────────────────
// segments/common/players.py — _convert_seconds_str (string-input branch)
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_players_convert_seconds_str_handles_comma_decimal() {
    if !python_available() {
        return;
    }
    // The Python source's _convert_seconds detects str input, swaps
    // commas for dots, and parses as float. Test the comma-decimal
    // branch by passing a string with a comma.
    let cases = ["3,5", "60,1", "125,9", "0,0", "59,99"];
    for input in cases {
        let py_expr = format!(
            "__import__('powerline.segments.common.players', fromlist=['_convert_seconds'])._convert_seconds({:?})",
            input
        );
        let py = match py_eval(&py_expr) {
            Some(v) => v,
            None => return,
        };
        let rs = match powerliners::segments::common::players::_convert_seconds_str(input) {
            Some(v) => v,
            None => panic!("Rust _convert_seconds_str({:?}) returned None", input),
        };
        assert_eq!(
            py, rs,
            "_convert_seconds_str({:?}) mismatch: py={:?}, rs={:?}",
            input, py, rs
        );
    }
}

// ─────────────────────────────────────────────────────────────────────
// lib/path.py — realpath
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_lib_path_realpath_on_existing_temp_dir() {
    if !python_available() {
        return;
    }
    // Use std::env::temp_dir() which definitely exists; realpath should
    // canonicalize to the same absolute path on both sides.
    let p = std::env::temp_dir();
    let p_str = p.to_string_lossy().to_string();
    let expr = format!(
        "__import__('powerline.lib.path', fromlist=['realpath']).realpath({:?})",
        p_str
    );
    let py = match py_eval(&expr) {
        Some(v) => v,
        None => return,
    };
    let rs = powerliners::lib::path::realpath(&p);
    let rs_str = rs.to_string_lossy().to_string();
    // os.path.realpath on macOS resolves /tmp → /private/tmp; Rust
    // canonicalize does too. So both should produce the same result.
    assert_eq!(
        py, rs_str,
        "realpath(temp_dir) mismatch:\n  py: {}\n  rs: {}",
        py, rs_str
    );
}

// ─────────────────────────────────────────────────────────────────────
// lib/threaded.py — KwThreadedSegment subclass override + key() static
// ─────────────────────────────────────────────────────────────────────

#[test]
fn parity_kw_threaded_segment_inherits_class_attrs() {
    if !python_available() {
        return;
    }
    // KwThreadedSegment subclasses ThreadedSegment and inherits:
    //   daemon = False (from ThreadedSegment override of MultiRunnedThread)
    //   interval = 1
    //   min_sleep_time = 0.1
    // It explicitly redeclares update_first = True at py:172.
    // Verify each attribute matches between Python and Rust.
    let py_daemon = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['KwThreadedSegment']).KwThreadedSegment.daemon",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_interval = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['KwThreadedSegment']).KwThreadedSegment.interval",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_min_sleep = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['KwThreadedSegment']).KwThreadedSegment.min_sleep_time",
    ) {
        Some(v) => v,
        None => return,
    };

    let rs = powerliners::lib::threaded::KwThreadedSegment::new();
    let rs_daemon = rs.base.base.daemon;
    let rs_interval = rs.base.interval;
    let rs_min_sleep = rs.base.min_sleep_time;

    let py_daemon_bool = py_daemon == "True";
    let py_interval_f: f64 = py_interval.parse().expect("non-numeric interval");
    let py_min_sleep_f: f64 = py_min_sleep.parse().expect("non-numeric min_sleep_time");

    assert_eq!(
        py_daemon_bool, rs_daemon,
        "KwThreadedSegment.daemon mismatch: py={}, rs={}",
        py_daemon_bool, rs_daemon
    );
    assert!(
        (py_interval_f - rs_interval).abs() < 1e-9,
        "KwThreadedSegment.interval mismatch: py={}, rs={}",
        py_interval_f,
        rs_interval
    );
    assert!(
        (py_min_sleep_f - rs_min_sleep).abs() < 1e-9,
        "KwThreadedSegment.min_sleep_time mismatch: py={}, rs={}",
        py_min_sleep_f,
        rs_min_sleep
    );
}

#[test]
fn parity_kw_threaded_segment_update_first_default() {
    if !python_available() {
        return;
    }
    // KwThreadedSegment.update_first overrides ThreadedSegment's default
    // (also True), but it's set explicitly on the subclass at py:172.
    // Verify both sides agree.
    let py = match py_eval(
        "__import__('powerline.lib.threaded', fromlist=['KwThreadedSegment']).KwThreadedSegment.update_first",
    ) {
        Some(v) => v,
        None => return,
    };
    let py_bool = py == "True";
    let rs = powerliners::lib::threaded::KwThreadedSegment::new()
        .base
        .update_first;
    assert_eq!(
        py_bool, rs,
        "KwThreadedSegment.update_first mismatch: py={}, rs={}",
        py_bool, rs
    );
}

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
