// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/__init__.py`.
//!
//! Vim segment registry. Upstream is 650 LOC of vim-specific segments
//! (mode indicator, file format, file encoding, line/column, etc.) and
//! the `window_cached` decorator used to cache results across renders.
//!
//! Most ports defer until the vim integration is wired. This first
//! chunk exports the child plugin modules and a stub
//! `window_cached` identity adapter so consumers compile.

pub mod plugin;

/// Port of `window_cached()` decorator from
/// `powerline/segments/vim/__init__.py:71`.
///
/// Python: caches the wrapped fn's return per window_id, returning
/// cached value when window is non-current ('nc' mode).
///
/// Rust port: identity passthrough — caching deferred until segment
/// dispatch substrate is ported. Marker fn so callers can express the
/// upstream `@window_cached` decoration intent at the call site.
pub fn window_cached<F>(func: F) -> F {
    func
}
