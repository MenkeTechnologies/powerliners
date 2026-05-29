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
}
