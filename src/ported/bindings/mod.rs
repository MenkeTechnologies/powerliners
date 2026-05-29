// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/__init__.py`.
//!
//! Upstream is a 0-byte namespace package marker. No Python definitions
//! to port — this module file exists only to declare child binding
//! modules per the package layout.

pub mod config;
pub mod ipython;
pub mod pdb;
pub mod qtile;
pub mod tmux;
pub mod vim;
pub mod wm;
pub mod zsh;
