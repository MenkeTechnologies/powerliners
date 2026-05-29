// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/net.py`.
//!
//! Network segment helpers: hostname, external IP, internal IP via
//! interface scoring, and network-load rate computation. The actual
//! psutil / netifaces / urllib_read calls are stubbed since they
//! require external Rust crates; the pure-functional pieces
//! (interface scoring, hostname/domain split, render_one rate
//! aggregation) are surfaced for unit testing.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// import os                                        // py:5
// import socket                                    // py:6
// from powerline.lib.url import urllib_read        // py:8
// from powerline.lib.threaded import ThreadedSegment, KwThreadedSegment                  // py:9
// from powerline.lib.monotonic import monotonic    // py:10
// from powerline.lib.humanize_bytes import humanize_bytes                                 // py:11
// from powerline.segments import with_docstring    // py:12
// from powerline.theme import requires_segment_info                                       // py:13

use crate::ported::lib::humanize_bytes::humanize_bytes;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

/// Sentinel UUID that bypasses live hostname lookup during shell
/// tests. Identical to the env.py user-segment UUID at
/// `powerline/segments/common/env.py:171`.
pub const POWERLINE_TEST_HOSTNAME_UUID: &str = "ee5bcdc6-b749-11e7-9456-50465d597777";

/// Port of `hostname()` segment from
/// `powerline/segments/common/net.py:16`.
///
/// `hostname_lookup` is a caller-supplied closure returning the
/// system hostname (Python uses `socket.gethostname()`). Returns
/// `None` when `only_if_ssh=true` and `SSH_CLIENT` is unset.
pub fn hostname<F>(
    environ: &Map<String, Value>,
    only_if_ssh: bool,
    exclude_domain: bool,
    hostname_lookup: F,
) -> Option<String>
where
    F: FnOnce() -> String,
{
    // py:24-28  _POWERLINE_RUNNING_SHELL_TESTS UUID short-circuit
    if let Some(test_uuid) = environ
        .get("_POWERLINE_RUNNING_SHELL_TESTS")
        .and_then(|v| v.as_str())
    {
        if test_uuid == POWERLINE_TEST_HOSTNAME_UUID {
            return Some("hostname".to_string());
        }
    }
    // py:29-30  if only_if_ssh and not SSH_CLIENT: return None
    if only_if_ssh
        && !environ
            .get("SSH_CLIENT")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
    {
        return None;
    }
    // py:31-33  exclude_domain → split on '.' first
    let h = hostname_lookup();
    if exclude_domain {
        Some(h.split('.').next().unwrap_or(&h).to_string())
    } else {
        Some(h)
    }
}

/// Port of `_external_ip()` from
/// `powerline/segments/common/net.py:36`.
///
/// `read` is the caller-supplied closure that returns the raw body
/// (Python calls `urllib_read(query_url)`).
pub fn _external_ip<F>(read: F) -> Option<String>
where
    F: FnOnce() -> Option<String>,
{
    // py:37  return urllib_read(query_url).strip()
    read().map(|s| s.trim().to_string())
}

/// Port of `ExternalIpSegment.render()` from
/// `powerline/segments/common/net.py:51`.
pub fn external_ip_render(ip: Option<&str>) -> Option<Vec<Value>> {
    // py:52-53  if not ip: return None
    let ip = ip.filter(|s| !s.is_empty())?;
    // py:54  return [{contents, divider_highlight_group}]
    Some(vec![json!({
        "contents": ip,
        "divider_highlight_group": "background:divider",
    })])
}

/// Returns the `_interface_starts` priority dict from
/// `powerline/segments/common/net.py:79-91`.
pub fn interface_starts() -> &'static [(&'static str, i32)] {
    // py:79-91  ordered by Python dict-iteration sense (LinkedHashMap-equivalent)
    &[
        ("eth", 10),
        ("enp", 10),
        ("en", 10),
        ("ath", 9),
        ("wlan", 9),
        ("wlp", 9),
        ("teredo", 1),
        ("lo", -10),
        ("docker", -5),
        ("vmnet", -5),
        ("vboxnet", -5),
    ]
}

