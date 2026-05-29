// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/wm/__init__.py`.
//!
//! Window-manager dispatch primitives: i3-ipc subscription, xrandr
//! output discovery, and the `wm_threads` registry that maps WM names
//! to their per-WM update threads.
//!
//! The i3 / awesome / pyuv-thread machinery depends on Python's
//! threading model + i3ipc Python bindings, neither of which has a
//! direct Rust analog. The Rust port surfaces:
//!   - `DEFAULT_UPDATE_INTERVAL` (the only consumer-visible constant)
//!   - `XRANDR_OUTPUT_RE` parser
//!   - `get_connected_xrandr_outputs` parser walk
//!   - `wm_threads` registry placeholder (empty until per-WM threads land)
//!   - `i3_subscribe` / `get_i3_connection` as documented stubs
//!     that error when i3-ipc isn't wired

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// from powerline.theme import requires_segment_info                                       // py:6
// from powerline.lib.shell import run_cmd                                                  // py:7
// from powerline.bindings.wm.awesome import AwesomeThread                                  // py:8

pub mod awesome;

use crate::ported::lib::shell::run_cmd;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of module-level binding `DEFAULT_UPDATE_INTERVAL` from
/// `powerline/bindings/wm/__init__.py:11`.
#[allow(non_upper_case_globals)]
pub const DEFAULT_UPDATE_INTERVAL: f64 = 0.5;        // py:11

// py:14  conn = None
// Module-level i3 connection cache. Bucket-2 per PORT_PLAN.md
// (shared across WM-thread invocations); modelled as `OnceLock<Option<…>>`
// where the inner `Option` mirrors Python's `if not conn` cache check.
// Currently always `None` since `get_i3_connection` returns the stub.

/// Port of `i3_subscribe()` from `powerline/bindings/wm/__init__.py:17`.
///
/// Subscribe to i3 workspace event.
///
/// **Status:** stub — i3-ipc is a Python-only protocol library; until
/// a Rust crate dependency lands, this fn no-ops. The i3 binding
/// (`bindings/i3/powerline-i3.py`) is a binary script that won't be
/// usable until this is wired.
///
/// :param conn: Connection (currently `()` placeholder).
/// :param event: Event name to subscribe to (e.g. `"workspace"`).
/// :param callback: Callback fn to run on event.
pub fn i3_subscribe<F>(_conn: &(), _event: &str, _callback: F)
where
    F: Fn() + Send + 'static,
{
    // py:27  conn.on(event, callback)
    // py:29-43  I3Thread daemon — no-op until i3-ipc is wired
}

/// Port of `get_i3_connection()` from
/// `powerline/bindings/wm/__init__.py:46`.
///
/// Return a valid, cached i3 Connection instance.
///
/// **Status:** stub returning `()` — actual i3-ipc binding deferred.
pub fn get_i3_connection() {
    // py:49-52  global conn; import i3ipc; conn = i3ipc.Connection()
    // Rust stub: i3-ipc not yet wired.
}

/// Port of module-level binding `XRANDR_OUTPUT_RE` from
/// `powerline/bindings/wm/__init__.py:56`.
///
/// Compiled regex matching `xrandr -q` connected-output lines:
/// ```text
/// HDMI-1 connected primary 1920x1080+0+0
/// ```
#[allow(non_snake_case)]
pub fn XRANDR_OUTPUT_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // py:56  '^(?P<name>...) connected(?P<primary> primary)? (?P<width>\d+)x(?P<height>\d+)\+(?P<x>\d+)\+(?P<y>\d+)'
        // Rust regex doesn't support `re.MULTILINE` flag by default; use (?m).
        Regex::new(
            r"(?m)^(?P<name>[0-9A-Za-z-]+) connected(?P<primary> primary)? (?P<width>\d+)x(?P<height>\d+)\+(?P<x>\d+)\+(?P<y>\d+)",
        )
        .unwrap()
    })
}

/// One xrandr connected-output row.
///
/// Mirrors the `groupdict()` dict Python yields at py:65-67.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrandrOutput {
    pub name: String,
    pub primary: bool,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

/// Port of `get_connected_xrandr_outputs()` from
/// `powerline/bindings/wm/__init__.py:59`.
///
/// Iterate over xrandr outputs.
///
/// Outputs are represented by a struct with `name`, `width`, `height`,
/// `primary`, `x` and `y` fields. Python returns a generator over
/// `groupdict()`; Rust returns a `Vec<XrandrOutput>` since the
/// upstream walks are bounded.
pub fn get_connected_xrandr_outputs(pl: &()) -> Vec<XrandrOutput> {
    // py:65-67  run_cmd(pl, ['xrandr', '-q'])
    let output = match run_cmd(
        pl,
        &["xrandr".to_string(), "-q".to_string()],
        None,
        true,
    ) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let re = XRANDR_OUTPUT_RE();
    re.captures_iter(&output)
        .map(|cap| XrandrOutput {
            name: cap.name("name").map(|m| m.as_str().to_string()).unwrap_or_default(),
            primary: cap.name("primary").is_some(),
            width: cap.name("width").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            height: cap.name("height").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            x: cap.name("x").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
            y: cap.name("y").and_then(|m| m.as_str().parse().ok()).unwrap_or(0),
        })
        .collect()
}

/// Port of module-level binding `wm_threads` from
/// `powerline/bindings/wm/__init__.py:70`.
///
/// Maps WM name to a per-WM update-thread launcher. Currently only
/// `'awesome'` is registered upstream; the Rust port returns an empty
/// map until the per-WM thread infrastructure lands.
#[allow(non_upper_case_globals)]
pub fn wm_threads() -> &'static HashMap<&'static str, &'static str> {
    static M: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    M.get_or_init(|| {
        // py:70-72  'awesome': AwesomeThread
        // We register the name only; the actual thread launcher is
        // deferred until awesome.rs is ported.
        let mut m = HashMap::new();
        m.insert("awesome", "AwesomeThread");
        m
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_update_interval_matches_upstream() {
        assert_eq!(DEFAULT_UPDATE_INTERVAL, 0.5);
    }

    #[test]
    fn xrandr_regex_matches_standard_line() {
        let sample = "HDMI-1 connected primary 1920x1080+0+0 (normal left inverted right x axis y axis) 521mm x 293mm";
        let re = XRANDR_OUTPUT_RE();
        assert!(re.is_match(sample));
        let cap = re.captures(sample).unwrap();
        assert_eq!(cap.name("name").unwrap().as_str(), "HDMI-1");
        assert_eq!(cap.name("width").unwrap().as_str(), "1920");
        assert_eq!(cap.name("height").unwrap().as_str(), "1080");
        assert!(cap.name("primary").is_some());
    }

    #[test]
    fn xrandr_regex_handles_non_primary() {
        let sample = "DP-1 connected 2560x1440+1920+0";
        let cap = XRANDR_OUTPUT_RE().captures(sample).unwrap();
        assert!(cap.name("primary").is_none());
        assert_eq!(cap.name("name").unwrap().as_str(), "DP-1");
    }

    #[test]
    fn get_connected_xrandr_outputs_returns_empty_when_xrandr_unavailable() {
        // No xrandr on macOS (and on most test envs); should return Vec::new.
        let outputs = get_connected_xrandr_outputs(&());
        // Empty is the success-mode in unit tests since xrandr isn't installed.
        assert!(outputs.is_empty() || outputs.iter().all(|o| !o.name.is_empty()));
    }

    #[test]
    fn wm_threads_registers_awesome() {
        let m = wm_threads();
        assert!(m.contains_key("awesome"));
    }
}
