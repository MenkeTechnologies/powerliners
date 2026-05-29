// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/awesome/powerline-awesome.py`.
//!
//! awesome WM `powerline-awesome` script — top-level entry point that
//! parses the `interval` CLI arg and calls `run()` on the
//! `AwesomeThread` module. Upstream is a binary script invoked via
//! `python -m powerline.bindings.awesome.powerline-awesome`; the
//! Rust analog is a binary at `src/bin/powerline-awesome.rs` (TBD)
//! that delegates here.

// #!/usr/bin/env python                            // py:1
// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:3
// import sys                                       // py:5
// from powerline.bindings.wm import DEFAULT_UPDATE_INTERVAL                                // py:7
// from powerline.bindings.wm.awesome import run    // py:8

use crate::ported::bindings::wm::DEFAULT_UPDATE_INTERVAL;
use crate::ported::bindings::wm::awesome::run;

/// Port of `main()` from
/// `powerline/bindings/awesome/powerline-awesome.py:11`.
///
/// Python:
/// ```python
/// def main():
///     try:
///         interval = float(sys.argv[1])
///     except IndexError:
///         interval = DEFAULT_UPDATE_INTERVAL
///     run(interval=interval)
/// ```
pub fn main() {
    // py:12-15  interval = float(sys.argv[1]) or DEFAULT_UPDATE_INTERVAL
    let _interval = std::env::args()
        .nth(1)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(DEFAULT_UPDATE_INTERVAL);
    // py:16  run(interval=interval)
    // Note: upstream `run()` is the unported orchestrator that
    // spawns `AwesomeThread` against a `Powerline` instance.
    // The Rust port's `run()` is a stub explaining the gap.
    run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_runs_without_panic() {
        // The stubbed run() returns immediately; main() should too.
        main();
    }
}
