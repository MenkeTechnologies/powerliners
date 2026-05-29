// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/vim/plugin/__init__.py`.
//!
//! Upstream is the `extend_path` namespace-package pattern (see
//! `matchers/mod.rs` for the explanation). Rust port carries the
//! static child-module declarations only.

pub mod ale;
pub mod capslock;
pub mod coc;
pub mod commandt;
pub mod nerdtree;
pub mod syntastic;
pub mod tagbar;

