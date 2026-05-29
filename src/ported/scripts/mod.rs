// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/`.
//!
//! Mirror of the five entry-point scripts that live OUTSIDE the
//! `powerline` Python package (they're standalone executables at the
//! repo root in upstream). Each script becomes a Rust module here +
//! a thin shim under `src/bin/` that calls into `main()`.
//!
//! - `powerline-config`  → [`powerline_config::main`]
//! - `powerline-lint`    → [`powerline_lint::main`]
//! - `powerline-render`  → [`powerline_render::main`]
//! - `powerline-daemon`  → [`powerline_daemon::main`]
//! - `powerline-release` → [`powerline_release::main`]
//!
//! Because `scripts/extract_py_names.py` only walks the `powerline`
//! Python package (it skips `vendor/powerline/scripts/`), every fn
//! ported here is flagged by the drift gate. Such names land in
//! `tests/data/fake_fn_allowlist.txt` with a rationale citing the
//! exact `scripts/<name>:LINE` source.

pub mod powerline_config;
pub mod powerline_daemon;
pub mod powerline_lint;
pub mod powerline_release;
pub mod powerline_render;
