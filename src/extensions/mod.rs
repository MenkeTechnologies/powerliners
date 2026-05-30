// vim:fileencoding=utf-8:noet
//! Features powerliners ships beyond the strict 1:1 port of
//! `powerline-status`. Sanctioned non-port location per `docs/PORT.md`.
//!
//! Currently houses:
//!
//! - [`mem_usage`] — 1:1 port of mKaloer's
//!   [`powerline_mem_segment`](https://github.com/mKaloer/powerline_mem_segment)
//!   plugin (`powerlinemem.mem_usage` Python module). Provides the 4
//!   adapter entrypoints the user's theme references by name:
//!     - `mem_usage`            → `USED/TOTAL` formatted bytes
//!     - `mem_usage_percent`    → `NN%` of total memory
//!     - `mem_swap`             → `USED/TOTAL` formatted swap bytes
//!     - `mem_swap_percentage`  → `NN%` of total swap
//!
//! These names are wired into `ADAPTERS` in
//! `src/bin/shared/render_runtime.rs` so the daemon's segment lookup
//! finds them without a Python `__import__` round-trip.

pub mod mem_usage;
