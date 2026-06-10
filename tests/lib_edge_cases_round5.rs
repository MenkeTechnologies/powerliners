// vim:fileencoding=utf-8:noet
//! Hand-crafted edge-case tests for `src/ported/lib/{overrides,memoize}.rs`.
//!
//! These pin specific behavioural contracts that the inline `#[cfg(test)]`
//! blocks in the source files (and the bulk parity harness in
//! `parity_against_upstream.rs`) don't exercise. Each test targets ONE
//! bug class — empty-key edge cases, error-swallowing divergence, cache
//! timestamp staleness — not a mirror of the happy path.
//!
//! Bug classes covered:
//!   1. `keyvaluesplit("=foo")` — empty key on the LHS of `=`. Upstream
//!      Python returns `("", parse_value("foo"))` because `s.index("=")`
//!      is 0; downstream code in `build_nested`/`parsedotval` then
//!      `o.split(".")` produces `[""]` and the key insert is `""`. Pin
//!      that the Rust port matches: empty-string key, not error.
//!   2. `parse_override_var("_x=1;y=2")` silently filters the `_x` failure
//!      via `.filter_map(...).ok()`. This DIVERGES from Python's generator
//!      semantics (Python raises ValueError on first iteration). The test
//!      pins the current Rust-only "swallow" behaviour so a future fix
//!      that switches to Result<Vec<_>, String> forces a test update —
//!      and so the regression direction is visible in code review.
//!   3. `memoize.decorated_function` records the cache timestamp from
//!      BEFORE `compute()` runs, not after (see src/ported/lib/memoize.rs
//!      lines 134 / 160). Python re-reads `monotonic()` AFTER the call
//!      (py:38-39). For a long-running compute, this makes the cache
//!      expire too early. The test demonstrates the early-expiry symptom
//!      on a short timeout with a deliberately-slow compute closure.

use powerliners::lib::memoize::memoize;
use powerliners::lib::overrides::{keyvaluesplit, parse_override_var, parse_value};
use serde_json::{json, Map};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;

/// `keyvaluesplit("=foo")` returns Ok with an empty-string key and the
/// parsed RHS value, mirroring CPython's `s.index("=") == 0; o = s[:0]`
/// at `vendor/powerline/powerline/lib/overrides.py:42-43`.
///
/// NOT a mirror test: the existing inline test only asserts the
/// happy-path `ext.tmux.theme=default` shape. Empty LHS is a real CLI
/// input shape (`powerline --theme-option =something`) that would
/// otherwise sneak through unvalidated.
#[test]
fn keyvaluesplit_empty_lhs_returns_empty_string_key() {
    let r = keyvaluesplit("=hello");
    assert!(
        r.is_ok(),
        "keyvaluesplit(\"=hello\") should succeed (matches Python s[:0] = ''), got {:?}",
        r
    );
    let (k, v) = r.unwrap();
    assert_eq!(k, "", "empty-LHS produces empty-string key per py:42");
    assert_eq!(v, json!("hello"), "RHS is parsed via parse_value");
}

/// `parse_value("-")` — a bare minus — currently swallows the
/// `serde_json::from_str` error and falls back to `Value::String("-")`
/// (see `src/ported/lib/overrides.rs:48` `unwrap_or_else`).
///
/// Upstream Python `json.loads("-")` raises `JSONDecodeError` which
/// propagates out of `parse_value`. The Rust port turns the error into a
/// raw-string return. This test pins the CURRENT Rust behaviour so the
/// divergence is visible — a future port-faithfulness fix that propagates
/// the error must update this test.
///
/// NOT boilerplate: the existing tests only cover well-formed numbers
/// (`"42"`, `"-3"`, `"2.5"`). A bare `-` is the smallest input that
/// triggers the JSON-start heuristic AND fails JSON parsing — the
/// exact code-path divergence point.
#[test]
fn parse_value_bare_minus_silently_falls_back_to_string() {
    let v = parse_value("-");
    assert_eq!(
        v,
        json!("-"),
        "bare '-' currently returns Value::String('-') via unwrap_or_else \
         fallback at overrides.rs:48 (Python raises JSONDecodeError here — \
         divergence; see bug report)"
    );
}

