// vim:fileencoding=utf-8:noet
//! Port of `powerline/matchers/__init__.py`.
//!
//! Upstream consists of:
//! ```python
//! # py:2  from __future__ import ...
//! # py:3  from pkgutil import extend_path
//! # py:6  __path__ = extend_path(__path__, __name__)
//! ```
//!
//! `extend_path` makes the package a *namespace package* so third-party
//! plugins can drop modules into `matchers/` at runtime. Rust's module
//! system has no analogue — submodules are statically declared at
//! compile time. The Rust port is therefore the bare child-module
//! declarations below; runtime plugin extension is intentionally not
//! supported by powerliners and would be a separate
//! `src/extensions/plugin_host.rs` feature.

pub mod vim;

