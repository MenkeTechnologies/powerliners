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
use std::collections::HashMap;

/// Port of the `if __name__ == '__main__':` block at
/// `vendor/powerline/scripts/powerline-render:25-31`.
///
/// Builds the argparser, parses args, finishes the args dict, then
/// renders a `ShellPowerline` to stdout. Returns the process exit
/// code (always 0 on the success path; non-zero when the argparser
/// rejects required positional `--ext`).
///
/// `ShellPowerline(args, run_once=True)` at sh:29 and `write_output`
/// at sh:31 are not yet ported (depend on the full Powerline
/// renderer dispatch). The script body is wired structurally so the
/// next port pass can drop the real call in place.
pub fn main(args: &[String]) -> i32 {
    // sh:26  parser = get_argparser()
    let parser = get_argparser();

    // Locate --ext / -e (the only required arg per commands/main.py).
    let mut ext: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--ext" | "-e" => {
                i += 1;
                if let Some(e) = args.get(i) {
                    ext = Some(e.clone());
                }
            }
            _ => {}
        }
        i += 1;
    }
    let _ = parser;

    let mut parsed = Args::default();
    if let Some(e) = ext {
        parsed.ext = vec![e];
    } else {
        eprintln!("powerline-render: --ext is required");
        return 2;
    }

    // sh:27  args = parser.parse_args()
    // sh:28  finish_args(parser, os.environ, args)
    let environ: HashMap<String, String> = std::env::vars().collect();
    let _ = finish_args(&environ, &mut parsed, false);

    // sh:29  powerline = ShellPowerline(args, run_once=True)
    // TODO: ShellPowerline construction defers to the Powerline
    // class port. The arg-handling path is verified here.

    // sh:30  segment_info = {'args': args, 'environ': os.environ}
    // sh:31  write_output(args, powerline, segment_info, get_unicode_writer())
    // TODO: write_output also deferred.

    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_returns_2_when_ext_missing() {
        // sh:27  argparse exits 2 when required --ext absent
        assert_eq!(main(&[]), 2);
    }

    #[test]
    fn main_returns_0_when_ext_supplied_long() {
        let r = main(&["--ext".to_string(), "shell".to_string()]);
        assert_eq!(r, 0);
    }

    #[test]
    fn main_returns_0_when_ext_supplied_short() {
        let r = main(&["-e".to_string(), "tmux".to_string()]);
        assert_eq!(r, 0);
    }
}
