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
}
