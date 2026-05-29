// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/bat.py`.
//!
//! Battery segment. Surfaces:
//!   - `BATTERY_PERCENT_RE` regex used by the pmset path (py:146)
//!   - `parse_pmset_output(text)` extracts (percent, ac_charging)
//!     from the macOS `pmset -g batt` output
//!   - `parse_linux_status(text)` parses the `/sys/class/power_supply/
//!     {dev}/status` value into the boolean state flag (`!= "Discharging"`)
//!   - `flatten_battery(devices)` aggregates per-device energy /
//!     energy_full into the percent + state pair
//!   - `battery(get_status, ...)` segment builder with both the
//!     percent-format and gamify-heart paths
//!
//! Live dbus/UPower, /sys/class/power_supply, pmset, win32com,
//! GetSystemPowerStatus dispatch paths are stubbed since they need
//! external bindings; the pure parsing + aggregation pieces are
//! surfaced for unit testing.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import sys                                       // py:5
// import re                                        // py:6
// from powerline.lib.shell import run_cmd          // py:8

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Port of `BATTERY_PERCENT_RE` from
/// `powerline/segments/common/bat.py:146`.
#[allow(non_snake_case)]
pub fn BATTERY_PERCENT_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(\d+)%").unwrap())
}

/// Parses `pmset -g batt` output into `(percent, ac_charging)`.
/// Mirrors py:148-151:
///   battery_percent = BATTERY_PERCENT_RE.search(output).group(1)
///   ac_charging = 'AC' in output
///   return int(percent), ac_charging
pub fn parse_pmset_output(text: &str) -> Option<(u8, bool)> {
    // py:149  BATTERY_PERCENT_RE.search(output).group(1)
    let caps = BATTERY_PERCENT_RE().captures(text)?;
    let percent: u8 = caps.get(1)?.as_str().parse().ok()?;
    // py:150  ac_charging = 'AC' in battery_summary
    let ac_charging = text.contains("AC");
    Some((percent, ac_charging))
}

/// Parses `/sys/class/power_supply/{dev}/status` into the boolean
/// state flag matching py:111:
///   `state &= (line.strip() != 'Discharging')`
///
/// The Python source ANDs the per-device flag into an accumulator;
/// the standalone parser returns true when the device is not
/// discharging.
pub fn parse_linux_status(text: &str) -> bool {
    // py:111  line.strip() != 'Discharging'
    text.trim() != "Discharging"
}

/// Aggregates per-device energy + energy_full + status into the
/// `(percent, ac_state)` pair returned by the DBUS+UPower and
/// /sys/class/power_supply paths (py:53-77 and py:101-114).
///
/// `devices` is a slice of `(energy, energy_full, state_bool)`
/// tuples. `state` accumulator starts at true and ANDs each
/// device's state.
pub fn flatten_battery(devices: &[(f64, f64, bool)]) -> (f64, bool) {
    // py:54-72  iterate energy + energy_full + state
    let mut total_energy = 0.0;
    let mut total_full = 0.0;
    let mut state = true;
    for (e, f, s) in devices {
        total_energy += e;
        total_full += f;
        state &= s;
    }
    // py:74-77  if energy_full > 0: return (energy * 100 / energy_full)
    let percent = if total_full > 0.0 {
        total_energy * 100.0 / total_full
    } else {
        0.0
    };
    (percent, state)
}

/// Port of `battery()` from
/// `powerline/segments/common/bat.py:215`.
///
/// `get_status` returns `(percent, ac_powered)` or None when battery
/// status isn't available. Default `format` is `"{ac_state} {capacity:3.0%}"`
/// matching py:215.
#[allow(clippy::too_many_arguments)]
pub fn battery(
    get_status: impl FnOnce() -> Option<(f64, bool)>,
    format: &str,
    steps: u32,
    gamify: bool,
    full_heart: &str,
    empty_heart: &str,
    online: &str,
    offline: &str,
) -> Option<Vec<Value>> {
    // py:255-257  if status is None: return None
    let (capacity, ac_powered) = get_status()?;
    let ac_state = if ac_powered { online } else { offline };
    let mut ret: Vec<Value> = Vec::new();
    if gamify {
        // py:262-263  denom = int(steps); numer = int(denom * capacity / 100)
        let denom = steps as i64;
        let numer = ((denom as f64) * capacity / 100.0) as i64;
        // py:264-269  ac_state segment
        let online_or_offline_group = if ac_powered {
            "battery_online"
        } else {
            "battery_offline"
        };
        ret.push(json!({
            "contents": ac_state,
            "draw_inner_divider": false,
            "highlight_groups": [
                online_or_offline_group,
                "battery_ac_state",
                "battery_gradient",
                "battery"
            ],
            "gradient_level": 0,
        }));
        // py:270-275  full_heart segment (numer hearts)
        ret.push(json!({
            "contents": full_heart.repeat(numer.max(0) as usize),
            "draw_inner_divider": false,
            "highlight_groups": ["battery_full", "battery_gradient", "battery"],
            "gradient_level": 0,
        }));
        // py:276-282  empty_heart segment
        let empty_count = (denom - numer).max(0) as usize;
        ret.push(json!({
            "contents": empty_heart.repeat(empty_count),
            "draw_inner_divider": false,
            "highlight_groups": ["battery_empty", "battery_gradient", "battery"],
            "gradient_level": 100,
        }));
    } else {
        // py:284-288  format.format(ac_state=..., capacity=capacity/100.0)
        let pct_str = format!("{:3.0}%", capacity);
        let contents = format
            .replace("{ac_state}", ac_state)
            .replace("{capacity:3.0%}", &pct_str);
        ret.push(json!({
            "contents": contents,
            "highlight_groups": ["battery_gradient", "battery"],
            "gradient_level": 100.0 - capacity,
        }));
    }
    Some(ret)
}

