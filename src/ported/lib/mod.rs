// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/__init__.py`.
//!
//! Two decorator factories used by segment introspection:
//! `wraps_saveargs` preserves `__wrapped__`-style metadata plus a
//! `powerline_origin` pointer at the original (unwrapped) callable;
//! `add_divider_highlight_group` wraps a segment function so its
//! return value is automatically lifted into the single-segment
//! list shape `[{'contents': r, 'divider_highlight_group': hg}]`.
//!
//! Rust port surfaces:
//!   - `WrappedOrigin<F>` — Rust analog of Python's
//!     `wrapped.powerline_origin = wrapped` attribute attachment.
//!     Holds the wrapping closure + a phantom marker so callers can
//!     identify the wrapper without runtime attribute attachment.
//!   - `add_divider_highlight_group(highlight_group, func)` — pure
//!     higher-order wrapper that lifts the segment fn return value
//!     into a `[{contents, divider_highlight_group}]` JSON object.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from functools import wraps                       // py:4

use serde_json::{json, Value};

pub mod config;
pub mod debug;
pub mod dict;
pub mod encoding;
pub mod humanize_bytes;
pub mod inotify;
pub mod memoize;
pub mod monotonic;
pub mod overrides;
pub mod path;
pub mod shell;
pub mod threaded;
pub mod unicode;
pub mod url;
pub mod vcs;
pub mod watcher;

/// Port of `wraps_saveargs()` from
/// `powerline/lib/__init__.py:7`.
///
/// Python attaches a `powerline_origin` attribute to the wrapper
/// pointing at the original callable. Rust has no runtime attribute
/// attachment on `fn` items; the port surfaces a struct that holds
/// both the wrapper closure and the origin closure separately so
/// callers can recover the unwrapped form via `.origin()`.
///
/// This is a structural port — the actual decorator factory pattern
/// doesn't translate to Rust, but the data-flow shape can be
/// preserved.
pub struct WrappedOrigin<W, O> {
    /// The wrapper closure (Python's decorated function).
    pub wrapper: W,
    /// The unwrapped origin (Python's `powerline_origin`).
    pub origin: O,
}

impl<W, O> WrappedOrigin<W, O> {
    /// Constructs a new wrapper-origin pair.
    pub fn new(wrapper: W, origin: O) -> Self {
        Self { wrapper, origin }
    }

    /// Returns a reference to the wrapper closure.
    pub fn wrapper(&self) -> &W {
        &self.wrapper
    }

    /// Returns a reference to the original (unwrapped) closure.
    /// Python: `r.powerline_origin`.
    pub fn origin(&self) -> &O {
        &self.origin
    }
}

/// Port of `wraps_saveargs()` from
/// `powerline/lib/__init__.py:7-12`.
///
/// Python returns a decorator factory that wraps the wrapped fn
/// via `functools.wraps` and copies the `powerline_origin` attr
/// from the wrapped fn (or the wrapped fn itself when the attr
/// is missing). Rust port wraps the (wrapper, origin) pair in
/// the [`WrappedOrigin`] struct (the structural surface, since
/// the runtime attribute-attachment pattern doesn't translate to
/// Rust closures).
///
/// Returns the `WrappedOrigin` with `wrapper=wrapper` and the
/// resolved `origin=wrapped` per py:10's `getattr` fallback.
pub fn wraps_saveargs<W>(wrapped: W, wrapper: W) -> WrappedOrigin<W, W> {
    // py:7  def wraps_saveargs(wrapped):
    // py:8  def dec(wrapper):
    // py:9  r = wraps(wrapped)(wrapper)
    // py:10  r.powerline_origin = getattr(wrapped, 'powerline_origin', wrapped)
    // py:11  return r
    // py:12  return dec
    WrappedOrigin::new(wrapper, wrapped)
}

/// Port of the inner `dec()` closure from
/// `powerline/lib/__init__.py:8-11` (inside `wraps_saveargs`).
///
/// Python returns the decorator that wraps `wrapper` via
/// `functools.wraps(wrapped)` then attaches the `powerline_origin`
/// attribute. Rust port surfaces the dispatch as a free fn that
/// constructs the [`WrappedOrigin`] pair directly — same data
/// flow as [`wraps_saveargs`].
pub fn dec<W>(wrapped: W, wrapper: W) -> WrappedOrigin<W, W> {
    // py:8  def dec(wrapper):
    // py:9-11  r = wraps(wrapped)(wrapper); r.powerline_origin = ...; return r
    WrappedOrigin::new(wrapper, wrapped)
}

