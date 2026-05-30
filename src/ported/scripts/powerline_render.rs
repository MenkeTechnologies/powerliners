// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/powerline-render`.
//!
//! Driver that builds a `ShellPowerline` and renders one prompt /
//! statusline string to stdout.

// #!/usr/bin/env python                              // sh:1
// from __future__ import (...)                       // sh:4
// import sys                                         // sh:6
// import os                                          // sh:7
// from powerline.shell import ShellPowerline         // sh:10
// from powerline.commands.main import get_argparser, finish_args, write_output  // sh:15
// from powerline.lib.encoding import get_unicode_writer  // sh:16
// if sys.version_info < (3,):                        // sh:19
//     write = sys.stdout.write
// else:
//     write = sys.stdout.buffer.write

use crate::ported::commands::main::{finish_args, get_argparser, Args};
use crate::ported::scripts::powerline_daemon::parse_client_argv;
use std::collections::HashMap;
use std::io::Write;

/// Port of the `if __name__ == '__main__':` block at
/// `vendor/powerline/scripts/powerline-render:25-31`.
///
/// Builds the argparser, parses args, finishes the args dict, then
/// hands off to `render_fn` for the actual `ShellPowerline` render and
/// writes the bytes to stdout. Mirrors the structure of upstream's
/// `powerline-render` script: get_argparser → parse → finish_args →
/// segment_info → write_output. The render bin supplies `render_fn`
/// the same way `powerline_daemon::main` takes a `RenderFn` callback —
/// keeps the script port free of bin-private adapter dispatch.
///
/// Returns the process exit code (0 on success; 2 when `--ext` is
/// missing, matching argparse's "required argument" rejection).
pub fn main<F>(args: &[String], render_fn: F) -> i32
where
    F: FnOnce(&Args, &HashMap<String, String>, &str) -> Vec<u8>,
{
    // sh:26  parser = get_argparser()
    let parser = get_argparser();
    let _ = parser;

    // sh:27  args = parser.parse_args()
    let mut parsed = parse_client_argv(args);
    if parsed.ext.is_empty() {
        eprintln!("powerline-render: --ext is required");
        return 2;
    }

    // sh:28  finish_args(parser, os.environ, args)
    let environ: HashMap<String, String> = std::env::vars().collect();
    let _ = finish_args(&environ, &mut parsed, false);

    // sh:29  powerline = ShellPowerline(args, run_once=True)
    // sh:30  segment_info = {'args': args, 'environ': os.environ}
    // sh:31  write_output(args, powerline, segment_info, get_unicode_writer())
    //
    // The injected `render_fn` encapsulates ShellPowerline construction +
    // write_output's render dispatch + the os.environ + cwd plumbing.
    // Mirrors the daemon's RenderFn callback architecture.
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let bytes = render_fn(&parsed, &environ, &cwd);
    let _ = std::io::stdout().lock().write_all(&bytes);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noop_render(_args: &Args, _environ: &HashMap<String, String>, _cwd: &str) -> Vec<u8> {
        Vec::new()
    }

    #[test]
    fn main_returns_2_when_ext_missing() {
        // sh:27  argparse exits 2 when required --ext absent
        assert_eq!(main(&[], noop_render), 2);
    }

    #[test]
    fn main_returns_0_when_ext_supplied_positional() {
        let r = main(&["shell".to_string()], noop_render);
        assert_eq!(r, 0);
    }

    #[test]
    fn main_passes_parsed_args_to_render_fn() {
        let captured: std::cell::RefCell<Option<Args>> = std::cell::RefCell::new(None);
        let capture = |a: &Args, _e: &HashMap<String, String>, _c: &str| -> Vec<u8> {
            *captured.borrow_mut() = Some(a.clone());
            Vec::new()
        };
        let r = main(
            &[
                "tmux".to_string(),
                "left".to_string(),
                "-w".to_string(),
                "80".to_string(),
            ],
            capture,
        );
        assert_eq!(r, 0);
        let got = captured.borrow().clone().expect("render_fn not called");
        assert_eq!(got.ext, vec!["tmux".to_string()]);
        assert_eq!(got.side.as_deref(), Some("left"));
        assert_eq!(got.width, Some(80));
    }

    #[test]
    fn main_writes_render_fn_bytes_through_to_caller_path() {
        // Smoke test: render_fn returns specific bytes — assert the
        // happy path reaches it. (Stdout capture across processes is
        // tested in integration.)
        let r = main(&["shell".to_string()], |_a, _e, _c| b"hello".to_vec());
        assert_eq!(r, 0);
    }
}