/// Port of `_get_battery_status` (top-level) from
/// `powerline/segments/common/bat.py:208`.
///
/// Python caches the dispatcher fn after the first call. Rust port
/// surfaces the dispatch as a simple `(percent, ac_state)` Option;
/// the caching happens at the caller's level since Rust closures
/// don't rebind a global by name.
pub fn _get_battery_status<F>(fetcher: F) -> Option<(f64, bool)>
where
    F: FnOnce() -> Option<(f64, bool)>,
{
    // py:213-216  try _fetch_battery_info; except: _failing_get_status
    fetcher()
}

/// Builds the per-device list passed to `flatten_battery` from
/// the two parallel slices: `energies` and `energy_fulls` of the same
/// length plus a single `state_flag`.
///
/// Mirrors the inner loop at py:101-114 where each device produces
/// (energy_full, energy, status_str) and the aggregator AND-folds
/// the status flag.
pub fn build_device_table(
    energies: &[f64],
    energy_fulls: &[f64],
    states: &[bool],
) -> Vec<(f64, f64, bool)> {
    energies
        .iter()
        .zip(energy_fulls.iter())
        .zip(states.iter())
        .map(|((e, f), s)| (*e, *f, *s))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_percent_re_matches_pmset_output() {
        // py:148-149  matches digits before %
        let r = BATTERY_PERCENT_RE();
        let c = r.captures("Battery 87%; discharging").unwrap();
        assert_eq!(&c[1], "87");
    }

    #[test]
    fn battery_percent_re_matches_zero() {
        let r = BATTERY_PERCENT_RE();
        let c = r.captures("0%; discharging").unwrap();
        assert_eq!(&c[1], "0");
    }

    #[test]
    fn parse_pmset_output_extracts_percent_and_no_ac() {
        let r = parse_pmset_output("Battery 87%; discharging; 4:23 remaining");
        assert_eq!(r, Some((87, false)));
    }

    #[test]
    fn parse_pmset_output_detects_ac_charging() {
        // py:150  'AC' in output
        let r = parse_pmset_output("Battery 87%; charging; AC Power");
        assert_eq!(r, Some((87, true)));
    }

    #[test]
    fn parse_pmset_output_no_percent_returns_none() {
        let r = parse_pmset_output("No batt info");
        assert!(r.is_none());
    }

    #[test]
    fn parse_linux_status_discharging_returns_false() {
        // py:111  state &= (line.strip() != 'Discharging')
        assert!(!parse_linux_status("Discharging"));
        assert!(!parse_linux_status("  Discharging  \n"));
    }

    #[test]
    fn parse_linux_status_charging_returns_true() {
        assert!(parse_linux_status("Charging"));
        assert!(parse_linux_status("Full"));
        assert!(parse_linux_status("Not charging"));
    }

    #[test]
    fn flatten_battery_single_device() {
        // py:55-72 single-device case
        let r = flatten_battery(&[(50.0, 100.0, true)]);
        assert!((r.0 - 50.0).abs() < 1e-9);
        assert!(r.1);
    }

    #[test]
    fn flatten_battery_multi_device_averages_percent() {
        // py:55-72 sums energy + energy_full across devices
        let r = flatten_battery(&[(50.0, 100.0, true), (25.0, 100.0, true)]);
        // total energy = 75, total full = 200 → 37.5%
        assert!((r.0 - 37.5).abs() < 1e-9);
    }

    #[test]
    fn flatten_battery_zero_full_returns_zero() {
        // py:74-77  if energy_full > 0 else 0.0
        let r = flatten_battery(&[(0.0, 0.0, true)]);
        assert_eq!(r.0, 0.0);
    }

    #[test]
    fn flatten_battery_state_is_and_fold() {
        // py:65-72  state &= dev.state
        let r = flatten_battery(&[(10.0, 100.0, true), (10.0, 100.0, false)]);
        assert!(!r.1);
        let r2 = flatten_battery(&[(10.0, 100.0, true), (10.0, 100.0, true)]);
        assert!(r2.1);
    }

    #[test]
    fn battery_none_status_returns_none() {
        let r = battery(
            || None,
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        );
        assert!(r.is_none());
    }

    #[test]
    fn battery_percent_format_emits_single_segment() {
        // py:283-287 non-gamify path
        let r = battery(
            || Some((75.0, false)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        let contents = r[0]["contents"].as_str().unwrap();
        assert!(contents.contains("75"));
        // offline branch (ac_powered=false) → " " ac_state
        assert!(contents.starts_with(' '));
    }

    #[test]
    fn battery_percent_format_emits_correct_gradient_level() {
        // py:287  gradient_level = 100 - capacity
        let r = battery(
            || Some((20.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        let level = r[0]["gradient_level"].as_f64().unwrap();
        assert!((level - 80.0).abs() < 1e-9);
    }

    #[test]
    fn battery_gamify_emits_three_segments() {
        // py:262-282 gamify path: ac_state + full hearts + empty hearts
        let r = battery(
            || Some((60.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            true,
            "F",
            "E",
            "ON",
            "OFF",
        )
        .unwrap();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn battery_gamify_ac_state_uses_online_group_when_charging() {
        let r = battery(|| Some((60.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        // py:267  'battery_online' if ac_powered else 'battery_offline'
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "battery_online");
        assert_eq!(r[0]["contents"], "ON");
    }

    #[test]
    fn battery_gamify_ac_state_uses_offline_group_when_not_charging() {
        let r = battery(|| Some((60.0, false)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "battery_offline");
        assert_eq!(r[0]["contents"], "OFF");
    }

    #[test]
    fn battery_gamify_emits_correct_full_and_empty_counts() {
        // py:262-263  denom = steps; numer = denom * capacity / 100
        // 60% of 5 steps = 3 full, 2 empty
        let r = battery(|| Some((60.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        assert_eq!(r[1]["contents"], "FFF");
        assert_eq!(r[2]["contents"], "EE");
    }

    #[test]
    fn battery_gamify_zero_percent_emits_only_empty() {
        let r = battery(|| Some((0.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        assert_eq!(r[1]["contents"], "");
        assert_eq!(r[2]["contents"], "EEEEE");
    }

    #[test]
    fn battery_gamify_full_percent_emits_only_full() {
        let r = battery(|| Some((100.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        assert_eq!(r[1]["contents"], "FFFFF");
        assert_eq!(r[2]["contents"], "");
    }

    #[test]
    fn battery_gamify_full_segment_gradient_zero() {
        // py:274  gradient_level=0  for full
        let r = battery(|| Some((50.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        assert_eq!(r[1]["gradient_level"], 0);
    }

    #[test]
    fn battery_gamify_empty_segment_gradient_100() {
        // py:281  gradient_level=100 for empty (most alert)
        let r = battery(|| Some((50.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        assert_eq!(r[2]["gradient_level"], 100);
    }

    #[test]
    fn battery_gamify_draw_inner_divider_all_false() {
        // py:267, 273, 280  draw_inner_divider = False everywhere
        let r = battery(|| Some((50.0, true)), "", 5, true, "F", "E", "ON", "OFF").unwrap();
        for seg in &r {
            assert_eq!(seg["draw_inner_divider"], false);
        }
    }

    #[test]
    fn battery_percent_uses_online_when_ac_powered() {
        let r = battery(
            || Some((80.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        // ac_state="C" → online
        let contents = r[0]["contents"].as_str().unwrap();
        assert!(contents.starts_with('C'));
    }

    #[test]
    fn build_device_table_zips_parallel_slices() {
        let r = build_device_table(&[10.0, 20.0], &[100.0, 200.0], &[true, false]);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], (10.0, 100.0, true));
        assert_eq!(r[1], (20.0, 200.0, false));
    }

    #[test]
    fn battery_percent_format_emits_battery_gradient_groups() {
        // py:285  highlight_groups: ['battery_gradient', 'battery']
        let r = battery(
            || Some((50.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "battery_gradient");
        assert_eq!(groups[1], "battery");
    }

    #[test]
    fn get_battery_status_delegates_to_fetcher() {
        let r = _get_battery_status(|| Some((42.0, true)));
        assert_eq!(r, Some((42.0, true)));
    }

    #[test]
    fn get_battery_status_propagates_none() {
        let r = _get_battery_status(|| None);
        assert!(r.is_none());
    }
}