/// Compiled prefix regex for `_interface_key`. Matches the alpha
/// prefix + optional first digit (or end-of-string).
pub fn _interface_start_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^([a-z]+?)(\d|$)").unwrap())
}

/// Port of `_interface_key()` from
/// `powerline/segments/common/net.py:94`.
///
/// Sort key used by `interface='auto'` selection. Higher key wins
/// (Python sorts with `reverse=True`).
pub fn _interface_key(interface: &str) -> i64 {
    // py:95  match = _interface_start_re.match(interface)
    let caps = match _interface_start_re().captures(interface) {
        Some(c) => c,
        // py:104-105  return 0 when no match
        None => return 0,
    };
    let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
    // py:97-99  base = _interface_starts[prefix] * 100; KeyError → 500
    let base = match interface_starts().iter().find(|(p, _)| *p == prefix) {
        Some((_, v)) => (*v as i64) * 100,
        None => 500,
    };
    // py:100-102  if match.group(2): return base - int(group(2)); else base
    let suffix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    if let Ok(n) = suffix.parse::<i64>() {
        base - n
    } else {
        base
    }
}

/// Compiled `replace_num_pat` regex for the `NetworkLoadSegment`
/// activity-based fallback at py:191.
pub fn replace_num_pat() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"[a-zA-Z]+").unwrap())
}

/// Port of `NetworkLoadSegment.render_one()` from
/// `powerline/segments/common/net.py:246`.
///
/// Computes the per-interval rate (in bytes/sec) from the two
/// `(timestamp_ms, (bytes_recv, bytes_sent))` samples and emits two
/// segments (DL + UL) with optional gradient highlighting.
#[allow(clippy::too_many_arguments)]
pub fn render_one(
    prev: Option<(f64, (u64, u64))>,
    last: Option<(f64, (u64, u64))>,
    recv_format: &str,
    sent_format: &str,
    suffix: &str,
    si_prefix: bool,
    recv_max: Option<f64>,
    sent_max: Option<f64>,
) -> Option<Vec<Value>> {
    // py:247-248  if not idata or 'prev' not in idata: return None
    let (t1, b1) = prev?;
    let (t2, b2) = last?;
    // py:251  measure_interval = t2 - t1
    let interval = t2 - t1;
    let mut out: Vec<Value> = Vec::new();
    for (i, key) in [(0usize, "recv"), (1usize, "sent")] {
        let fmt = if key == "recv" {
            recv_format
        } else {
            sent_format
        };
        // py:259-262  value = (b2[i] - b1[i]) / interval; ZeroDivisionError → 0
        let bytes_delta = if i == 0 { b2.0 - b1.0 } else { b2.1 - b1.1 };
        let value = if interval == 0.0 {
            0.0
        } else {
            bytes_delta as f64 / interval
        };
        // py:264-265  hl_groups = ['network_load_'+key, 'network_load']
        let max = if key == "recv" { recv_max } else { sent_max };
        let is_gradient = max.is_some();
        let mut hl_groups: Vec<String> =
            vec![format!("network_load_{}", key), "network_load".to_string()];
        // py:267  hl_groups[:0] = (group + '_gradient' for group in hl_groups)
        if is_gradient {
            let gradient: Vec<String> = hl_groups
                .iter()
                .map(|g| format!("{}_gradient", g))
                .collect();
            let mut new_groups = gradient;
            new_groups.extend(hl_groups);
            hl_groups = new_groups;
        }
        // py:268-273  build segment
        let contents = fmt.replace("{value}", &humanize_bytes(value, suffix, si_prefix));
        let mut entry = json!({
            "contents": contents,
            "divider_highlight_group": "network_load:divider",
            "highlight_groups": hl_groups,
        });
        // py:274-278  gradient_level
        if let Some(m) = max {
            let level = if value >= m { 100.0 } else { value * 100.0 / m };
            entry["gradient_level"] = json!(level);
        }
        out.push(entry);
    }
    Some(out)
}

