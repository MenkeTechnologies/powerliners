// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/lemonbar/powerline-lemonbar.py`.
//!
//! lemonbar driver script: spawns one `lemonbar` subprocess per
//! connected xrandr output, then re-renders the statusline at the
//! configured interval and pipes it into each lemonbar's stdin.
//!
//! Upstream is a Python binary script (`__main__`). The Rust analog
//! is a binary at `src/bin/powerline-lemonbar.rs` (TBD). This module
//! exports the assembly pieces the future binary will use.

// #!/usr/bin/env python                            // py:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:3
// import time                                      // py:5
// import re                                        // py:6
// import subprocess                                // py:7
// from threading import Lock, Timer                // py:9
// from powerline.lemonbar import LemonbarPowerline                                          // py:11
// from powerline.commands.lemonbar import get_argparser                                     // py:12
// from powerline.bindings.wm import get_connected_xrandr_outputs                            // py:13

use crate::ported::bindings::wm::{get_connected_xrandr_outputs, XrandrOutput};
use crate::ported::commands::lemonbar::get_argparser;

/// One lemonbar instance: the connected output it shows, the spawned
/// subprocess writing into it, and the per-screen segment width
/// (`width / 5` per upstream py:23).
pub struct LemonbarInstance {
    pub screen_name: String,
    pub width: i64,
    // Process handle: in the real binary this is a `Child` with
    // captured stdin. The library-side carries the metadata only.
}

/// Build the lemonbar command line for one xrandr output.
///
/// Mirrors py:21:
/// ```python
/// command = [args.bar_command, '-g', '{0}x{1}+{2}+{3}'.format(
///     screen['width'], args.height, screen['x'], screen['y']
/// )] + args.args[1:]
/// ```
pub fn build_bar_command(
    bar_command: &str,
    screen: &XrandrOutput,
    height: &str,
    extra: &[String],
) -> Vec<String> {
    let mut cmd = vec![
        bar_command.to_string(),
        "-g".to_string(),
        format!("{}x{}+{}+{}", screen.width, height, screen.x, screen.y),
    ];
    cmd.extend_from_slice(extra);
    cmd
}

/// Port of the inner `render()` closure from
/// `powerline/bindings/lemonbar/powerline-lemonbar.py:32-40`.
///
/// For each `(output, process, width)` triple in `bars`, calls
/// `powerline.render(mode, width, matcher_info=output)` and writes
/// the result + `\n` into the lemonbar process's stdin.
///
/// Python uses `Timer` for `reschedule=True` to call itself again
/// after `args.interval` seconds (py:33-34); the Rust port returns
/// the encoded per-bar bytes and leaves the timer wiring + stdin
/// flush to the caller (real-binary loop).
///
/// `mode` is `modes[0]` from the outer scope; `render_fn` is the
/// caller-supplied closure that maps `(mode, width, matcher_info)`
/// to the rendered statusline string.
pub fn render<F>(
    bars: &[(String, i64)], // (matcher_info, width) per screen
    mode: &str,
    mut render_fn: F,
) -> Vec<(String, Vec<u8>)>
where
    F: FnMut(&str, i64, &str) -> String,
{
    // py:32  def render(reschedule=False):
    // py:33-34  if reschedule: Timer(args.interval, render, kwargs={'reschedule': True}).start()
    // (timer scheduling lives in the binary driver)
    // py:36-40  for output, process, width in bars: write(powerline.render(...))
    let mut out: Vec<(String, Vec<u8>)> = Vec::with_capacity(bars.len());
    for (output, width) in bars {
        let rendered = render_fn(mode, *width, output);
        let mut buf = rendered.into_bytes();
        buf.push(b'\n');
        out.push((output.clone(), buf));
    }
    out
}

/// Port of the inner `update()` closure from
/// `powerline/bindings/lemonbar/powerline-lemonbar.py:42-44`.
///
/// Stores the new mode (`evt.change` in the i3-ipc event payload)
/// into the shared `modes[0]` slot and re-runs `render()`.
///
/// Python captures `modes` and `render` from the outer scope; the
/// Rust port takes the mode slot as `&mut String` and returns the
/// rendered output (callers route through `render()` themselves).
pub fn update<F>(
    modes_slot: &mut String,
    new_mode: &str,
    bars: &[(String, i64)],
    render_fn: F,
) -> Vec<(String, Vec<u8>)>
where
    F: FnMut(&str, i64, &str) -> String,
{
    // py:42  def update(evt):
    // py:43  modes[0] = evt.change
    *modes_slot = new_mode.to_string();
    // py:44  render()
    render(bars, modes_slot, render_fn)
}

/// Port of the `if __name__ == '__main__':` block from
/// `powerline/bindings/lemonbar/powerline-lemonbar.py:16`.
///
/// Status: stub. Powerline.render() + i3-ipc loop deferred until
/// orchestrator + i3 integration lands. Returns immediately after
/// printing the arg-spec it would have parsed.
pub fn main() {
    // py:17-18  parser = get_argparser(); args = parser.parse_args()
    let parser = get_argparser();
    let _ = parser; // arg-spec is data-only without a CLI runner

    // py:20  powerline = LemonbarPowerline(); powerline.update_renderer()
    // (Powerline class not ported — skipped.)

    // py:22-25  for screen in get_connected_xrandr_outputs(pl):
    //              build command, spawn process, track instance
    let _outputs = get_connected_xrandr_outputs(&());

    // py:27-46  Timer + render + i3-ipc loop — deferred.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_screen() -> XrandrOutput {
        XrandrOutput {
            name: "HDMI-1".to_string(),
            primary: true,
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
        }
    }

    #[test]
    fn build_bar_command_assembles_geometry() {
        let s = fake_screen();
        let cmd = build_bar_command("lemonbar", &s, "24", &[]);
        assert_eq!(cmd, vec![
            "lemonbar".to_string(),
            "-g".to_string(),
            "1920x24+0+0".to_string(),
        ]);
    }

    #[test]
    fn build_bar_command_appends_extra_args() {
        let s = fake_screen();
        let extra = vec!["-B".to_string(), "#000000".to_string()];
        let cmd = build_bar_command("lemonbar", &s, "30", &extra);
        assert_eq!(cmd, vec![
            "lemonbar".to_string(),
            "-g".to_string(),
            "1920x30+0+0".to_string(),
            "-B".to_string(),
            "#000000".to_string(),
        ]);
    }

    #[test]
    fn render_emits_one_payload_per_bar_with_newline() {
        // py:39-40  process.stdin.write(rendered + b'\n')
        let bars = vec![
            ("HDMI-1".to_string(), 100_i64),
            ("DP-1".to_string(), 200_i64),
        ];
        let out = render(&bars, "default", |mode, width, matcher| {
            format!("[{mode}:{width}:{matcher}]")
        });
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].0, "HDMI-1");
        assert_eq!(out[0].1, b"[default:100:HDMI-1]\n");
        assert_eq!(out[1].0, "DP-1");
        assert_eq!(out[1].1, b"[default:200:DP-1]\n");
    }

    #[test]
    fn update_sets_new_mode_and_renders() {
        // py:43-44  modes[0] = evt.change; render()
        let mut mode = "default".to_string();
        let bars = vec![("X".to_string(), 50_i64)];
        let out = update(&mut mode, "resize", &bars, |m, w, mi| {
            format!("m={m},w={w},mi={mi}")
        });
        assert_eq!(mode, "resize");
        assert_eq!(out[0].1, b"m=resize,w=50,mi=X\n");
    }
}