/// `parse_override_var("_x=1;y=2")` should — per upstream — raise on
/// the first iteration because `_x` triggers the leading-underscore
/// check at `overrides.py:39-40`. The Rust port instead drops `_x`
/// silently via `.filter_map(...).ok()` and returns the remaining
/// well-formed item.
///
/// This test pins the CURRENT silent-drop behaviour, asserting only
/// the well-formed pair survives. A future fix that returns
/// `Result<Vec<(String, Value)>, String>` to match Python's
/// fail-fast generator will need to flip this assertion — exactly the
/// visibility the test exists to provide.
///
/// NOT boilerplate: the existing `parse_override_var_skips_empty` test
/// covers empty items (which Python ALSO skips via `if item`).
/// Leading-underscore items are NOT skipped by Python — they raise.
/// The divergence point is precisely this case.
#[test]
fn parse_override_var_silently_drops_leading_underscore_items() {
    let items = parse_override_var("_x=1;y=2");
    // Python would have raised on _x; Rust currently drops it and
    // returns just y=2.
    assert_eq!(
        items.len(),
        1,
        "current Rust behaviour: leading-underscore items are silently \
         filtered (overrides.rs:184 `.filter_map(...).ok()`); \
         Python upstream raises ValueError"
    );
    assert_eq!(items[0].0, "y");
    assert_eq!(items[0].1, json!(2));
}

/// `parse_override_var("foo;bar=1")` — `foo` has no `=`, Python raises
/// TypeError on first iteration. The Rust port currently filters it
/// out via the same `.ok()` chain. Companion to the underscore test:
/// the two failure modes both go through the same divergent code
/// path, so locking both down makes the eventual fix obvious.
#[test]
fn parse_override_var_silently_drops_no_equals_items() {
    let items = parse_override_var("foo;bar=1");
    assert_eq!(
        items.len(),
        1,
        "missing-equals items are silently filtered (overrides.rs:184); \
         Python upstream raises TypeError"
    );
    assert_eq!(items[0].0, "bar");
    assert_eq!(items[0].1, json!(1));
}

/// `memoize.decorated_function` writes the cache `time` field from a
/// snapshot taken BEFORE `compute()` runs (see `memoize.rs:134, 160`).
/// Upstream Python re-reads `monotonic()` AFTER `func()` (py:38-39),
/// so the stored timestamp reflects when the result became available,
/// not when the call started.
///
/// Symptom: with a 100ms timeout and a 60ms compute that's invoked at
/// `t=0`, the cache entry stores `time = 0`. The cached result becomes
/// stale at `t=100ms` (per the `cached.time + self.timeout` check at
/// memoize.rs:143) — only 40ms after `compute()` finished returning.
/// Python would store `time ≈ 60ms` and the entry would stay live
/// until `t=160ms`. We exercise this by waiting just over (timeout -
/// compute_delay) post-result and observing the second call recomputes.
///
/// NOT a mirror of the existing `memoize_recomputes_after_timeout` test —
/// that test uses a compute closure that returns instantly. The bug
/// here only surfaces when compute time is a significant fraction of
/// the timeout; the existing test would pass either way.
#[test]
fn memoize_stores_pre_compute_timestamp_causing_early_expiry() {
    // Window choices:
    //   timeout       = 200ms
    //   compute delay = 120ms
    //   wait after  = 90ms
    // Total wall-clock at second call ≈ compute (120ms) + wait (90ms) = 210ms
    // Stored time in Rust = t_start ≈ 0  → expires at 0 + 200ms = 200ms
    //                                   → at 210ms the entry IS expired → recompute
    // Stored time in Python = t_start + 120ms = 120ms → expires at 320ms
    //                                   → at 210ms the entry is still live → NO recompute
    // So under the bug, counter == 2; under correct (Python) behaviour, counter == 1.
    let m = memoize::new(0.200);
    let counter = Arc::new(AtomicU32::new(0));
    let mut kwargs = Map::new();
    kwargs.insert("x".to_string(), json!(42));

    let c1 = counter.clone();
    m.get_or_compute(&kwargs, move |_| {
        c1.fetch_add(1, Ordering::SeqCst);
        sleep(Duration::from_millis(120));
        json!("computed")
    });
    sleep(Duration::from_millis(90));
    let c2 = counter.clone();
    m.get_or_compute(&kwargs, move |_| {
        c2.fetch_add(1, Ordering::SeqCst);
        json!("recomputed")
    });
    // Pin the CURRENT (buggy) Rust behaviour: the second call did
    // recompute because the stored timestamp is too old. A future fix
    // moving the `monotonic()` read to after `compute()` (per py:38-39)
    // will reduce this to 1 — that's the desired regression direction.
    let n = counter.load(Ordering::SeqCst);
    assert_eq!(
        n, 2,
        "memoize.rs:134/160 stores `time` from BEFORE compute(); a slow \
         compute makes the entry expire too early. Python (memoize.py:38-39) \
         re-reads monotonic() AFTER func(), so this case would NOT recompute. \
         Observed counter={}, expected=2 under current bug.",
        n
    );
}
