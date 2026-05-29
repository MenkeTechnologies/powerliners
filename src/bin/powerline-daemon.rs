// vim:fileencoding=utf-8:noet
//! `powerline-daemon` binary entry.
//!
//! Wires the already-ported pieces:
//!   - `_find_config_files` + `load_json_config` + `mergedicts` for
//!     config loading
//!   - `Colorscheme::new` for highlight resolution
//!   - `Theme` (constructed inline) for the segment table
//!   - `gen_segment_getter` for segment dict preparation
//!   - `Renderer::render` / `do_render` / `_render_segments` for the
//!     render loop
//!   - `TmuxRenderer::hlstyle` for the `#[fg=...,bg=...]` markup
//!   - `scripts::powerline_daemon::main` for the lifecycle
//!
//! Lives in `src/bin/` (sanctioned non-port location). No new fns
//! land under `src/ported/` from this file.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};

use powerliners::ported::lib::config::load_json_config;
use powerliners::ported::lib::dict::mergedicts;
use powerliners::ported::scripts::powerline_daemon as daemon;
use powerliners::ported::scripts::powerline_daemon::{RenderFn, SpawnWmFn};
use powerliners::ported::{_find_config_files, get_config_paths};
use powerliners::ported::colorscheme::Colorscheme;
use powerliners::ported::renderer::{Renderer, RenderReturn};
use powerliners::ported::renderers::tmux::{ColorSpec, TmuxRenderer};
use powerliners::ported::segment::gen_segment_getter;
use powerliners::ported::theme::Theme;

/// Adapter signature for one built-in segment fn.
/// Reads from `args` (segment kwargs) + `segment_info` (runtime env)
/// and returns either a string (single chunk) or list-of-dicts (multi-
/// chunk) as `Value`.
type AdapterFn = fn(&Map<String, Value>, &Map<String, Value>) -> Option<Value>;

fn search_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(pcp) = std::env::var("POWERLINE_CONFIG_PATHS") {
        for p in pcp.split(':').filter(|s| !s.is_empty()) {
            paths.push(PathBuf::from(p));
        }
    }
    paths.extend(get_config_paths());
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        let bundled = PathBuf::from(manifest).join("vendor/powerline/powerline/config_files");
        if bundled.is_dir() {
            paths.push(bundled);
        }
    }
    paths
}

fn load_one(name: &str, paths: &[PathBuf]) -> Option<Map<String, Value>> {
    let matches = _find_config_files(paths, name).ok()?;
    let p = matches.first()?;
    let v = load_json_config(p).ok()?;
    v.as_object().cloned()
}

fn load_cascade(levels: &[String], paths: &[PathBuf]) -> Option<Map<String, Value>> {
    let mut out: Map<String, Value> = Map::new();
    let mut loaded = 0u32;
    for level in levels {
        if let Ok(matches) = _find_config_files(paths, level) {
            if let Some(p) = matches.first() {
                if let Ok(v) = load_json_config(p) {
                    if let Some(o) = v.as_object().cloned() {
                        mergedicts(&mut out, o, true);
                        loaded += 1;
                    }
                }
            }
        }
    }
    if loaded == 0 {
        None
    } else {
        Some(out)
    }
}

#[derive(Clone)]
struct Configs {
    colorscheme: Arc<Colorscheme>,
    theme: Arc<Theme>,
}

