// vim:fileencoding=utf-8:noet
//! Shell-side bindings for the Reactive Prompt Push extension
//! ([`crate::extensions::watch`]). Sanctioned non-port location per
//! `docs/PORT.md`.
//!
//! The bindings are shipped as compiled-in source strings via
//! `include_str!` so tooling can emit them (`powerline-config` help,
//! docs, an install snippet) without a separate data file to locate at
//! runtime. Each binding creates the per-client wake FIFO, exports its
//! path as `POWERLINE_RESET_FIFO` (which the shell then passes into every
//! `powerline` invocation and thus to the daemon's `render_fn`), and
//! wires the read end into the line editor so a one-byte wake triggers an
//! in-place prompt redraw.
//!
//! Only zsh is provided today: it is the one mainstream shell whose line
//! editor (ZLE) can watch an arbitrary fd (`zle -F`) and redraw the
//! prompt mid-line (`zle reset-prompt`). bash's `readline` has no public
//! equivalent, so a bash binding would have to fake it with `PROMPT_COMMAND`
//! polling — which defeats the whole point (edge-triggered, no timer).

/// zsh binding source: FIFO + `zle -F` wake fd → `zle reset-prompt`.
pub const ZSH_REACTIVE: &str = include_str!("reactive.zsh");

/// Return the zsh reactive-push binding source, ready to `source`.
pub fn zsh_reactive() -> &'static str {
    ZSH_REACTIVE
}