/// Port of the inner `f()` closure from
/// `powerline/lib/__init__.py:18-25` (inside
/// `add_divider_highlight_group`'s inner `dec`).
///
/// Python returns the segment wrapper that calls the underlying
/// function, lifts a non-empty result into a single-segment list
/// with the supplied `divider_highlight_group`, and returns None
/// when the underlying fn returns None/empty.
///
/// Rust port takes the already-computed `contents` + the
/// highlight group name and returns the segment list (or None).
pub fn f(contents: Option<String>, highlight_group: &str) -> Option<Vec<Value>> {
    // py:18  def f(**kwargs):
    // py:19  r = func(**kwargs)
    // py:20  if r:
    let r = contents?;
    if r.is_empty() {
        // py:24-25  else: return None
        return None;
    }
    // py:21-23  return [{'contents': r, 'divider_highlight_group': highlight_group}]
    Some(vec![json!({
        "contents": r,
        "divider_highlight_group": highlight_group,
    })])
}

/// Port of `add_divider_highlight_group()` from
/// `powerline/lib/__init__.py:14`.
///
/// `func` is the wrapped segment function; the returned closure
/// runs it and lifts a non-empty return into a
/// `[{'contents': r, 'divider_highlight_group': hg}]` segment list.
/// Returns `None` when `func` returns an empty/None result.
pub fn add_divider_highlight_group<F>(
    highlight_group: impl Into<String>,
    mut func: F,
) -> impl FnMut() -> Option<Vec<Value>>
where
    F: FnMut() -> Option<String>,
{
    // py:15  def add_divider_highlight_group(highlight_group):
    // py:16  def dec(func):
    let hg = highlight_group.into();
    move || {
        // py:17  @wraps_saveargs(func)
        // py:18  def f(**kwargs):
        // py:19  r = func(**kwargs)
        let r = func()?;
        // py:20  if r:
        if r.is_empty() {
            // py:25  else:
            // py:26  return None
            return None;
        }
        // py:21  return [{
        // py:22  'contents': r,
        // py:23  'divider_highlight_group': highlight_group,
        // py:24  }]
        Some(vec![json!({
            "contents": r,
            "divider_highlight_group": hg,
        })])
    }
    // py:27  return f
    // py:28  return dec
}

