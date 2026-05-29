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
}
