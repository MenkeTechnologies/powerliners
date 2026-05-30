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
    // py:141  if which('pmset'):
    // py:142  pl.debug('Using pmset')
    // py:144  BATTERY_PERCENT_RE = re.compile(r'(\d+)%')
    // py:146  def _get_battery_status(pl):
    // py:147  battery_summary = run_cmd(pl, ['pmset', '-g', 'batt'])
    // py:148  battery_percent = BATTERY_PERCENT_RE.search(battery_summary).group(1)
    // py:149  ac_charging = 'AC' in battery_summary
    // py:150  return int(battery_percent), ac_charging
    // py:151  return _get_battery_status
    // py:152  else:
    // py:153  pl.debug('Not using pmset: executable not found')
    let caps = BATTERY_PERCENT_RE().captures(text)?;
    let percent: u8 = caps.get(1)?.as_str().parse().ok()?;
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
    // py:82  if os.path.isdir('/sys/class/power_supply'):
    // py:83  # ENERGY_* attributes represents capacity in µWh only.
    // py:84  # CHARGE_* attributes represents capacity in µAh only.
    // py:85  linux_capacity_units = ('energy', 'charge')
    // py:86  linux_energy_full_fmt = '/sys/class/power_supply/{0}/{1}_full'
    // py:87  linux_energy_fmt = '/sys/class/power_supply/{0}/{1}_now'
    // py:88  linux_status_fmt = '/sys/class/power_supply/{0}/status'
    // py:89  devices = []
    // py:90  for linux_supplier in os.listdir('/sys/class/power_supply'):
    // py:91  for unit in linux_capacity_units:
    // py:92  energy_path = linux_energy_fmt.format(linux_supplier, unit)
    // py:93  if not os.path.exists(energy_path):
    // py:94  continue
    // py:95  pl.debug('Using /sys/class/power_supply with battery {0} and unit {1}',
    // py:96  linux_supplier, unit)
    // py:97  devices.append((linux_supplier, unit))
    // py:98  break  # energy or charge, not both
    // py:100  def _get_battery_status(pl):
    // py:101  energy = 0.0
    // py:102  energy_full = 0.0
    // py:103  state = True
    // py:104  for device, unit in devices:
    // py:105  with open(linux_energy_full_fmt.format(device, unit), 'r') as f:
    // py:106  energy_full += int(float(f.readline().split()[0]))
    // py:107  with open(linux_energy_fmt.format(device, unit), 'r') as f:
    // py:108  energy += int(float(f.readline().split()[0]))
    // py:109  try:
    // py:110  with open(linux_status_fmt.format(device), 'r') as f:
    // py:111  state &= (f.readline().strip() != 'Discharging')
    // py:112  except IOError:
    // py:113  state = None
    // py:114  return (energy * 100.0 / energy_full), state
    // py:115  return _get_battery_status
    // py:116  pl.debug('Not using /sys/class/power_supply as no batteries were found')
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
    // py:11  def _fetch_battery_info(pl):
    // py:12  try:
    // py:13  import dbus
    // py:14  except ImportError:
    // py:15  pl.debug('Not using DBUS+UPower as dbus is not available')
    // py:16  else:
    // py:17  try:
    // py:18  bus = dbus.SystemBus()
    // py:19  except Exception as e:
    // py:20  pl.exception('Failed to connect to system bus: {0}', str(e))
    // py:22  interface = 'org.freedesktop.UPower'
    // py:24  up = bus.get_object(interface, '/org/freedesktop/UPower')
    // py:33  devices = []
    // py:34  for devpath in up.EnumerateDevices(dbus_interface=interface):
    // py:41  if int(devget('Type')) != 2:
    // py:44  if not bool(devget('IsPresent')):
    // py:47  if not bool(devget('PowerSupply')):
    // py:50  devices.append(devpath)
    // py:53  def _flatten_battery(pl):
    // py:54  energy = 0.0
    // py:55  energy_full = 0.0
    // py:56  state = True
    // py:57  for devpath in devices:
    // py:58  dev = bus.get_object(interface, devpath)
    // py:59  energy_full += float(
    // py:60  dbus.Interface(dev, dbus_interface=devinterface).Get(
    // py:61  devtype_name,
    // py:62  'EnergyFull'
    // py:63  ),
    // py:64  )
    // py:65  energy += float(
    // py:66  dbus.Interface(dev, dbus_interface=devinterface).Get(
    // py:67  devtype_name,
    // py:68  'Energy'
    // py:69  ),
    // py:70  )
    // py:71  state &= dbus.Interface(dev, dbus_interface=devinterface).Get(
    // py:72  devtype_name,
    // py:73  'State'
    // py:74  ) != 2
    // py:75  if energy_full > 0:
    // py:76  return (energy * 100.0 / energy_full), state
    // py:77  else:
    // py:78  return 0.0, state
    // py:79  return _flatten_battery
    // py:80  pl.debug('Not using DBUS+UPower as no batteries were found')
    let mut total_energy = 0.0;
    let mut total_full = 0.0;
    let mut state = true;
    for (e, f, s) in devices {
        total_energy += e;
        total_full += f;
        state &= s;
    }
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
    // py:230  def battery(pl, format='{ac_state} {capacity:3.0%}', steps=5, gamify=False, full_heart='O', empty_heart='O', online='C', offline=' '):
    // py:231-263  docstring
    // py:264  try:
    // py:265  capacity, ac_powered = _get_battery_status(pl)
    // py:266  except NotImplementedError:
    // py:267  pl.info('Unable to get battery status.')
    // py:268  return None
    let (capacity, ac_powered) = get_status()?;
    let ac_state = if ac_powered { online } else { offline };
    // py:270  ret = []
    let mut ret: Vec<Value> = Vec::new();
    // py:271  if gamify:
    if gamify {
        // py:272  denom = int(steps)
        // py:273  numer = int(denom * capacity / 100)
        let denom = steps as i64;
        let numer = ((denom as f64) * capacity / 100.0) as i64;
        // py:274  ret.append({
        // py:275  'contents': online if ac_powered else offline,
        // py:276  'draw_inner_divider': False,
        // py:277  'highlight_groups': ['battery_online' if ac_powered else 'battery_offline', 'battery_ac_state', 'battery_gradient', 'battery'],
        // py:278  'gradient_level': 0,
        // py:279  })
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
        // py:280  ret.append({
        // py:281  'contents': full_heart * numer,
        // py:282  'draw_inner_divider': False,
        // py:283  'highlight_groups': ['battery_full', 'battery_gradient', 'battery'],
        // py:284  # Using zero as "nothing to worry about": it is least alert color.
        // py:285  'gradient_level': 0,
        // py:286  })
        ret.push(json!({
            "contents": full_heart.repeat(numer.max(0) as usize),
            "draw_inner_divider": false,
            "highlight_groups": ["battery_full", "battery_gradient", "battery"],
            "gradient_level": 0,
        }));
        // py:287  ret.append({
        // py:288  'contents': empty_heart * (denom - numer),
        // py:289  'draw_inner_divider': False,
        // py:290  'highlight_groups': ['battery_empty', 'battery_gradient', 'battery'],
        // py:291  # Using a hundred as it is most alert color.
        // py:292  'gradient_level': 100,
        // py:293  })
        let empty_count = (denom - numer).max(0) as usize;
        ret.push(json!({
            "contents": empty_heart.repeat(empty_count),
            "draw_inner_divider": false,
            "highlight_groups": ["battery_empty", "battery_gradient", "battery"],
            "gradient_level": 100,
        }));
    } else {
        // py:294  else:
        // py:295  ret.append({
        // py:296  'contents': format.format(ac_state=(online if ac_powered else offline), capacity=(capacity / 100.0)),
        // py:297  'highlight_groups': ['battery_gradient', 'battery'],
        // py:298  # Gradients are "least alert – most alert" by default, capacity has
        // py:299  # the opposite semantics.
        // py:300  'gradient_level': 100 - capacity,
        // py:301  })
        //
        // Inlined port of Python `format.format(ac_state=…, capacity=…)`
        // for the two placeholders the segment uses: `{ac_state}` and
        // `{capacity[:N.M%]}`. Implementation notes:
        //   • Python passes `capacity / 100.0` then the `%` type
        //     multiplies by 100 — net integer-percent in 0..100; we just
        //     format the 0..100 value directly.
        //   • Field width N in `{capacity:N.M%}` applies to the FULL
        //     output (`87%`), NOT just the digits. `:3.0%` w/ capacity=87
        //     → '87%' (3 chars, no pad); capacity=5 → ' 5%' (right-pad).
        //   • Precision M defaults to 6 when `%` type is used without
        //     explicit `.M` (Python float-type default).
        // No helper fns — closures live inside the body so no Rust-only
        // names land under src/ported/ (PORT.md Rule 0).
        let parse_pct_spec = |spec: &str| -> Option<(Option<usize>, Option<usize>)> {
            let spec = spec.strip_suffix('%')?;
            let (w_str, p_str) = match spec.find('.') {
                Some(dot) => (&spec[..dot], Some(&spec[dot + 1..])),
                None => (spec, None),
            };
            let width = if w_str.is_empty() {
                None
            } else {
                w_str.parse().ok()
            };
            let precision = match p_str {
                Some(p) => p.parse().ok(),
                None => Some(6),
            };
            Some((width, precision))
        };
        let render_placeholder = |spec: &str| -> String {
            if spec == "ac_state" {
                return ac_state.to_string();
            }
            if let Some(rest) = spec.strip_prefix("capacity") {
                let rest = rest.strip_prefix(':').unwrap_or(rest);
                let (width, precision) = parse_pct_spec(rest).unwrap_or((None, None));
                let value_str = match precision {
                    Some(p) => format!("{:.*}", p, capacity),
                    None => format!("{}", capacity as i64),
                };
                let core = format!("{}%", value_str);
                return match width {
                    Some(w) if core.chars().count() < w => format!("{:>1$}", core, w),
                    _ => core,
                };
            }
            format!("{{{}}}", spec)
        };
        let mut contents = String::with_capacity(format.len());
        let bytes = format.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                if let Some(end) = format[i + 1..].find('}') {
                    contents.push_str(&render_placeholder(&format[i + 1..i + 1 + end]));
                    i += 1 + end + 1;
                    continue;
                }
            }
            contents.push(bytes[i] as char);
            i += 1;
        }
        // When the offline placeholder collapses to whitespace (the
        // default powerline.json override sets `offline: " "`) AND the
        // format string actually starts with the `{ac_state}` slot, strip
        // the leading whitespace that came from the placeholder + the
        // separator that follows it. Format strings that don't lead with
        // `{ac_state}` (e.g. `{capacity:5.1%}` alone) keep any
        // intrinsic width-pad whitespace from the `%` spec — that
        // whitespace is part of the percentage rendering, not the icon.
        let contents = if ac_state.trim().is_empty() && format.starts_with("{ac_state}") {
            contents.trim_start().to_string()
        } else {
            contents
        };
        ret.push(json!({
            "contents": contents,
            "highlight_groups": ["battery_gradient", "battery"],
            "gradient_level": 100.0 - capacity,
        }));
    }
    // py:302  return ret
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
    // py:117  else:
    // py:118  pl.debug("Checking for first capacity battery percentage")
    // py:119  for batt in os.listdir('/sys/class/power_supply'):
    // py:120  if os.path.exists('/sys/class/power_supply/{0}/capacity'.format(batt)):
    // py:121  def _get_battery_perc(pl):
    // py:122  state = True
    // py:123  with open('/sys/class/power_supply/{0}/capacity'.format(batt), 'r') as f:
    // py:124  perc = int(f.readline().split()[0])
    // py:125  try:
    // py:126  with open(linux_status_fmt.format(batt), 'r') as f:
    // py:127  state &= (f.readline().strip() != 'Discharging')
    // py:128  except IOError:
    // py:129  state = None
    // py:130  return perc, state
    // py:131  return _get_battery_perc
    // py:132  else:
    // py:133  pl.debug('Not using /sys/class/power_supply: no directory')
    // py:135  try:
    // py:136  from shutil import which  # Python-3.3 and later
    // py:137  except ImportError:
    // py:138  pl.info('Using dumb "which" which only checks for file in /usr/bin')
    // py:139  which = lambda f: (lambda fp: os.path.exists(fp) and fp)(os.path.join('/usr/bin', f))
    // py:155  if sys.platform.startswith('win') or sys.platform == 'cygwin':
    // py:156  # From http://stackoverflow.com/a/21083571/273566, reworked
    // py:157  try:
    // py:158  from win32com.client import GetObject
    // py:159  except ImportError:
    // py:160  pl.debug('Not using win32com.client as it is not available')
    // py:161  else:
    // py:162  try:
    // py:163  wmi = GetObject('winmgmts:')
    // py:164  except Exception as e:
    // py:165  pl.exception('Failed to run GetObject from win32com.client: {0}', str(e))
    // py:166  else:
    // py:167  for battery in wmi.InstancesOf('Win32_Battery'):
    // py:168  pl.debug('Using win32com.client with Win32_Battery')
    // py:170  def _get_battery_status(pl):
    // py:171  # http://msdn.microsoft.com/en-us/library/aa394074(v=vs.85).aspx
    // py:172  return battery.EstimatedChargeRemaining, battery.BatteryStatus == 6
    // py:174  return _get_battery_status
    // py:175  pl.debug('Not using win32com.client as no batteries were found')
    // py:176  from ctypes import Structure, c_byte, c_ulong, byref
    // py:177  if sys.platform == 'cygwin':
    // py:178  pl.debug('Using cdll to communicate with kernel32 (Cygwin)')
    // py:186  class PowerClass(Structure):
    // py:187  _fields_ = [
    // py:188  ('ACLineStatus', c_byte),
    // py:189  ('BatteryFlag', c_byte),
    // py:190  ('BatteryLifePercent', c_byte),
    // py:191  ('Reserved1', c_byte),
    // py:192  ('BatteryLifeTime', c_ulong),
    // py:193  ('BatteryFullLifeTime', c_ulong)
    // py:194  ]
    // py:196  def _get_battery_status(pl):
    // py:197  powerclass = PowerClass()
    // py:198  result = library_loader.kernel32.GetSystemPowerStatus(byref(powerclass))
    // py:199  # http://msdn.microsoft.com/en-us/library/windows/desktop/aa372693(v=vs.85).aspx
    // py:200  if result:
    // py:201  return None
    // py:202  return powerclass.BatteryLifePercent, powerclass.ACLineStatus == 1
    // py:204  if _get_battery_status() is None:
    // py:205  pl.debug('Not using GetSystemPowerStatus because it failed')
    // py:206  else:
    // py:207  pl.debug('Using GetSystemPowerStatus')
    // py:209  return _get_battery_status
    // py:211  raise NotImplementedError
    // py:214  def _get_battery_status(pl):
    // py:215  global _get_battery_status
    // py:217  def _failing_get_status(pl):
    // py:218  raise NotImplementedError
    // py:220  try:
    // py:221  _get_battery_status = _fetch_battery_info(pl)
    // py:222  except NotImplementedError:
    // py:223  _get_battery_status = _failing_get_status
    // py:224  except Exception as e:
    // py:225  pl.exception('Exception while obtaining battery status: {0}', str(e))
    // py:226  _get_battery_status = _failing_get_status
    // py:227  return _get_battery_status(pl)
    fetcher()
}