/// Port of `class ExternalIpSegment(ThreadedSegment)` from
/// `powerline/segments/common/net.py:41-54`.
///
/// Marker struct holding the query URL + the
/// thread-update-interval class constant.
#[derive(Debug, Clone)]
pub struct ExternalIpSegment {
    /// Python: `self.query_url` set by `set_state` (py:45). Default
    /// per py:44 keyword arg.
    pub query_url: String,
}

impl ExternalIpSegment {
    /// Port of the `interval` class attribute at
    /// `powerline/segments/common/net.py:42`.
    pub const INTERVAL: u64 = 300;

    /// Port of the `set_state` keyword default at
    /// `powerline/segments/common/net.py:44`.
    pub const DEFAULT_QUERY_URL: &'static str = "http://ipv4.icanhazip.com/";

    /// Construct with the default query URL.
    pub fn new() -> Self {
        Self {
            query_url: Self::DEFAULT_QUERY_URL.to_string(),
        }
    }

    /// Port of `ExternalIpSegment.set_state()` from
    /// `powerline/segments/common/net.py:44-46`.
    ///
    /// Stores `query_url` on self per py:45. The base class
    /// `set_state(**kwargs)` dispatch at py:46 is the
    /// `ThreadedSegment.set_state` port and is invoked separately.
    pub fn set_state(&mut self, query_url: Option<&str>) {
        // py:45  self.query_url = query_url
        if let Some(url) = query_url {
            self.query_url = url.to_string();
        }
    }

    /// Port of `ExternalIpSegment.update()` from
    /// `powerline/segments/common/net.py:48-49`.
    ///
    /// Returns `_external_ip(query_url=self.query_url)` per py:49.
    /// `read` is the caller-supplied closure that performs the HTTP
    /// GET (the Python `urllib_read` call).
    pub fn update<F>(&self, read: F) -> Option<String>
    where
        F: FnOnce(&str) -> Option<String>,
    {
        // py:49  return _external_ip(query_url=self.query_url)
        _external_ip(|| read(&self.query_url))
    }

    /// Port of `ExternalIpSegment.render()` from
    /// `powerline/segments/common/net.py:51-54`.
    ///
    /// Delegates to the standalone `external_ip_render` since render
    /// has no instance-state dependency.
    pub fn render(&self, ip: Option<&str>) -> Option<Vec<Value>> {
        external_ip_render(ip)
    }
}

impl Default for ExternalIpSegment {
    fn default() -> Self {
        Self::new()
    }
}

/// Port of `internal_ip()` from
/// `powerline/segments/common/net.py:76-77` (the no-netifaces
/// branch).
///
/// Python's source has two branches: the `netifaces`-enabled one
/// at py:109-128 and the fallback at py:76-77 returning None. Rust
/// doesn't bundle a `netifaces` analog by default; the port surfaces
/// the fallback. The full interface-resolution + `getifaddrs` path
/// is deferred to a follow-on pass that wires a Rust crate.
pub fn internal_ip(_interface: &str, _ipv: u8) -> Option<String> {
    // py:77  return None
    None
}

/// Port of `NetworkLoadSegment.key()` from
/// `powerline/segments/common/net.py:198-200`.
///
/// Static dispatch key — Python's KwThreadedSegment uses the result
/// as the cache key for the per-interface state.
pub fn network_load_key(interface: &str) -> String {
    // py:200  return interface
    interface.to_string()
}

/// Port of the `/proc/net/route` default-interface scanner at
/// `powerline/segments/common/net.py:208-216`.
///
/// Returns the interface name whose destination column is all
/// zeros (the default-route entry), or None when no such row exists.
/// `content` is the raw text of `/proc/net/route` (Python reads it
/// at py:209-210).
pub fn parse_proc_net_route_default(content: &str) -> Option<String> {
    // py:210-216  for line in f: parts = line.split(); destination → 0
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // py:212  if len(parts) > 1
        if parts.len() < 2 {
            continue;
        }
        let iface = parts[0];
        let destination = parts[1];
        // py:214  if not destination.replace('0', '')  — all zeros
        if destination.chars().all(|c| c == '0') {
            // py:215  interface = iface.decode('utf-8')
            return Some(iface.to_string());
        }
    }
    None
}

