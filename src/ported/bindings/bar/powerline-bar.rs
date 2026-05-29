// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/bar/powerline-bar.py`.
//!
//! Deprecated lemonbar driver — kept for backwards compatibility per
//! upstream py:24's warning. New deployments should use
//! `bindings/lemonbar/powerline-lemonbar.py`.

// #!/usr/bin/env python                            // py:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:3
// import os                                        // py:5
// import sys                                       // py:6
// import time                                      // py:7
// from threading import Lock, Timer                // py:9
// from argparse import ArgumentParser              // py:10
// from powerline.lemonbar import LemonbarPowerline                                          // py:12
// from powerline.lib.encoding import get_unicode_writer                                     // py:13
// from powerline.bindings.wm import DEFAULT_UPDATE_INTERVAL                                  // py:14

use crate::ported::bindings::wm::DEFAULT_UPDATE_INTERVAL;
use crate::ported::commands::lint::{ArgAction, ArgParser, Argument};

/// Returns the argument parser for the deprecated `powerline-bar` binary.
///
/// Inlined argparse setup from py:17-22 — single `--i3` flag.
pub fn get_argparser() -> ArgParser {
    ArgParser {
        description: "Powerline lemonbar bindings.".to_string(), // py:17
        arguments: vec![
            // py:18-21  --i3
            Argument {
                flags: vec!["--i3".into()],
                action: ArgAction::StoreTrue,
                metavar: None,
                help: "Subscribe for i3 events.".to_string(),
            },
        ],
    }
}

/// Port of the inner `render()` closure from
/// `powerline/bindings/bar/powerline-bar.py:31-39`.
///
/// Calls `powerline.render(mode=modes[0])` and emits the result +
/// newline to stdout. Python uses `Timer(DEFAULT_UPDATE_INTERVAL,
/// render, kwargs={'reschedule': True}).start()` at py:32-33 to
/// reschedule itself; the Rust port returns the encoded bytes and
/// leaves the scheduling to the caller (real-binary loop).
///
/// `render_fn` is the caller-supplied closure that maps a mode
/// string to the rendered statusline.
pub fn render<F>(mode: &str, render_fn: F) -> Vec<u8>
where
    F: FnOnce(&str) -> String,
{
    // py:31  def render(reschedule=False):
    // py:32-33  if reschedule: Timer(...).start()  (caller-side)
    // py:37  write(powerline.render(mode=modes[0]))
    let rendered = render_fn(mode);
    let mut buf = rendered.into_bytes();
    // py:38  write('\n')
    buf.push(b'\n');
    buf
}

/// Port of the inner `update()` closure from
/// `powerline/bindings/bar/powerline-bar.py:41-43`.
///
/// Stores the new mode (i3 `evt.change` payload) into the shared
/// `modes[0]` slot and re-renders. Python captures `modes` + render
/// from the outer scope; the Rust port takes the slot as `&mut`
/// and dispatches through the caller-supplied render closure.
pub fn update<F>(modes_slot: &mut String, new_mode: &str, render_fn: F) -> Vec<u8>
where
    F: FnOnce(&str) -> String,
{
    // py:41  def update(evt):
    // py:42  modes[0] = evt.change
    *modes_slot = new_mode.to_string();
    // py:43  render()
    render(modes_slot, render_fn)
}

/// Port of the `if __name__ == '__main__':` block from
/// `powerline/bindings/bar/powerline-bar.py:16`.
///
/// **Status:** stub. Powerline + lemonbar dispatch deferred.
/// Emits the upstream deprecation warning so callers know to migrate.
pub fn main() {
    // py:17-22  argparser
    let _parser = get_argparser();

    // py:23  powerline = LemonbarPowerline(); update_renderer()
    // (Powerline class not ported — skipped.)

    // py:24  pl.warn("The 'bar' bindings are deprecated, please switch to 'lemonbar'")
    eprintln!(
        "powerline-bar: The 'bar' bindings are deprecated, please switch to 'lemonbar'"
    );

    // py:27-45  Timer-based render loop + i3-ipc subscription — deferred.
    let _interval = DEFAULT_UPDATE_INTERVAL;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argparser_has_only_i3_flag() {
        let p = get_argparser();
        assert_eq!(p.description, "Powerline lemonbar bindings.");
        assert_eq!(p.arguments.len(), 1);
        assert_eq!(p.arguments[0].flags, vec!["--i3".to_string()]);
        assert_eq!(p.arguments[0].action, ArgAction::StoreTrue);
    }

    #[test]
    fn render_appends_newline_to_render_fn_output() {
        // py:37-38  write(powerline.render(mode)); write('\n')
        let out = render("vis", |m| format!("statusline[{m}]"));
        assert_eq!(out, b"statusline[vis]\n");
    }

    #[test]
    fn update_sets_mode_then_renders() {
        // py:42-43
        let mut mode = "default".to_string();
        let out = update(&mut mode, "resize", |m| format!("M={m}"));
        assert_eq!(mode, "resize");
        assert_eq!(out, b"M=resize\n");
    }
}