fn build_configs(ext: &str) -> Result<Configs, String> {
    let paths = search_paths();
    let main = load_one("config", &paths).ok_or("config.json not found")?;
    let colors_json = load_one("colors", &paths).ok_or("colors.json not found")?;

    let (cs_name, theme_name) = {
        let mut cs = "default".to_string();
        let mut th = "default".to_string();
        if let Some(ext_block) = main
            .get("ext")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get(ext))
            .and_then(|v| v.as_object())
        {
            if let Some(s) = ext_block.get("colorscheme").and_then(|v| v.as_str()) {
                cs = s.to_string();
            }
            if let Some(s) = ext_block.get("theme").and_then(|v| v.as_str()) {
                th = s.to_string();
            }
        }
        (cs, th)
    };

    let cs_levels = vec![
        format!("colorschemes/{}", cs_name),
        format!("colorschemes/{}/__main__", ext),
        format!("colorschemes/{}/{}", ext, cs_name),
    ];
    let colorscheme_json =
        load_cascade(&cs_levels, &paths).ok_or_else(|| format!("no colorscheme for {}", ext))?;

    let top_theme = main
        .get("common")
        .and_then(|c| c.get("default_top_theme"))
        .and_then(|v| v.as_str())
        .unwrap_or("powerline");
    let theme_levels = vec![
        format!("themes/{}", top_theme),
        format!("themes/{}/{}", ext, theme_name),
    ];
    let theme_json =
        load_cascade(&theme_levels, &paths).ok_or_else(|| format!("no theme for {}", ext))?;

    let colorscheme = Colorscheme::new(&colorscheme_json, &colors_json);

    // Build the Theme.segments table from the theme JSON via gen_segment_getter.
    let get_segment = gen_segment_getter(
        &(),
        ext,
        &Map::new(),
        vec![theme_json.clone()],
        theme_json
            .get("default_module")
            .and_then(|v| v.as_str()),
        |module: &str, name: &str| {
            // Resolve known module.fn pairs to the adapter registry.
            adapter_id(module, name).is_some()
        },
        Some(top_theme),
    );

    let segments_json = theme_json
        .get("segments")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let mut line_map: Map<String, Value> = Map::new();
    for side in ["left", "right"] {
        let mut side_arr: Vec<Value> = Vec::new();
        if let Some(specs) = segments_json.get(side).and_then(|v| v.as_array()) {
            for spec in specs {
                if let Some(spec_obj) = spec.as_object() {
                    if let Some(prepared) = get_segment(spec_obj, side) {
                        side_arr.push(Value::Object(prepared));
                    }
                }
            }
        }
        line_map.insert(side.to_string(), Value::Array(side_arr));
    }

    let dividers = theme_json
        .get("dividers")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let spaces = theme_json
        .get("spaces")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    let outer_padding = theme_json
        .get("outer_padding")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    let mut empty_seg = Map::new();
    empty_seg.insert("contents".to_string(), Value::Null);
    let mut empty_hl = Map::new();
    empty_hl.insert("fg".to_string(), Value::Bool(false));
    empty_hl.insert("bg".to_string(), Value::Bool(false));
    empty_hl.insert("attrs".to_string(), Value::from(0));
    empty_seg.insert("highlight".to_string(), Value::Object(empty_hl));

    let theme = Theme {
        colorscheme: Value::Null,
        dividers,
        cursor_space_multiplier: None,
        cursor_columns: None,
        spaces,
        outer_padding,
        segments: vec![line_map],
        empty_segment: Value::Object(empty_seg),
        shutdown_called: std::sync::Mutex::new(Vec::new()),
    };

    Ok(Configs {
        colorscheme: Arc::new(colorscheme),
        theme: Arc::new(theme),
    })
}

/// Look up the segment id (module + name) in the adapter registry.
/// Returns the canonical "module.name" form on match, None on miss.
fn adapter_id(module: &str, name: &str) -> Option<&'static str> {
    let full = format!("{}.{}", module, name);
    ADAPTERS.iter().find(|(k, _)| *k == full.as_str()).map(|(k, _)| *k)
}

fn invoke_adapter(
    id: &str,
    args: &Map<String, Value>,
    segment_info: &Map<String, Value>,
) -> Option<Value> {
    let entry = ADAPTERS.iter().find(|(k, _)| *k == id)?;
    entry.1(args, segment_info)
}

// =============================================================
// Adapters: each maps a built-in Rust segment fn into the
// dispatcher's uniform signature. Lives in the bin (non-port).
// =============================================================

fn ad_hostname(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::net::hostname;
    let environ = info
        .get("environ")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let only_if_ssh = args
        .get("only_if_ssh")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let exclude_domain = args
        .get("exclude_domain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let s = hostname(&environ, only_if_ssh, exclude_domain, || {
        std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    })?;
    Some(Value::String(s))
}

