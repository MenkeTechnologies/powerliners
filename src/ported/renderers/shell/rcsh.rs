// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/shell/rcsh.py`.
//!
//! Upstream source in full:
//!
//! ```python
//! # vim:fileencoding=utf-8:noet
//! from __future__ import (unicode_literals, division, absolute_import, print_function)
//!
//! from powerline.renderers.shell.readline import ReadlineRenderer
//!
//! renderer = ReadlineRenderer
//! ```
//!
//! `rc` shell uses the same escape conventions as readline. Port is a
//! thin alias pointing at `ReadlineRenderer`.
//!
//! **Status:** deferred — `ReadlineRenderer` lives in `readline.rs`
//! which depends on `ShellRenderer` → `Renderer` → orchestrator
//! stack. The `pub type renderer = ReadlineRenderer;` alias will be
//! restored once `readline.rs` is ported (PORT_CHECKLIST Phase 2).