/// Port of `_failing_get_status()` from
/// `powerline/segments/common/bat.py:217-218`.
///
/// Unconditional-error battery-status callable installed when
/// `_fetch_battery_info` raises `NotImplementedError` (py:222-223 in
/// `battery()`'s setup block). Returns `None` so `battery()` shorts
/// to the early-return at py:228 (segment hidden).
///
/// Python raises `NotImplementedError`; the Rust port returns `None`
/// since the caller treats Option-None and raise-NIE identically at
/// the call site (both bail out of the segment computation).
pub fn _failing_get_status(_pl: &()) -> Option<(f64, bool)> {
    // py:217  def _failing_get_status(pl):
    // py:218  raise NotImplementedError
    None
}

/// Port of `_fetch_battery_info()` from
/// `powerline/segments/common/bat.py:11-217`.
///
/// Python tries dbus+UPower → /sys/class/power_supply → pmset
/// (macOS) → win32com → GetSystemPowerStatus (Windows) and returns
/// the first working backend's closure. The Rust port can't drive
/// dbus/win32 without external bindings; callers inject the per-
/// backend fetcher via the `try_backends` slice.
///
/// Returns the first backend that yields `Some` per the dispatch
/// chain; `None` if none succeeds (caller installs
/// `_failing_get_status`).
pub fn _fetch_battery_info(
    try_backends: &[&dyn Fn() -> Option<(f64, bool)>],
) -> Option<(f64, bool)> {
    // py:11  def _fetch_battery_info(pl):
    // py:12-216  try-cascade through dbus/sys/pmset/win32com/GetSystemPowerStatus
    for backend in try_backends {
        if let Some(result) = backend() {
            return Some(result);
        }
    }
    None
}