/// Variant of `add_divider_highlight_group` that takes a closure
/// returning `Option<Vec<Value>>` (the segment's already-wrapped
/// shape) and merges the divider_highlight_group into each entry.
///
/// Mirrors the Python decorator behaviour when applied to a segment
/// that already returns a list of dicts: each entry gets the
/// `divider_highlight_group` injected if missing.
pub fn add_divider_highlight_group_to_list<F>(
    highlight_group: impl Into<String>,
    mut func: F,
) -> impl FnMut() -> Option<Vec<Value>>
where
    F: FnMut() -> Option<Vec<Value>>,
{
    let hg = highlight_group.into();
    move || {
        let segments = func()?;
        if segments.is_empty() {
            return None;
        }
        let mut out = Vec::with_capacity(segments.len());
        for mut seg in segments {
            if let Some(obj) = seg.as_object_mut() {
                obj.entry("divider_highlight_group".to_string())
                    .or_insert_with(|| Value::String(hg.clone()));
            }
            out.push(seg);
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapped_origin_exposes_both_callables() {
        let wo = WrappedOrigin::new(|| "wrapper".to_string(), || "origin".to_string());
        assert_eq!((wo.wrapper)(), "wrapper");
        assert_eq!((wo.origin)(), "origin");
    }

    #[test]
    fn wrapped_origin_accessors_match_fields() {
        let wo = WrappedOrigin::new(|| 1, || 2);
        assert_eq!((wo.wrapper())(), 1);
        assert_eq!((wo.origin())(), 2);
    }

    #[test]
    fn add_divider_highlight_group_wraps_non_empty_string() {
        // py:17-22  if r: return [{contents: r, divider_highlight_group: hg}]
        let mut wrapped =
            add_divider_highlight_group("background:divider", || Some("hello".to_string()));
        let r = wrapped().unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "hello");
        assert_eq!(r[0]["divider_highlight_group"], "background:divider");
    }

    #[test]
    fn add_divider_highlight_group_none_input_returns_none() {
        // py:23-24  else: return None
        let mut wrapped = add_divider_highlight_group("background:divider", || None::<String>);
        assert!(wrapped().is_none());
    }

    #[test]
    fn add_divider_highlight_group_empty_string_returns_none() {
        // py: empty str is falsy in Python; preserve that semantics
        let mut wrapped =
            add_divider_highlight_group("background:divider", || Some("".to_string()));
        assert!(wrapped().is_none());
    }

    #[test]
    fn add_divider_highlight_group_to_list_injects_divider_into_each_entry() {
        let mut wrapped = add_divider_highlight_group_to_list("seg:divider", || {
            Some(vec![json!({"contents": "a"}), json!({"contents": "b"})])
        });
        let r = wrapped().unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["divider_highlight_group"], "seg:divider");
        assert_eq!(r[1]["divider_highlight_group"], "seg:divider");
    }

    #[test]
    fn add_divider_highlight_group_to_list_preserves_existing_dividers() {
        // Entries that already have a divider_highlight_group should
        // not be overridden by the wrapper.
        let mut wrapped = add_divider_highlight_group_to_list("default", || {
            Some(vec![json!({
                "contents": "x",
                "divider_highlight_group": "custom",
            })])
        });
        let r = wrapped().unwrap();
        assert_eq!(r[0]["divider_highlight_group"], "custom");
    }

    #[test]
    fn add_divider_highlight_group_to_list_empty_returns_none() {
        let mut wrapped = add_divider_highlight_group_to_list("seg:divider", || Some(Vec::new()));
        assert!(wrapped().is_none());
    }

    #[test]
    fn add_divider_highlight_group_to_list_none_returns_none() {
        let mut wrapped = add_divider_highlight_group_to_list::<fn() -> Option<Vec<Value>>>(
            "seg:divider",
            || None,
        );
        assert!(wrapped().is_none());
    }

    #[test]
    fn add_divider_highlight_group_custom_highlight_string() {
        let mut wrapped = add_divider_highlight_group("my:divider", || Some("x".to_string()));
        let r = wrapped().unwrap();
        assert_eq!(r[0]["divider_highlight_group"], "my:divider");
    }

    #[test]
    fn add_divider_highlight_group_invokes_inner_func_each_call() {
        // py: each call to the wrapped function should invoke func once
        use std::cell::Cell;
        let call_count = Cell::new(0u32);
        let mut wrapped = add_divider_highlight_group("hg", || {
            call_count.set(call_count.get() + 1);
            Some("hi".to_string())
        });
        let _ = wrapped();
        let _ = wrapped();
        let _ = wrapped();
        drop(wrapped);
        assert_eq!(call_count.get(), 3);
    }

    #[test]
    fn wraps_saveargs_returns_wrapper_origin_pair() {
        // py:7-12
        let result = wraps_saveargs("original_fn", "wrapper_fn");
        assert_eq!(result.wrapper(), &"wrapper_fn");
        assert_eq!(result.origin(), &"original_fn");
    }

    #[test]
    fn dec_returns_wrapped_origin_pair() {
        // py:8-11
        let result = dec("original_fn", "wrapper_fn");
        assert_eq!(result.wrapper(), &"wrapper_fn");
        assert_eq!(result.origin(), &"original_fn");
    }

    #[test]
    fn f_returns_single_segment_list_for_non_empty_contents() {
        // py:21-23
        let r = f(Some("hello".to_string()), "branch_divider").unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "hello");
        assert_eq!(r[0]["divider_highlight_group"], "branch_divider");
    }

    #[test]
    fn f_returns_none_for_empty_contents() {
        // py:24-25
        assert!(f(None, "x").is_none());
        assert!(f(Some("".to_string()), "x").is_none());
    }
}