fn ad_date(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::time::date;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d");
    let istime = args
        .get("istime")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let timezone = args.get("timezone").and_then(|v| v.as_str());
    let chunks = date(&(), format, istime, timezone);
    Some(Value::Array(chunks))
}

fn read_cpu_percent() -> f64 {
    // `top -l 1 -s 0 -n 0` prints one summary, including the CPU usage
    // line. On darwin: `CPU usage: 3.4% user, 5.2% sys, 91.3% idle`.
    // We sum user + sys for the "active" reading. Linux fallback parses
    // /proc/stat first delta over a 100ms sleep.
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("top")
            .args(["-l", "1", "-s", "0", "-n", "0"])
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix("CPU usage: ") {
                    let mut user = 0.0f64;
                    let mut sys = 0.0f64;
                    for part in rest.split(',') {
                        let part = part.trim();
                        if let Some(p) = part.strip_suffix("% user") {
                            user = p.trim().parse().unwrap_or(0.0);
                        } else if let Some(p) = part.strip_suffix("% sys") {
                            sys = p.trim().parse().unwrap_or(0.0);
                        }
                    }
                    return user + sys;
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        let read = || -> Option<(u64, u64)> {
            let s = std::fs::read_to_string("/proc/stat").ok()?;
            let line = s.lines().next()?;
            let parts: Vec<u64> = line
                .split_whitespace()
                .skip(1)
                .filter_map(|p| p.parse().ok())
                .collect();
            let total: u64 = parts.iter().sum();
            let idle = *parts.get(3)?;
            Some((total, idle))
        };
        if let (Some((t1, i1)), _) = (
            read(),
            std::thread::sleep(std::time::Duration::from_millis(100)),
        ) {
            if let Some((t2, i2)) = read() {
                let dt = t2.saturating_sub(t1) as f64;
                let di = i2.saturating_sub(i1) as f64;
                if dt > 0.0 {
                    return 100.0 * (1.0 - di / dt);
                }
            }
        }
    }
    0.0
}

fn ad_cpu_load_percent(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::sys::render as cpu_render;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{0:.0f}%");
    let pct = read_cpu_percent();
    let chunks = cpu_render(pct, format);
    Some(Value::Array(chunks))
}

fn ad_mem_usage(_args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // powerlinemem.mem_usage isn't in upstream; reproduce its typical
    // output (memory used percent) using `vm_stat` on darwin or
    // /proc/meminfo on linux.
    #[cfg(target_os = "macos")]
    let pct = {
        let out = std::process::Command::new("vm_stat").output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut free = 0u64;
        let mut active = 0u64;
        let mut inactive = 0u64;
        let mut wired = 0u64;
        let mut compressed = 0u64;
        let mut page_size: u64 = 4096;
        for line in text.lines() {
            if let Some(p) = line.strip_prefix("Mach Virtual Memory Statistics: (page size of ") {
                if let Some(n) = p.split(' ').next().and_then(|s| s.parse().ok()) {
                    page_size = n;
                }
            }
            let parse = |label: &str| -> Option<u64> {
                line.strip_prefix(label)
                    .and_then(|r| r.trim().trim_end_matches('.').parse().ok())
            };
            if let Some(n) = parse("Pages free:") {
                free = n;
            }
            if let Some(n) = parse("Pages active:") {
                active = n;
            }
            if let Some(n) = parse("Pages inactive:") {
                inactive = n;
            }
            if let Some(n) = parse("Pages wired down:") {
                wired = n;
            }
            if let Some(n) = parse("Pages occupied by compressor:") {
                compressed = n;
            }
        }
        let used = (active + wired + compressed) * page_size;
        let total = (free + active + inactive + wired + compressed) * page_size;
        if total == 0 {
            0.0
        } else {
            100.0 * used as f64 / total as f64
        }
    };
    #[cfg(target_os = "linux")]
    let pct = {
        let s = std::fs::read_to_string("/proc/meminfo").ok()?;
        let mut total = 0u64;
        let mut available = 0u64;
        for line in s.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.first() == Some(&"MemTotal:") {
                total = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            }
            if parts.first() == Some(&"MemAvailable:") {
                available = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
            }
        }
        if total == 0 {
            0.0
        } else {
            100.0 * (total - available) as f64 / total as f64
        }
    };
    Some(Value::Array(vec![serde_json::json!({
        "contents": format!("{:.0}%", pct),
        "highlight_groups": ["mem_usage_gradient", "mem_usage"],
        "gradient_level": pct,
    })]))
}

