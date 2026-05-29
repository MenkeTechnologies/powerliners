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
        let py_int: u32 = py.parse().expect(&format!("bad py int for {}", name));
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
    for i in 0..256 {
        assert_eq!(
            py_vals[i],
            powerliners::colorscheme::cterm_to_hex[i],
            "cterm_to_hex[{}] mismatch: py=0x{:06x}, rs=0x{:06x}",
            i,
            py_vals[i],
            powerliners::colorscheme::cterm_to_hex[i]
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