/// Port of the activity-based interface picker at
/// `powerline/segments/common/net.py:218-228`.
///
/// `interfaces` is an iterator of `(name, rx_bytes, tx_bytes)`.
/// Returns the interface with the highest total `rx+tx`, excluding
/// loopback (`lo`), VMware (`vmnet`), and Linux-SIT (`sit`) by
/// regex-extracted alpha prefix. Defaults to `"eth0"` per py:220.
pub fn pick_active_interface<'a, I>(interfaces: I) -> String
where
    I: IntoIterator<Item = (&'a str, u64, u64)>,
{
    // py:220  interface, total = 'eth0', -1
    let mut interface = "eth0".to_string();
    let mut total: i64 = -1;
    let re = replace_num_pat();
    for (name, rx, tx) in interfaces {
        // py:222  base = self.replace_num_pat.match(name)
        let base = match re.find(name) {
            Some(m) => m.as_str(),
            // py:223  None in (base, ...) → continue
            None => continue,
        };
        // py:223  excluded prefixes
        if matches!(base, "lo" | "vmnet" | "sit") {
            continue;
        }
        // py:225  activity = rx + tx
        let activity = (rx as i64) + (tx as i64);
        if activity > total {
            total = activity;
            interface = name.to_string();
        }
    }
    interface
}

/// Port of the sysfs path builders used by `_get_bytes` at
/// `powerline/segments/common/net.py:181/183`.
///
/// Returns `/sys/class/net/<interface>/statistics/{rx,tx}_bytes`.
pub fn sysfs_rx_path(interface: &str) -> String {
    // py:181  '/sys/class/net/{interface}/statistics/rx_bytes'
    format!("/sys/class/net/{}/statistics/rx_bytes", interface)
}

/// `tx_bytes` sysfs path. See [`sysfs_rx_path`].
pub fn sysfs_tx_path(interface: &str) -> String {
    // py:183  '/sys/class/net/{interface}/statistics/tx_bytes'
    format!("/sys/class/net/{}/statistics/tx_bytes", interface)
}

/// Port of `_get_bytes()` from
/// `powerline/segments/common/net.py:180-185` (the no-psutil
/// branch).
///
/// `rx_content` / `tx_content` are the raw text of the
/// `rx_bytes`/`tx_bytes` sysfs files. Returns `(rx, tx)` parsed
/// as u64, or None when either parse fails.
pub fn _get_bytes_sysfs(rx_content: &str, tx_content: &str) -> Option<(u64, u64)> {
    // py:182  rx = int(file.read())
    let rx: u64 = rx_content.trim().parse().ok()?;
    // py:184  tx = int(file.read())
    let tx: u64 = tx_content.trim().parse().ok()?;
    // py:185  return (rx, tx)
    Some((rx, tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(pairs: &[(&str, &str)]) -> Map<String, Value> {
        let mut m = Map::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), Value::String((*v).into()));
        }
        m
    }

    #[test]
    fn hostname_uuid_returns_literal_hostname() {
        // py:24-28  shell-test UUID short-circuit
        let env = env_with(&[(
            "_POWERLINE_RUNNING_SHELL_TESTS",
            POWERLINE_TEST_HOSTNAME_UUID,
        )]);
        let r = hostname(&env, false, false, || "myhost".to_string());
        assert_eq!(r, Some("hostname".to_string()));
    }

    #[test]
    fn hostname_returns_lookup_when_no_ssh_constraint() {
        let env = Map::new();
        let r = hostname(&env, false, false, || "myhost.local".to_string());
        assert_eq!(r, Some("myhost.local".to_string()));
    }

    #[test]
    fn hostname_exclude_domain_strips_dot_suffix() {
        // py:31-32  hostname.split('.')[0]
        let env = Map::new();
        let r = hostname(&env, false, true, || "myhost.lan.example.com".to_string());
        assert_eq!(r, Some("myhost".to_string()));
    }

    #[test]
    fn hostname_only_if_ssh_returns_none_without_ssh_client() {
        // py:29-30  if only_if_ssh and not SSH_CLIENT: return None
        let env = Map::new();
        let r = hostname(&env, true, false, || "myhost".to_string());
        assert!(r.is_none());
    }

    #[test]
    fn hostname_only_if_ssh_returns_hostname_when_ssh_client_set() {
        let env = env_with(&[("SSH_CLIENT", "127.0.0.1 22 22")]);
        let r = hostname(&env, true, false, || "myhost".to_string());
        assert_eq!(r, Some("myhost".to_string()));
    }

    #[test]
    fn external_ip_trims_whitespace() {
        // py:37  urllib_read(query_url).strip()
        let r = _external_ip(|| Some("  1.2.3.4\n".to_string()));
        assert_eq!(r, Some("1.2.3.4".to_string()));
    }

    #[test]
    fn external_ip_none_when_read_fails() {
        let r = _external_ip(|| None);
        assert!(r.is_none());
    }

    #[test]
    fn external_ip_render_emits_segment_with_background_divider() {
        // py:54  return [{contents, divider_highlight_group}]
        let r = external_ip_render(Some("1.2.3.4")).unwrap();
        assert_eq!(r[0]["contents"], "1.2.3.4");
        assert_eq!(r[0]["divider_highlight_group"], "background:divider");
    }

    #[test]
    fn external_ip_render_none_for_empty_or_none() {
        assert!(external_ip_render(None).is_none());
        assert!(external_ip_render(Some("")).is_none());
    }

    #[test]
    fn interface_starts_contains_known_prefixes() {
        // py:79-91 entries
        let table = interface_starts();
        let lookup =
            |key: &str| -> Option<i32> { table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v) };
        assert_eq!(lookup("eth"), Some(10));
        assert_eq!(lookup("enp"), Some(10));
        assert_eq!(lookup("en"), Some(10));
        assert_eq!(lookup("lo"), Some(-10));
        assert_eq!(lookup("teredo"), Some(1));
        assert_eq!(lookup("docker"), Some(-5));
    }

    #[test]
    fn interface_key_eth0_returns_priority() {
        // py:97-101  base = 10*100 - 0 = 1000
        assert_eq!(_interface_key("eth0"), 1000);
    }

    #[test]
    fn interface_key_eth1_lower_than_eth0() {
        // py:100  base - int(group(2))
        assert_eq!(_interface_key("eth1"), 999);
        assert_eq!(_interface_key("eth2"), 998);
        assert!(_interface_key("eth0") > _interface_key("eth1"));
    }

    #[test]
    fn interface_key_lo_lower_than_eth() {
        // py:103-104  lo = -10*100 = -1000
        assert_eq!(_interface_key("lo"), -1000);
        assert!(_interface_key("eth0") > _interface_key("lo"));
    }

    #[test]
    fn interface_key_unknown_prefix_defaults_to_500() {
        // py:97-99  KeyError → 500
        assert_eq!(_interface_key("custom0"), 500);
    }

    #[test]
    fn interface_key_no_alpha_prefix_returns_zero() {
        // py:104-105  no match → 0
        assert_eq!(_interface_key(""), 0);
        assert_eq!(_interface_key("0"), 0);
    }

    #[test]
    fn interface_start_re_matches_prefix_only() {
        let r = _interface_start_re();
        let c = r.captures("eth0").unwrap();
        assert_eq!(&c[1], "eth");
        assert_eq!(&c[2], "0");
    }

    #[test]
    fn replace_num_pat_extracts_alpha_only() {
        let r = replace_num_pat();
        let m = r.find("eth0").unwrap();
        assert_eq!(m.as_str(), "eth");
    }

    #[test]
    fn render_one_no_samples_returns_none() {
        let r = render_one(
            None,
            None,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            None,
            None,
        );
        assert!(r.is_none());
    }

    #[test]
    fn render_one_computes_recv_and_sent_rates() {
        // 100 bytes recv + 200 bytes sent over 10s → 10 + 20 B/s
        let prev = Some((0.0, (0u64, 0u64)));
        let last = Some((10.0, (100u64, 200u64)));
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            None,
            None,
        )
        .unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["contents"], "DL 10 B/s");
        assert_eq!(r[1]["contents"], "UL 20 B/s");
    }

    #[test]
    fn render_one_zero_interval_returns_zero_rate() {
        // py:259-262  ZeroDivisionError → 0
        let prev = Some((5.0, (0u64, 0u64)));
        let last = Some((5.0, (100u64, 200u64)));
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            None,
            None,
        )
        .unwrap();
        assert_eq!(r[0]["contents"], "DL 0 B/s");
        assert_eq!(r[1]["contents"], "UL 0 B/s");
    }

    #[test]
    fn render_one_with_gradient_appends_gradient_groups_and_level() {
        // py:266-278  is_gradient: prepend _gradient groups + set level
        let prev = Some((0.0, (0u64, 0u64)));
        let last = Some((10.0, (100u64, 200u64))); // recv = 10 B/s
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            Some(20.0),
            None,
        )
        .unwrap();
        // recv segment: gradient groups prepended, level = 10*100/20 = 50
        let recv = &r[0];
        let groups = recv["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0], "network_load_recv_gradient");
        assert_eq!(groups[1], "network_load_gradient");
        assert_eq!(groups[2], "network_load_recv");
        assert_eq!(groups[3], "network_load");
        assert_eq!(recv["gradient_level"], 50.0);
        // sent segment: no gradient
        let sent = &r[1];
        assert!(sent.get("gradient_level").is_none());
    }

    #[test]
    fn render_one_gradient_clamps_to_100_at_max() {
        // py:275-276  if value >= max: gradient_level = 100
        let prev = Some((0.0, (0u64, 0u64)));
        let last = Some((1.0, (200u64, 0u64))); // recv = 200 B/s > max
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            Some(100.0),
            None,
        )
        .unwrap();
        assert_eq!(r[0]["gradient_level"], 100.0);
    }

    #[test]
    fn render_one_emits_network_load_divider_group() {
        let prev = Some((0.0, (0u64, 0u64)));
        let last = Some((1.0, (10u64, 20u64)));
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            false,
            None,
            None,
        )
        .unwrap();
        for s in &r {
            assert_eq!(s["divider_highlight_group"], "network_load:divider");
        }
    }

    #[test]
    fn render_one_si_prefix_uses_decimal_units() {
        let prev = Some((0.0, (0u64, 0u64)));
        let last = Some((1.0, (1000u64, 0u64)));
        let r = render_one(
            prev,
            last,
            "DL {value}",
            "UL {value}",
            "B/s",
            true,
            None,
            None,
        )
        .unwrap();
        // 1000 B/s in SI → "1 kB/s"
        let contents = r[0]["contents"].as_str().unwrap();
        assert!(contents.contains("k") || contents.contains("K"));
    }

    #[test]
    fn powerline_test_hostname_uuid_matches_upstream() {
        // py:25 sentinel
        assert_eq!(
            POWERLINE_TEST_HOSTNAME_UUID,
            "ee5bcdc6-b749-11e7-9456-50465d597777"
        );
    }

    #[test]
    fn external_ip_segment_interval_matches_upstream() {
        // py:42  interval = 300
        assert_eq!(ExternalIpSegment::INTERVAL, 300);
    }

    #[test]
    fn external_ip_segment_default_query_url() {
        // py:44  default keyword arg
        assert_eq!(
            ExternalIpSegment::DEFAULT_QUERY_URL,
            "http://ipv4.icanhazip.com/"
        );
        let s = ExternalIpSegment::new();
        assert_eq!(s.query_url, ExternalIpSegment::DEFAULT_QUERY_URL);
    }

    #[test]
    fn external_ip_segment_set_state_overrides_query_url() {
        // py:44-46
        let mut s = ExternalIpSegment::new();
        s.set_state(Some("http://ipv6.icanhazip.com/"));
        assert_eq!(s.query_url, "http://ipv6.icanhazip.com/");
    }

    #[test]
    fn external_ip_segment_set_state_no_arg_preserves_url() {
        let mut s = ExternalIpSegment::new();
        s.set_state(None);
        assert_eq!(s.query_url, ExternalIpSegment::DEFAULT_QUERY_URL);
    }

    #[test]
    fn external_ip_segment_update_dispatches_to_query_url() {
        // py:48-49
        let mut s = ExternalIpSegment::new();
        s.query_url = "http://test.example/ip".to_string();
        let ip = s.update(|url| {
            assert_eq!(url, "http://test.example/ip");
            Some("  4.3.2.1\n".to_string())
        });
        assert_eq!(ip, Some("4.3.2.1".to_string()));
    }

    #[test]
    fn external_ip_segment_render_delegates_to_external_ip_render() {
        // py:51-54
        let s = ExternalIpSegment::new();
        let r = s.render(Some("1.2.3.4")).unwrap();
        assert_eq!(r[0]["contents"], "1.2.3.4");
    }

    #[test]
    fn internal_ip_fallback_returns_none() {
        // py:76-77  no-netifaces branch
        assert!(internal_ip("eth0", 4).is_none());
        assert!(internal_ip("auto", 6).is_none());
    }

    #[test]
    fn network_load_key_returns_interface_name() {
        // py:200
        assert_eq!(network_load_key("eth0"), "eth0");
        assert_eq!(network_load_key("auto"), "auto");
    }

    #[test]
    fn parse_proc_net_route_default_finds_zero_destination_row() {
        // py:208-216  destination column all-zeros indicates default route
        let content = concat!(
            "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n",
            "eth0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0\n",
            "eth0\t000000FE\t00000000\t0001\t0\t0\t0\t000000FF\t0\t0\t0\n",
        );
        assert_eq!(
            parse_proc_net_route_default(content),
            Some("eth0".to_string())
        );
    }

    #[test]
    fn parse_proc_net_route_default_returns_none_when_no_default_route() {
        let content = "Iface\tDestination\nlo\tFFFFFFFF\n";
        assert!(parse_proc_net_route_default(content).is_none());
    }

    #[test]
    fn parse_proc_net_route_default_handles_blank_lines() {
        let content = "\n\neth0\t00000000\tx\n";
        assert_eq!(
            parse_proc_net_route_default(content),
            Some("eth0".to_string())
        );
    }

    #[test]
    fn pick_active_interface_picks_highest_total() {
        // py:218-228  activity = rx + tx; highest wins
        let ifaces = vec![("eth0", 100u64, 50u64), ("wlan0", 1000u64, 500u64)];
        assert_eq!(pick_active_interface(ifaces), "wlan0");
    }

    #[test]
    fn pick_active_interface_excludes_lo_and_vmnet_and_sit() {
        // py:223  excluded prefixes
        let ifaces = vec![
            ("lo", 1_000_000u64, 1_000_000u64),
            ("vmnet0", 999u64, 999u64),
            ("sit0", 500u64, 500u64),
            ("eth0", 100u64, 100u64),
        ];
        assert_eq!(pick_active_interface(ifaces), "eth0");
    }

    #[test]
    fn pick_active_interface_empty_input_defaults_to_eth0() {
        // py:220
        let empty: Vec<(&str, u64, u64)> = vec![];
        assert_eq!(pick_active_interface(empty), "eth0");
    }

    #[test]
    fn pick_active_interface_only_excluded_defaults_to_eth0() {
        let ifaces = vec![("lo", 1u64, 1u64)];
        assert_eq!(pick_active_interface(ifaces), "eth0");
    }

    #[test]
    fn sysfs_rx_path_builds_correct_path() {
        // py:181
        assert_eq!(
            sysfs_rx_path("eth0"),
            "/sys/class/net/eth0/statistics/rx_bytes"
        );
    }

    #[test]
    fn sysfs_tx_path_builds_correct_path() {
        // py:183
        assert_eq!(
            sysfs_tx_path("eth0"),
            "/sys/class/net/eth0/statistics/tx_bytes"
        );
    }

    #[test]
    fn get_bytes_sysfs_parses_integers() {
        // py:182-185
        let r = _get_bytes_sysfs("1234\n", "5678\n");
        assert_eq!(r, Some((1234, 5678)));
    }

    #[test]
    fn get_bytes_sysfs_returns_none_for_invalid_input() {
        assert!(_get_bytes_sysfs("not-a-number", "5678").is_none());
        assert!(_get_bytes_sysfs("1234", "abc").is_none());
    }
}