/// Port of `_flatten_battery()` from
/// `powerline/segments/common/bat.py:53-77` (the dbus+UPower
/// closure).
///
/// Wrapper around [`flatten_battery`] that mirrors the dbus
/// closure signature — `pl` is the powerline logger handle, `devices`
/// is the precomputed (energy, energy_full, state) tuple list.
/// Python's `_flatten_battery` captures `devices` from its outer
/// scope; the Rust port takes it as an explicit argument.
pub fn _flatten_battery(_pl: &(), devices: &[(f64, f64, bool)]) -> (f64, bool) {
    // py:53  def _flatten_battery(pl):
    // py:54-77  energy/energy_full sum + AND-fold of state
    flatten_battery(devices)
}

/// Port of `_get_battery_perc()` from
/// `powerline/segments/common/bat.py:121-130` (the
/// /sys/class/power_supply closure).
///
/// Reads `/sys/class/power_supply/<batt>/capacity` and `.../status`
/// for the supplied battery device. Returns
/// `Some((percent_0_100, charging_flag))` or `None` when the device
/// files can't be read.
///
/// Python captures `batt` (the device dir name) from the outer for-
/// loop; the Rust port takes it as an explicit argument.
pub fn _get_battery_perc(_pl: &(), batt: &str) -> Option<(u8, Option<bool>)> {
    // py:121  def _get_battery_perc(pl):
    // py:123  with open('/sys/class/power_supply/{0}/capacity', 'r') as f:
    let capacity_path = format!("/sys/class/power_supply/{}/capacity", batt);
    let capacity_text = std::fs::read_to_string(capacity_path).ok()?;
    // py:124  perc = int(f.readline().split()[0])
    let perc: u8 = capacity_text.split_whitespace().next()?.parse().ok()?;
    // py:125  try:
    // py:126  with open(linux_status_fmt.format(batt), 'r') as f:
    let status_path = format!("/sys/class/power_supply/{}/status", batt);
    // py:122  state = True
    let state: Option<bool> = match std::fs::read_to_string(status_path) {
        Ok(status_text) => {
            // py:127  state &= (f.readline().strip() != 'Discharging')
            Some(parse_linux_status(&status_text))
        }
        Err(_) => {
            // py:128  except IOError:
            // py:129  state = None
            None
        }
    };
    // py:130  return perc, state
    Some((perc, state))
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
        // ac_state=" " (offline) is whitespace-only → leading whitespace
        // from `{ac_state} ` collapses so the icon slot doesn't reserve
        // a blank cell. capacity=75 → `75%` (3 chars, no width-3 pad).
        assert_eq!(contents, "75%");
    }

    #[test]
    fn battery_percent_format_keeps_separator_when_online_visible_icon() {
        // ac_state="C" (visible) → leading char + literal separator kept.
        let r = battery(
            || Some((75.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "C 75%");
    }

    #[test]
    fn battery_percent_format_matches_python_width_3_spec() {
        // Python `{capacity:3.0%}` pads the full output (digits + '%')
        // to width 3 — NOT the digits alone. capacity=5 → ' 5%' (3
        // chars). The previous `format!("{:3.0}%", 5.0)` hack produced
        // '  5%' (4 chars) because it padded the number.
        let r = battery(
            || Some((5.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "C  5%");
    }

    #[test]
    fn battery_percent_format_handles_100_percent_without_truncation() {
        // capacity=100 → '100%' (4 chars) > min width 3; Python doesn't
        // truncate, just emits the full value.
        let r = battery(
            || Some((100.0, true)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            " ",
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "C 100%");
    }

    #[test]
    fn battery_percent_format_offline_with_empty_string_also_collapses() {
        // offline="" (user opts out of the placeholder entirely) → same
        // result as offline=" ": leading whitespace from the format
        // string's literal separator is also trimmed.
        let r = battery(
            || Some((42.0, false)),
            "{ac_state} {capacity:3.0%}",
            5,
            false,
            "O",
            "O",
            "C",
            "",
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "42%");
    }

    #[test]
    fn battery_format_supports_alternate_precisions_via_battery_fn() {
        // Theme overrides like `{capacity:.2%}` should produce '87.00%'
        // — the previous two-replacement hack only handled the literal
        // '{capacity:3.0%}' substring. Drive through battery() because
        // the formatter has no standalone fn name per PORT.md Rule 0.
        // `{capacity:N.M%}` standalone (no `{ac_state}` placeholder) →
        // the offline-trim shortcut doesn't apply, output is the pure
        // format-spec result.
        let r1 = battery(
            || Some((87.0, true)),
            "{capacity:.2%}",
            5,
            false,
            "O",
            "O",
            "",
            "",
        )
        .unwrap();
        assert_eq!(r1[0]["contents"], "87.00%");
        let r2 = battery(
            || Some((5.0, true)),
            "{capacity:5.1%}",
            5,
            false,
            "O",
            "O",
            "",
            "",
        )
        .unwrap();
        assert_eq!(r2[0]["contents"], " 5.0%");
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

    #[test]
    fn failing_get_status_returns_none() {
        // py:217-218  raise NotImplementedError → None
        assert!(_failing_get_status(&()).is_none());
    }

    #[test]
    fn fetch_battery_info_returns_first_successful_backend() {
        let backend_a: &dyn Fn() -> Option<(f64, bool)> = &|| None;
        let backend_b: &dyn Fn() -> Option<(f64, bool)> = &|| Some((75.0, true));
        let backend_c: &dyn Fn() -> Option<(f64, bool)> = &|| Some((20.0, false));
        let r = _fetch_battery_info(&[backend_a, backend_b, backend_c]);
        assert_eq!(r, Some((75.0, true)));
    }

    #[test]
    fn fetch_battery_info_none_when_all_backends_fail() {
        let backend: &dyn Fn() -> Option<(f64, bool)> = &|| None;
        let r = _fetch_battery_info(&[backend, backend]);
        assert!(r.is_none());
    }

    #[test]
    fn flatten_battery_closure_delegates_to_flatten_battery() {
        let devices = vec![(20.0, 100.0, true), (80.0, 100.0, true)];
        let r = _flatten_battery(&(), &devices);
        // 100/200 = 50%
        assert_eq!(r.0, 50.0);
        assert!(r.1);
    }

    #[test]
    fn get_battery_perc_missing_files_returns_none() {
        // /sys/class/power_supply/<batt>/capacity doesn't exist for
        // a synthetic name.
        let r = _get_battery_perc(&(), "nonexistent_batt_zz_9999");
        assert!(r.is_none(), "expected None for missing battery, got {r:?}");
    }
}
