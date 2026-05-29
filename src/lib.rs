// vim:fileencoding=utf-8:noet
//! # powerliners — 1:1 Rust port of [powerline/powerline](https://github.com/powerline/powerline).
//!
//! See `docs/PORT.md` for the full porting doctrine. Summary:
//!
//! - `src/ported/` mirrors `vendor/powerline/powerline/` 1:1 at file, name,
//!   signature, control-flow, and comment level.
//! - Every `fn` carries `/// Port of <name>() from powerline/<file>.py:<line>`.
//! - Every Rust statement that ports a Python statement carries `// py:NNN`.
//! - Every Python `#` comment and `"""..."""` docstring carries over.
//! - `src/extensions/` is the ONLY non-port location, reserved for features
//!   powerline-status does not have (Cranelift JIT, persistent workers, etc.).
//!
//! The crate root re-exports `pub use ported::*;` so external call sites
//! (binaries, integration tests) can reference symbols by their Python name
//! without an extra `ported::` qualifier.

pub mod ported;

// Re-export the entire ported tree for ergonomic call sites.
// (Mirrors the way `powerline/__init__.py` re-exports its public API.)
pub use crate::ported::*;
