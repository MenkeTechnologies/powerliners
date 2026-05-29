// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/__init__.py`.
//!
//! Upstream source in full:
//!
//! ```python
//! # vim:fileencoding=utf-8:noet
//! from __future__ import (unicode_literals, division, absolute_import, print_function)
//!
//! from functools import wraps
//!
//! def wraps_saveargs(wrapped):
//!     def dec(wrapper):
//!         r = wraps(wrapped)(wrapper)
//!         r.powerline_origin = getattr(wrapped, 'powerline_origin', wrapped)
//!         return r
//!     return dec
//!
//! def add_divider_highlight_group(highlight_group):
//!     def dec(func):
//!         @wraps_saveargs(func)
//!         def f(**kwargs):
//!             r = func(**kwargs)
//!             if r:
//!                 return [{
//!                     'contents': r,
//!                     'divider_highlight_group': highlight_group,
//!                 }]
//!             else:
//!                 return None
//!         return f
//!     return dec
//! ```
//!
//! Both fns are Python decorator factories. `wraps_saveargs` preserves
//! `__wrapped__`-style metadata plus a `powerline_origin` attribute
//! pointing at the original (unwrapped) callable; this is read by the
//! segment introspection machinery in `segment.py` /
//! `lint/inspect.py`. `add_divider_highlight_group` wraps a segment
//! function so its return value is automatically lifted into the
//! single-segment list shape `[{'contents': r, 'divider_highlight_group': hg}]`.
//!
//! Rust port deferred: Python decorators rebind module-level names at
//! import time, attaching attributes via `setattr`. Rust has no
//! runtime attribute attachment on `fn` items. A faithful port maps
//! these to:
//! 1. `wraps_saveargs` → a marker struct + `Fn` trait wrapper that
//!    carries the `powerline_origin` pointer as a struct field.
//! 2. `add_divider_highlight_group` → a higher-order fn returning a
//!    closure that wraps an inner segment fn and post-processes its
//!    output into the divider-highlight-group list shape.
//!
//! Both depend on the `Segment` trait / dispatch shape that lives in
//! `segment.rs` (currently a scaffold stub). They'll port together
//! when segment.rs lands; for now this module is the bare child-module
//! declarations.

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
