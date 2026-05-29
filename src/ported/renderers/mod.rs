// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/__init__.py`.
//!
//! Upstream is a 0-byte namespace package marker (Python `__init__.py`
//! must exist to make `renderers/` importable). No Python definitions
//! to port — this module file exists only to declare child renderer
//! modules per the package layout.

pub mod i3bar;
pub mod ipython;
pub mod lemonbar;
pub mod pango_markup;
pub mod pdb;
pub mod shell;
pub mod tmux;
pub mod vim;