fn ad_system_load(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::sys::system_load;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{avg:.1f}");
    let threshold_good = args
        .get("threshold_good")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let threshold_bad = args
        .get("threshold_bad")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);
    let track_cpu_count = args
        .get("track_cpu_count")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let short = args
        .get("short")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let _ = (threshold_good, threshold_bad, track_cpu_count, short);
    Some(Value::Array(system_load(
        &(),
        format,
        threshold_good,
        threshold_bad,
        track_cpu_count,
        short,
    )?))
}

fn ad_uptime(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::sys::uptime;
    let days_format = args
        .get("days_format")
        .and_then(|v| v.as_str())
        .unwrap_or("{days:d}d ");
    let hours_format = args
        .get("hours_format")
        .and_then(|v| v.as_str())
        .unwrap_or("{hours:d}h ");
    let minutes_format = args
        .get("minutes_format")
        .and_then(|v| v.as_str())
        .unwrap_or("{minutes:d}m ");
    let seconds_format = args
        .get("seconds_format")
        .and_then(|v| v.as_str())
        .unwrap_or("{seconds:d}s");
    let shorten_len = args
        .get("shorten_len")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;
    let s = uptime(
        &(),
        days_format,
        hours_format,
        minutes_format,
        seconds_format,
        shorten_len,
    )?;
    Some(Value::String(s))
}

