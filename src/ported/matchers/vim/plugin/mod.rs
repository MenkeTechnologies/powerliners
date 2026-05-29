// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/vim/plugin/__init__.py`.
//!
//! Upstream is the `extend_path` namespace-package pattern (see
//! `matchers/mod.rs` for the explanation). Rust port carries the
//! static child-module declarations only.

pub mod commandt;
pub mod gundo;
pub mod nerdtree;

