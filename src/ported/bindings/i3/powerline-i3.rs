// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/i3/powerline-i3.py`.
//!
//! i3bar protocol driver: prints the i3bar JSON header, subscribes to
//! `workspace` events, and re-renders on each event at a configurable
//! interval.
//!
//! Upstream is a binary script invoked via
//! `python -m powerline.bindings.i3.powerline-i3 [name]`. The Rust
//! analog is `src/bin/powerline-i3.rs` (TBD). This module exports the
//! pieces a future binary will assemble.

// #!/usr/bin/env python                            // py:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:3
// import sys                                       // py:5
// import time                                      // py:6
// from threading import Lock                       // py:8
// from powerline.bindings.wm import get_i3_connection, i3_subscribe                       // py:10
// from powerline import Powerline                  // py:12
// from powerline.lib.monotonic import monotonic    // py:13

/// Port of `class I3Powerline(Powerline)` from
/// `powerline/bindings/i3/powerline-i3.py:16`.
///
/// Currently only changes the default log target.
pub struct I3Powerline;

impl I3Powerline {
    /// Port of `I3Powerline.default_log_stream` class attribute from
    /// `powerline/bindings/i3/powerline-i3.py:21`.
    ///
    /// Python: `default_log_stream = sys.stderr` — overrides the
    /// Powerline base default of `sys.stdout` because i3 reads the
    /// powerline-i3 stdout for the JSON protocol; logs must go
    /// elsewhere.
    pub const default_log_stream: &'static str = "stderr";
}

/// Port of the inner `render()` closure from
/// `powerline/bindings/i3/powerline-i3.py:40-44`.
///
/// Returns the i3bar JSON output line for one frame:
/// `,[<powerline.render()[:-1]>]` per py:43.
///
/// Python's `powerline.render()` returns the full segment array
/// JSON-encoded with a trailing newline; the `[:-1]` slice strips
/// the newline before wrapping in `,[ ... ]`. The Rust port takes
/// the rendered string as a closure result so callers route through
/// their own Powerline instance.
pub fn render<F>(render_fn: F) -> String
where
    F: FnOnce() -> String,
{
    // py:40  def render(event=None, data=None, sub=None):
    // py:41-42  global lock; with lock:
    // py:43  print (',[' + powerline.render()[:-1] + ']')
    let rendered = render_fn();
    // Python's [:-1] strips the trailing newline; mirror with trim_end.
    let trimmed = rendered.strip_suffix('\n').unwrap_or(&rendered);
    format!(",[{}]", trimmed)
}

/// Port of the `if __name__ == '__main__':` block from
/// `powerline/bindings/i3/powerline-i3.py:24`.
///
/// Prints the i3bar JSON header (`{"version": 1}`, `[`, `[]`),
/// subscribes to workspace events via `i3_subscribe`, then loops
/// rendering at the requested interval.
///
/// **Status:** stub — depends on `Powerline.render()` which requires
/// the orchestrator + renderer stack. Returns immediately, printing
/// the JSON header so callers see the expected protocol prefix even
/// without a render loop.
pub fn main() {
    use crate::ported::bindings::wm::{get_i3_connection, i3_subscribe};

    let _name = std::env::args().nth(1).unwrap_or_else(|| "wm".to_string()); // py:25-27

    // py:29-30  I3Powerline(name, renderer_module='i3bar') + update_renderer()
    // (Powerline class not yet ported — skipped.)

    // py:32  interval = 0.5  (DEFAULT_UPDATE_INTERVAL)
    let _interval = 0.5_f64;

    // py:34-36  i3bar JSON protocol header
    println!("{{\"version\": 1}}");                  // py:34
    println!("[");                                   // py:35
    println!("[]");                                  // py:36

    // py:38  lock = Lock()  (no actual lock needed in stub)

    // py:40-44  def render(event=None, data=None, sub=None): print rendered
    // (Powerline.render() unported — render() body skipped.)

    // py:46  i3 = get_i3_connection()
    get_i3_connection();
    // py:47  i3_subscribe(i3, 'workspace', render)
    i3_subscribe(&(), "workspace", || {});

    // py:49-52  while True: start_time = monotonic(); render(); time.sleep(...)
    // Stub: don't enter the infinite loop. A future binary will replace
    // this with the real driver.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i3powerline_log_stream_is_stderr() {
        assert_eq!(I3Powerline::default_log_stream, "stderr");
    }

    #[test]
    fn render_strips_trailing_newline_and_wraps_with_array_comma() {
        // py:43  ',[' + powerline.render()[:-1] + ']'
        let out = render(|| "[seg1, seg2]\n".to_string());
        assert_eq!(out, ",[[seg1, seg2]]");
    }

    #[test]
    fn render_passes_through_when_no_trailing_newline() {
        let out = render(|| "[seg]".to_string());
        assert_eq!(out, ",[[seg]]");
    }
}