fn ad_external_ip(_args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::net::{_external_ip, external_ip_render};
    let ip = _external_ip(|| {
        std::process::Command::new("curl")
            .args(["-s", "https://ipv4.icanhazip.com"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    });
    let chunks = external_ip_render(ip.as_deref())?;
    Some(Value::Array(chunks))
}

fn ad_internal_ip(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    let interface = args
        .get("interface")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    let ipv = args
        .get("ipv")
        .and_then(|v| v.as_u64())
        .map(|n| n as u8)
        .unwrap_or(4);
    let iface = if interface == "auto" {
        // Resolve default route's interface via netstat -rn on darwin /
        // ip route on linux.
        #[cfg(target_os = "macos")]
        {
            let out = std::process::Command::new("netstat")
                .args(["-rn", "-f", "inet"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            let mut found: Option<String> = None;
            for line in text.lines() {
                if line.starts_with("default ") {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    if let Some(name) = cols.get(3) {
                        found = Some(name.to_string());
                        break;
                    }
                }
            }
            found?
        }
        #[cfg(not(target_os = "macos"))]
        {
            let out = std::process::Command::new("ip")
                .args(["route", "show", "default"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            text.split_whitespace()
                .skip_while(|s| *s != "dev")
                .nth(1)?
                .to_string()
        }
    } else {
        interface.to_string()
    };

    let out = std::process::Command::new("ifconfig")
        .arg(&iface)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let needle = if ipv == 6 { "inet6 " } else { "inet " };
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(after) = trimmed.strip_prefix(needle) {
            let ip = after.split_whitespace().next()?.to_string();
            if ipv == 4 && ip == "127.0.0.1" {
                continue;
            }
            return Some(Value::String(ip));
        }
    }
    None
}

fn ad_network_load(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::net::render_one;
    let interface = args
        .get("interface")
        .and_then(|v| v.as_str())
        .unwrap_or("auto")
        .to_string();
    // Darwin: netstat -ib gives per-interface byte counters.
    // Linux: /sys/class/net/<iface>/statistics/{rx,tx}_bytes via _get_bytes_sysfs.
    #[cfg(target_os = "macos")]
    let bytes = {
        let read = || -> Option<(u64, u64)> {
            let out = std::process::Command::new("netstat")
                .args(["-ibn"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.first().map(|c| *c == interface).unwrap_or(false) {
                    let rx: u64 = cols.get(6).and_then(|c| c.parse().ok())?;
                    let tx: u64 = cols.get(9).and_then(|c| c.parse().ok())?;
                    return Some((rx, tx));
                }
            }
            None
        };
        let snap1 = read()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let snap2 = read()?;
        let rx_rate = snap2.0.saturating_sub(snap1.0) as f64 * 2.0;
        let tx_rate = snap2.1.saturating_sub(snap1.1) as f64 * 2.0;
        (rx_rate, tx_rate)
    };
    #[cfg(target_os = "linux")]
    let bytes = {
        let read = || {
            let rx = std::fs::read_to_string(format!(
                "/sys/class/net/{}/statistics/rx_bytes",
                interface
            ))
            .ok()?;
            let tx = std::fs::read_to_string(format!(
                "/sys/class/net/{}/statistics/tx_bytes",
                interface
            ))
            .ok()?;
            Some((rx.trim().parse().ok()?, tx.trim().parse().ok()?))
        };
        let snap1: (u64, u64) = read()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let snap2: (u64, u64) = read()?;
        (
            snap2.0.saturating_sub(snap1.0) as f64 * 2.0,
            snap2.1.saturating_sub(snap1.1) as f64 * 2.0,
        )
    };
    let recv_format = args
        .get("recv_format")
        .and_then(|v| v.as_str())
        .unwrap_or("DL {value:>8}");
    let sent_format = args
        .get("sent_format")
        .and_then(|v| v.as_str())
        .unwrap_or("UL {value:>8}");
    let suffix = args
        .get("suffix")
        .and_then(|v| v.as_str())
        .unwrap_or("B/s");
    let si_prefix = args
        .get("si_prefix")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let recv_max = args
        .get("recv_max")
        .and_then(|v| v.as_f64())
        .unwrap_or(1_000_000.0);
    let sent_max = args
        .get("sent_max")
        .and_then(|v| v.as_f64())
        .unwrap_or(1_000_000.0);
    let _ = bytes; // already-rate values; render_one wants raw snapshots
    // Re-snapshot to feed render_one: it needs (t1, (rx1,tx1)) and (t2, (rx2,tx2)).
    #[cfg(target_os = "macos")]
    let (prev, last) = {
        let read = || -> Option<(f64, (u64, u64))> {
            let out = std::process::Command::new("netstat")
                .args(["-ibn"])
                .output()
                .ok()?;
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines().skip(1) {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.first().map(|c| *c == interface).unwrap_or(false) {
                    let rx: u64 = cols.get(6).and_then(|c| c.parse().ok())?;
                    let tx: u64 = cols.get(9).and_then(|c| c.parse().ok())?;
                    let t = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs_f64();
                    return Some((t, (rx, tx)));
                }
            }
            None
        };
        let p = read()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let l = read()?;
        (p, l)
    };
    #[cfg(target_os = "linux")]
    let (prev, last) = {
        let read = || -> Option<(f64, (u64, u64))> {
            let rx = std::fs::read_to_string(format!(
                "/sys/class/net/{}/statistics/rx_bytes",
                interface
            ))
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()?;
            let tx = std::fs::read_to_string(format!(
                "/sys/class/net/{}/statistics/tx_bytes",
                interface
            ))
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()?;
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs_f64();
            Some((t, (rx, tx)))
        };
        let p = read()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        let l = read()?;
        (p, l)
    };
    let chunks = render_one(
        Some(prev),
        Some(last),
        recv_format,
        sent_format,
        suffix,
        si_prefix,
        Some(recv_max),
        Some(sent_max),
    )?;
    Some(Value::Array(chunks))
}

fn ad_spotify(_args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // Darwin-only AppleScript probe.
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                "if application \"Spotify\" is running then\n\
                 tell application \"Spotify\"\n\
                 if player state is playing then\n\
                 return artist of current track & \" - \" & name of current track\n\
                 end if\n\
                 end tell\n\
                 end if",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        if s.is_empty() {
            return None;
        }
        return Some(Value::Array(vec![serde_json::json!({
            "contents": s,
            "highlight_groups": ["now_playing"],
            "divider_highlight_group": Value::Null,
        })]));
    }
    #[cfg(not(target_os = "macos"))]
    None
}

fn ad_branch(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    let cwd = info
        .get("getcwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let status_colors = args
        .get("status_colors")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let out = std::process::Command::new("git")
        .args(["-C", &cwd, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if branch.is_empty() {
        return None;
    }
    let mut groups: Vec<Value> = vec![Value::String("branch".to_string())];
    if status_colors {
        let dirty = std::process::Command::new("git")
            .args(["-C", &cwd, "status", "--porcelain"])
            .output()
            .ok()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false);
        groups.insert(
            0,
            Value::String(
                if dirty { "branch_dirty" } else { "branch_clean" }.to_string(),
            ),
        );
    }
    Some(Value::Array(vec![serde_json::json!({
        "contents": branch,
        "highlight_groups": groups,
        "divider_highlight_group": Value::Null,
    })]))
}

fn ad_stash(_args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    let cwd = info
        .get("getcwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let out = std::process::Command::new("git")
        .args(["-C", &cwd, "stash", "list"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let count = String::from_utf8(out.stdout)
        .ok()?
        .lines()
        .filter(|l| !l.is_empty())
        .count();
    if count == 0 {
        return None;
    }
    Some(Value::Array(vec![serde_json::json!({
        "contents": format!("{}", count),
        "highlight_groups": ["stash"],
        "divider_highlight_group": Value::Null,
    })]))
}

fn ad_battery(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::bat::{battery, parse_pmset_output};
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{ac_state} {capacity:3.0%}");
    let steps = args
        .get("steps")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as u32;
    let gamify = args
        .get("gamify")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let full_heart = args
        .get("full_heart")
        .and_then(|v| v.as_str())
        .unwrap_or("O");
    let empty_heart = args
        .get("empty_heart")
        .and_then(|v| v.as_str())
        .unwrap_or("O");
    let online = args.get("online").and_then(|v| v.as_str()).unwrap_or("C");
    let offline = args.get("offline").and_then(|v| v.as_str()).unwrap_or(" ");
    let result = battery(
        || {
            let out = std::process::Command::new("pmset")
                .args(["-g", "batt"])
                .output()
                .ok()?;
            let text = String::from_utf8(out.stdout).ok()?;
            let (pct, ac) = parse_pmset_output(&text)?;
            Some((pct as f64, ac))
        },
        format,
        steps,
        gamify,
        full_heart,
        empty_heart,
        online,
        offline,
    )?;
    Some(Value::Array(result))
}

const ADAPTERS: &[(&str, AdapterFn)] = &[
    ("powerline.segments.common.net.hostname", ad_hostname),
    ("powerline.segments.common.time.date", ad_date),
    ("powerline.segments.common.sys.cpu_load_percent", ad_cpu_load_percent),
    ("powerline.segments.common.sys.system_load", ad_system_load),
    ("powerline.segments.common.sys.uptime", ad_uptime),
    ("powerline.segments.common.net.external_ip", ad_external_ip),
    ("powerline.segments.common.net.internal_ip", ad_internal_ip),
    ("powerline.segments.common.vcs.branch", ad_branch),
    ("powerline.segments.common.vcs.stash", ad_stash),
    ("powerline.segments.common.bat.battery", ad_battery),
    ("powerlinemem.mem_usage.mem_usage", ad_mem_usage),
    ("powerline.segments.common.net.network_load", ad_network_load),
    ("powerline.segments.common.players.spotify", ad_spotify),
];

/// Map a `Value`-encoded fg/bg (Python tuple `[cterm, hex]` OR `False`
/// sentinel) into the `ColorSpec` TmuxRenderer wants.
fn color_to_spec(v: &Value) -> Option<ColorSpec> {
    if let Some(b) = v.as_bool() {
        if !b {
            return None;
        }
    }
    if let Some(arr) = v.as_array() {
        let cterm = arr.first().and_then(|x| x.as_u64()).unwrap_or(0) as u16;
        let truecolor = arr.get(1).and_then(|x| x.as_u64()).map(|n| n as u32);
        return Some(ColorSpec { cterm, truecolor });
    }
    None
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // One slot per ext — the daemon caches keyed by PowerlineKey but
    // the configs themselves only depend on `ext`. Lazy-load on first
    // request, then reuse for the daemon's lifetime.
    let store: Arc<Mutex<HashMap<String, Configs>>> = Arc::new(Mutex::new(HashMap::new()));
    let renderer = Arc::new(Renderer::new(Map::new(), Map::new(), 1));
    let tmux = Arc::new(TmuxRenderer::new(false));

    let store_clone = store.clone();
    let renderer_clone = renderer.clone();
    let tmux_clone = tmux.clone();
    let render_fn: Arc<RenderFn> = Arc::new(move |args, environ, cwd, _is_daemon| {
        let ext = args.ext.first().cloned().unwrap_or_default();
        let side = args.side.clone().unwrap_or_default();

        let configs = {
            let mut guard = store_clone.lock().expect("config store poisoned");
            if let Some(c) = guard.get(&ext) {
                c.clone()
            } else {
                match build_configs(&ext) {
                    Ok(c) => {
                        guard.insert(ext.clone(), c.clone());
                        c
                    }
                    Err(e) => {
                        return format!("powerline-daemon: config error: {}\n", e).into_bytes()
                    }
                }
            }
        };

        // Build segment_info from the wire request.
        let mut environ_map: Map<String, Value> = Map::new();
        for (k, v) in environ {
            environ_map.insert(k.clone(), Value::String(v.clone()));
        }
        let mut segment_info: Map<String, Value> = Map::new();
        segment_info.insert("environ".to_string(), Value::Object(environ_map));
        segment_info.insert("home".to_string(), Value::String(environ.get("HOME").cloned().unwrap_or_default()));
        segment_info.insert("getcwd".to_string(), Value::String(cwd.to_string()));

        let tmux_for_hl = tmux_clone.clone();
        let tmux_for_hlstyle = tmux_clone.clone();
        let hl_fn = move |contents: Option<&str>,
                          fg: &Value,
                          bg: &Value,
                          attrs: &Value,
                          _hl_args: &Map<String, Value>|
              -> String {
            let style = tmux_for_hl.hlstyle(
                color_to_spec(fg),
                color_to_spec(bg),
                attrs.as_u64().map(|n| n as u32),
            );
            format!("{}{}", style, contents.unwrap_or(""))
        };
        let hlstyle_fn = move |fg: &Value,
                               bg: &Value,
                               attrs: &Value,
                               _hl_args: &Map<String, Value>|
              -> String {
            tmux_for_hlstyle.hlstyle(
                color_to_spec(fg),
                color_to_spec(bg),
                attrs.as_u64().map(|n| n as u32),
            )
        };

        let contents_func = |id: &str,
                             _pl: &(),
                             si: &Map<String, Value>,
                             args: &Map<String, Value>|
              -> Option<Value> { invoke_adapter(id, args, si) };

        let result = renderer_clone.render(
            None,
            args.width.map(|w| w as usize),
            if side.is_empty() { None } else { Some(&side) },
            0,
            false,
            false,
            Some(segment_info),
            None,
            None,
            &configs.theme,
            &configs.colorscheme,
            &contents_func,
            &hlstyle_fn,
            &hl_fn,
        );

        match result {
            RenderReturn::Plain(s) => s.into_bytes(),
            RenderReturn::Tuple { highlighted, .. } => highlighted.into_bytes(),
        }
    });
    let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_name, _t_evt, _pl_evt| None);
    let code = daemon::main(&argv, render_fn, spawn_wm_fn);
    std::process::exit(code);
}
