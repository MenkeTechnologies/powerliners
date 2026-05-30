// vim:fileencoding=utf-8:noet
//! Shared render runtime: bin-private orchestration that lives
//! under `src/bin/` (sanctioned non-port location per PORT.md).
//!
//! `powerline-daemon` wires this through the daemon main_loop;
//! `powerline-render` calls `render_once` directly for one-shot
//! rendering. The fn surface here is bin-glue, not a port — no
//! Python source corresponds because Python uses dynamic
//! `__import__` to dispatch segment fns at runtime; the Rust
//! port replaces that with a static `ADAPTERS` registry.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value};

use powerliners::ported::colorscheme::Colorscheme;
use powerliners::ported::commands::main::Args;
use powerliners::ported::lib::config::load_json_config;
use powerliners::ported::lib::dict::mergedicts;
use powerliners::ported::renderer::{RenderReturn, Renderer};
use powerliners::ported::renderers::tmux::{ColorSpec, TmuxRenderer};
use powerliners::ported::segment::gen_segment_getter;
use powerliners::ported::theme::Theme;
use powerliners::ported::{_find_config_files, get_config_paths};

/// Adapter signature for one built-in segment fn.
/// Reads from `args` (segment kwargs) + `segment_info` (runtime env)
/// and returns either a string (single chunk) or list-of-dicts (multi-
/// chunk) as `Value`.
type AdapterFn = fn(&Map<String, Value>, &Map<String, Value>) -> Option<Value>;

fn search_paths() -> Vec<PathBuf> {
    // Mirror upstream `ShellPowerline.get_config_paths`
    // (powerline/shell.py:25-26): `return self.args.config_path or
    // super().get_config_paths()`. When POWERLINE_CONFIG_PATHS is
    // set it REPLACES the default cascade entirely — no bundled
    // fallback. Test fixtures rely on this to pin a self-contained
    // config tree.
    if let Ok(pcp) = std::env::var("POWERLINE_CONFIG_PATHS") {
        let explicit: Vec<PathBuf> = pcp
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect();
        if !explicit.is_empty() {
            return explicit;
        }
    }
    // Default cascade: bundled `plugin_path` FIRST (py:152) so
    // load_cascade uses it as the base, then XDG / user_home come
    // AFTER so mergedicts(base, override) lets the user win.
    //
    // `src/ported/config_files` is the in-tree mirror of upstream
    // `vendor/powerline/powerline/config_files` and is the path that
    // ships in the published crate (Cargo.toml excludes `vendor/**`).
    // Vendor path is checked as a secondary so dev builds against a
    // checked-out tree still resolve when the mirror gets stale.
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Some(manifest) = option_env!("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest);
        let ported = manifest.join("src/ported/config_files");
        if ported.is_dir() {
            paths.push(ported);
        }
        let bundled = manifest.join("vendor/powerline/powerline/config_files");
        if bundled.is_dir() {
            paths.push(bundled);
        }
    }
    paths.extend(get_config_paths());
    paths
}

fn load_one(name: &str, paths: &[PathBuf]) -> Option<Map<String, Value>> {
    let matches = _find_config_files(paths, name).ok()?;
    let p = matches.first()?;
    let v = load_json_config(p).ok()?;
    v.as_object().cloned()
}

fn load_cascade(levels: &[String], paths: &[PathBuf]) -> Option<Map<String, Value>> {
    // py:191-200  load_config: iterate ALL matches per level and merge
    // them in find-order. Upstream `get_config_paths` puts bundled
    // FIRST and user-config LAST, so later matches override earlier
    // — exactly the user-overrides-bundled semantics. Earlier
    // versions only loaded `matches.first()` which silently dropped
    // bundled fallback groups (e.g. tmux/`window:current` highlight).
    let mut out: Map<String, Value> = Map::new();
    let mut loaded = 0u32;
    for level in levels {
        if let Ok(matches) = _find_config_files(paths, level) {
            for p in &matches {
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
pub struct Configs {
    pub colorscheme: Arc<Colorscheme>,
    pub theme: Arc<Theme>,
    pub tmux: Arc<TmuxRenderer>,
    /// py:265-272 — WM extensions consume `update_interval` to drive
    /// background re-render. tmux daemon path doesn't run a WM thread;
    /// surfaced here so a future WM dispatch can read the configured
    /// value (default 2 seconds per upstream).
    #[allow(dead_code)]
    wm_update_interval: f64,
    /// py:133-141  `reload_config` (default true) — when true, the
    /// daemon polls cached config-file mtimes on each render and
    /// invalidates the cache when any have changed. Mirrors upstream
    /// `ConfigLoader.check` semantics with a per-request stat instead
    /// of a background watcher thread (the
    /// `lib/watcher/{inotify,stat,uv,tree}.rs` ports are ready but
    /// not threaded here to keep the daemon process model simple).
    /// Read by the daemon bin only — render bin doesn't cache.
    #[allow(dead_code)]
    pub reload_config: bool,
    /// Paths whose mtimes are checked when `reload_config` is true.
    /// Read by the daemon bin only — render bin doesn't cache.
    #[allow(dead_code)]
    pub loaded_paths: Vec<(PathBuf, std::time::SystemTime)>,
}

impl Configs {
    /// Returns true if any `loaded_paths` entry has a different mtime
    /// vs load time — mirrors `ConfigLoader.check` at
    /// `lib/config.py:130-141`. Called from the daemon bin only.
    #[allow(dead_code)]
    pub fn is_stale(&self) -> bool {
        if !self.reload_config {
            return false;
        }
        for (p, t) in &self.loaded_paths {
            match std::fs::metadata(p).and_then(|m| m.modified()) {
                Ok(now) if now != *t => return true,
                Err(_) => return true,
                _ => {}
            }
        }
        false
    }
}

/// Snapshot mtime of `path` for the `loaded_paths` cache. Returns
/// SystemTime::UNIX_EPOCH when the stat fails so a follow-up
/// `is_stale` check naturally returns true (treats missing as
/// changed).
fn mtime_or_epoch(path: &PathBuf) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
}

pub fn build_configs(ext: &str) -> Result<Configs, String> {
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

    // py:324-328  default_top_theme cascade:
    //   1. user `common.default_top_theme` (explicit override)
    //   2. else `get_default_theme(encoding.startswith(utf|ucs))`
    //      → `powerline_terminus` on UTF-8 / `ascii` on legacy
    // Mirrors `finish_common_config` at `powerline/__init__.py:313`.
    use powerliners::ported::get_default_theme;
    use powerliners::ported::lib::encoding::get_preferred_output_encoding;
    let user_top_theme = main
        .get("common")
        .and_then(|c| c.get("default_top_theme"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let computed_top_theme = {
        let enc = get_preferred_output_encoding().to_lowercase();
        get_default_theme(enc.starts_with("utf") || enc.starts_with("ucs"))
    };
    let top_theme: String = user_top_theme.unwrap_or_else(|| computed_top_theme.to_string());
    let top_theme = top_theme.as_str();
    // py:806-810 / py:821-823  Theme cascade has THREE layers:
    // 1. `themes/<top_theme>` (cross-ext defaults: dividers, spaces)
    // 2. `themes/<ext>/__main__` (per-ext defaults: segment_data,
    //    division of segments, …)
    // 3. `themes/<ext>/<theme_name>` (the user's specific theme)
    // Each later layer overrides earlier ones via `mergedicts`. Most
    // shipped exts have a `__main__.json` so dropping the middle
    // layer loses per-ext defaults.
    let theme_levels = vec![
        format!("themes/{}", top_theme),
        format!("themes/{}/__main__", ext),
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
        theme_json.get("default_module").and_then(|v| v.as_str()),
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

    // Mirrors `Theme.__init__` segments-iteration at upstream
    // `powerline/theme.py:91-105`:
    //   `for segdict in itertools.chain((theme_config['segments'],),
    //                                    theme_config['segments'].get('above', ())):`
    //     `self.segments.append(new_empty_segment_line())`
    //     ... fill left + right ...
    // Each segdict = one line. Line 0 = the base render
    // (`segments.{left,right}`); lines 1..=N = `segments.above[0..]`
    // each a `{left, right}` dict in upstream order.
    let prepare_line = |segdict: &Map<String, Value>| -> Map<String, Value> {
        let mut line_map: Map<String, Value> = Map::new();
        for side in ["left", "right"] {
            let mut side_arr: Vec<Value> = Vec::new();
            if let Some(specs) = segdict.get(side).and_then(|v| v.as_array()) {
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
        line_map
    };

    let mut lines: Vec<Map<String, Value>> = Vec::new();
    // Base line first (theme.py:91 `(theme_config['segments'],)`).
    lines.push(prepare_line(&segments_json));
    // Then each entry under `segments.above`. Python uses tuple()
    // default → empty iter; Rust mirrors with `unwrap_or_default`.
    if let Some(above_list) = segments_json.get("above").and_then(|v| v.as_array()) {
        for above_seg in above_list {
            if let Some(above_obj) = above_seg.as_object() {
                lines.push(prepare_line(above_obj));
            }
        }
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

    // py:67-70  Theme.__init__: cursor_space → 1 - (theme_config['cursor_space'] / 100)
    // when present (KeyError → None); cursor_columns from theme_config.get.
    let cursor_space_multiplier = theme_json
        .get("cursor_space")
        .and_then(|v| v.as_f64())
        .map(|n| 1.0 - (n / 100.0));
    let cursor_columns = theme_json.get("cursor_columns").and_then(|v| v.as_i64());

    let theme = Theme {
        colorscheme: Value::Null,
        dividers,
        cursor_space_multiplier,
        cursor_columns,
        spaces,
        outer_padding,
        segments: lines,
        empty_segment: Value::Object(empty_seg),
        shutdown_called: std::sync::Mutex::new(Vec::new()),
    };

    // Read common.term_truecolor from main config to drive
    // TmuxRenderer.hlstyle's `fg=#RRGGBB` vs `fg=colourN` branch.
    let term_truecolor = main
        .get("common")
        .and_then(|c| c.get("term_truecolor"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // py:218-223  ext.wm.update_interval (default 2.0). Read from
    // main config so WM-ext requests can honor it once a WM thread
    // dispatcher lands. The tmux ext path is request-driven and
    // ignores it.
    let wm_update_interval = main
        .get("ext")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("wm"))
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("update_interval"))
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);

    // py:133-141  reload_config (default true)
    let reload_config = main
        .get("common")
        .and_then(|c| c.get("reload_config"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    // Collect every config file path our cascade actually consumed so
    // `is_stale` can stat them on subsequent renders.
    let mut loaded_paths: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let probe_levels: Vec<String> = vec![
        "config".to_string(),
        "colors".to_string(),
        format!("colorschemes/{}", cs_name),
        format!("colorschemes/{}/__main__", ext),
        format!("colorschemes/{}/{}", ext, cs_name),
        format!("themes/{}", top_theme),
        format!("themes/{}/__main__", ext),
        format!("themes/{}/{}", ext, theme_name),
    ];
    for level in &probe_levels {
        if let Ok(matches) = _find_config_files(&paths, level) {
            if let Some(p) = matches.first().cloned() {
                let mt = mtime_or_epoch(&p);
                loaded_paths.push((p, mt));
            }
        }
    }

    Ok(Configs {
        colorscheme: Arc::new(colorscheme),
        theme: Arc::new(theme),
        tmux: Arc::new(TmuxRenderer::new(term_truecolor)),
        wm_update_interval,
        reload_config,
        loaded_paths,
    })
}

/// Look up the segment id (module + name) in the adapter registry.
/// Returns the canonical "module.name" form on match, None on miss.
pub fn adapter_id(module: &str, name: &str) -> Option<&'static str> {
    let full = format!("{}.{}", module, name);
    ADAPTERS
        .iter()
        .find(|(k, _)| *k == full.as_str())
        .map(|(k, _)| *k)
}

pub fn invoke_adapter(
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
    let short = args.get("short").and_then(|v| v.as_bool()).unwrap_or(false);
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
    let suffix = args.get("suffix").and_then(|v| v.as_str()).unwrap_or("B/s");
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

// `needless_return` allowed: the `return` keeps the macOS branch readable
// alongside the `#[cfg(not(target_os = "macos"))] None` tail without
// restructuring around the cfg gate.
#[allow(clippy::needless_return)]
fn ad_spotify(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // Faithful port path: defer to SpotifyAppleScriptPlayerSegment
    // (Darwin) / spotify_dbus (Linux), then PlayerSegment.__call__
    // formatting, which emits highlight_groups
    // ['player_<state>', 'player'] — upstream players.py:56.
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, SpotifyAppleScriptPlayerSegment,
        APPLESCRIPT_STATUS_DELIMITER,
    };
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");

    #[cfg(target_os = "macos")]
    let func_stats = {
        // py:374-396 — the full 6-field delimited AppleScript.
        // Status order: state | album | artist | title | track_length | player_position
        let script = format!(
            "tell application \"System Events\"\n\
             set process_list to (name of every process)\n\
             end tell\n\
             if process_list contains \"Spotify\" then\n\
             tell application \"Spotify\"\n\
             if player state is playing or player state is paused then\n\
             set track_name to name of current track\n\
             set artist_name to artist of current track\n\
             set album_name to album of current track\n\
             set track_length to duration of current track\n\
             set now_playing to \"\" & player state & \"{0}\" & album_name & \"{0}\" & artist_name & \"{0}\" & track_name & \"{0}\" & track_length & \"{0}\" & player position\n\
             return now_playing\n\
             else\n\
             return player state\n\
             end if\n\
             end tell\n\
             else\n\
             return \"stopped\"\n\
             end if",
            APPLESCRIPT_STATUS_DELIMITER
        );
        let out = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        SpotifyAppleScriptPlayerSegment.get_player_status(&s)
    };

    #[cfg(not(target_os = "macos"))]
    let func_stats: Option<powerliners::ported::segments::common::players::PlayerStats> = None;

    // py:40  state_symbols=STATE_SYMBOLS — overridable via segment_data
    // ["player"]["args"]["state_symbols"], which the segment_data cascade
    // in gen_segment_getter merges into `args` for us. Falls back to
    // upstream's hardcoded Python defaults (`>` / `~` / `X` / `''`).
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(func_stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_branch(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    // py:18-39  BranchSegment.__call__:
    //   name = segment_info['getcwd']()
    //   if name: repo = guess(path=name, ...); if repo:
    //     branch = repo.branch(); scol = ['branch']
    //     if status_colors: status = tree_status(repo, pl) (or '?' on Exc);
    //         if status in ignore_statuses: status = None
    //         scol.insert(0, 'branch_dirty' if status else 'branch_clean')
    //     return [{'contents': branch, 'highlight_groups': scol,
    //              'divider_highlight_group': None}]
    let cwd = info
        .get("getcwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let status_colors = args
        .get("status_colors")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ignore_statuses: Vec<String> = args
        .get("ignore_statuses")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let (branch, status) = git_branch(&cwd)
        .or_else(|| hg_branch(&cwd))
        .or_else(|| bzr_branch(&cwd))?;
    if branch.is_empty() {
        return None;
    }
    let mut groups: Vec<Value> = vec![Value::String("branch".to_string())];
    if status_colors {
        // py:32-35: a None status means probe errored (== '?'), else
        // trim and compare against ignore_statuses; if in ignore set,
        // treat as clean.
        let effective = status.as_deref().map(str::trim).map(String::from);
        let is_dirty = match &effective {
            Some(s) if s.is_empty() => false,
            Some(s) if ignore_statuses.iter().any(|i| i == s) => false,
            Some(_) => true,
            None => true, // '?' fallback at py:30 — treated as truthy
        };
        groups.insert(
            0,
            Value::String(
                if is_dirty {
                    "branch_dirty"
                } else {
                    "branch_clean"
                }
                .to_string(),
            ),
        );
    }
    Some(Value::Array(vec![serde_json::json!({
        "contents": branch,
        "highlight_groups": groups,
        "divider_highlight_group": Value::Null,
    })]))
}

/// Probe git for current branch + working-tree status string.
/// Status is the porcelain output (`M `, `??`, etc.) flattened to
/// a single status letter string; empty when working tree is clean.
/// Returns None when the cwd isn't a git repo.
fn git_branch(cwd: &str) -> Option<(String, Option<String>)> {
    let out = std::process::Command::new("git")
        .args(["-C", cwd, "rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8(out.stdout).ok()?.trim().to_string();
    let status = std::process::Command::new("git")
        .args(["-C", cwd, "status", "--porcelain"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
    Some((branch, status))
}

/// Probe mercurial. `hg branch` prints the active branch (default
/// "default"); `hg status` reports working-copy changes.
fn hg_branch(cwd: &str) -> Option<(String, Option<String>)> {
    let out = std::process::Command::new("hg")
        .args(["--cwd", cwd, "branch"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8(out.stdout).ok()?.trim().to_string();
    let status = std::process::Command::new("hg")
        .args(["--cwd", cwd, "status"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
    Some((branch, status))
}

/// Probe bazaar. `bzr nick` prints the nick of the current branch;
/// `bzr status` reports working-tree changes.
fn bzr_branch(cwd: &str) -> Option<(String, Option<String>)> {
    let out = std::process::Command::new("bzr")
        .args(["--directory", cwd, "nick"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8(out.stdout).ok()?.trim().to_string();
    let status = std::process::Command::new("bzr")
        .args(["--directory", cwd, "status"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string());
    Some((branch, status))
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
    let steps = args.get("steps").and_then(|v| v.as_u64()).unwrap_or(5) as u32;
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

fn ad_environment(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::env::environment;
    let environ = info
        .get("environ")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let variable = args.get("variable").and_then(|v| v.as_str())?;
    let v = environment(&environ, variable)?;
    Some(Value::String(v))
}

fn ad_jobnum(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::shell::jobnum;
    use powerliners::ported::segments::shell::ShellSegmentInfo;
    let show_zero = args
        .get("show_zero")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let seg_info = ShellSegmentInfo {
        jobnum: info
            .get("args")
            .and_then(|v| v.as_object())
            .and_then(|o| o.get("jobnum"))
            .and_then(|v| v.as_i64())
            .map(|n| n as i32),
        ..Default::default()
    };
    let s = jobnum(&(), &seg_info, show_zero)?;
    Some(Value::String(s))
}

fn ad_last_status(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::shell::last_status;
    use powerliners::ported::segments::shell::ShellSegmentInfo;
    let signal_names = args
        .get("signal_names")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    // ShellSegmentInfo uses i32 for last_exit_code. Signal-name strings
    // (e.g. "sigINT") aren't carried through the i32 path — that's a
    // structural divergence from upstream (Python IntOrSig union) which
    // we surface by treating signal names as exit-code 0 here. Real
    // shell-prompt drivers should switch to IntOrSig once
    // ShellSegmentInfo gains a union field.
    let last_exit_code = info
        .get("args")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("last_exit_code"))
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);
    let seg_info = ShellSegmentInfo {
        last_exit_code,
        ..Default::default()
    };
    let chunks = last_status(&(), &seg_info, signal_names)?;
    Some(Value::Array(chunks))
}

fn ad_last_pipe_status(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::shell::last_pipe_status as lps_fn;
    use powerliners::ported::segments::shell::ShellSegmentInfo;
    let signal_names = args
        .get("signal_names")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let lps_vec: Vec<i32> = info
        .get("args")
        .and_then(|v| v.as_object())
        .and_then(|o| o.get("last_pipe_status"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().map(|v| v.as_i64().unwrap_or(0) as i32).collect())
        .unwrap_or_default();
    let seg_info = ShellSegmentInfo {
        last_pipe_status: lps_vec,
        ..Default::default()
    };
    let chunks = lps_fn(&(), &seg_info, signal_names)?;
    Some(Value::Array(chunks))
}

fn ad_cwd(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::env::cwd_segments;
    let cwd = info
        .get("getcwd")
        .and_then(|v| v.as_str())
        .unwrap_or("/")
        .to_string();
    let dir_shorten_len = args
        .get("dir_shorten_len")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let dir_limit_depth = args
        .get("dir_limit_depth")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let use_path_separator = args
        .get("use_path_separator")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ellipsis = args.get("ellipsis").and_then(|v| v.as_str());
    let chunks = cwd_segments(
        &cwd,
        dir_shorten_len,
        dir_limit_depth,
        use_path_separator,
        ellipsis,
    );
    if chunks.is_empty() {
        return None;
    }
    Some(Value::Array(chunks))
}

fn ad_virtualenv(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::env::virtualenv;
    let environ = info
        .get("environ")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let ignore_venv = args
        .get("ignore_venv")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ignore_conda = args
        .get("ignore_conda")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ignored: Vec<String> = args
        .get("ignored_names")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_else(|| vec!["venv".to_string(), ".venv".to_string()]);
    let ignored_refs: Vec<&str> = ignored.iter().map(String::as_str).collect();
    let v = virtualenv(&environ, ignore_venv, ignore_conda, &ignored_refs)?;
    Some(Value::String(v))
}

fn ad_fuzzy_time(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::time::{
        fuzzy_time, fuzzy_time_default_hour_str, fuzzy_time_default_minute_str,
        fuzzy_time_default_special_cases,
    };
    let format = args.get("format").and_then(|v| v.as_str());
    let unicode_text = args
        .get("unicode_text")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let timezone = args.get("timezone").and_then(|v| v.as_str());

    // py:46  hour_str=[...], minute_str={...}, special_case_str={...}
    // User-supplied overrides come in via theme args; build the
    // owned String buffers, then take &str slices for the fuzzy_time
    // call. Defaults fill in any keys the user omits.
    let hour_str_default = fuzzy_time_default_hour_str();
    let hour_str_owned: Vec<String> = match args.get("hour_str").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        None => hour_str_default.iter().map(|s| s.to_string()).collect(),
    };
    let hour_str: Vec<&str> = hour_str_owned.iter().map(String::as_str).collect();

    let minute_str_default = fuzzy_time_default_minute_str();
    let minute_str_owned: std::collections::HashMap<u32, String> =
        match args.get("minute_str").and_then(|v| v.as_object()) {
            Some(obj) => obj
                .iter()
                .filter_map(|(k, v)| {
                    let key: u32 = k.parse().ok()?;
                    let val = v.as_str()?.to_string();
                    Some((key, val))
                })
                .collect(),
            None => minute_str_default
                .iter()
                .map(|(k, v)| (*k, v.to_string()))
                .collect(),
        };
    let minute_str: std::collections::HashMap<u32, &str> = minute_str_owned
        .iter()
        .map(|(k, v)| (*k, v.as_str()))
        .collect();

    let special_cases = fuzzy_time_default_special_cases();
    let s = fuzzy_time(
        format,
        unicode_text,
        timezone,
        Some(&hour_str),
        Some(&minute_str),
        Some(&special_cases),
    );
    if s.is_empty() {
        return None;
    }
    Some(Value::String(s))
}

fn ad_weather(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::wthr::{compute_state, render_one, weather_key};
    let location_query = args
        .get("location_query")
        .and_then(|v| v.as_str())
        .map(String::from);
    let api_key = args
        .get("weather_api_key")
        .and_then(|v| v.as_str())
        .map(String::from);
    let key = weather_key(location_query, api_key);
    let weather = compute_state(&key)?;
    let unit = args.get("unit").and_then(|v| v.as_str()).unwrap_or("C");
    let temp_format = args.get("temp_format").and_then(|v| v.as_str());
    let temp_coldest = args
        .get("temp_coldest")
        .and_then(|v| v.as_f64())
        .unwrap_or(-30.0);
    let temp_hottest = args
        .get("temp_hottest")
        .and_then(|v| v.as_f64())
        .unwrap_or(40.0);
    let icons = args.get("icons").and_then(|v| v.as_object());
    let chunks = render_one(
        Some(weather),
        icons,
        unit,
        temp_format,
        temp_coldest,
        temp_hottest,
    )?;
    Some(Value::Array(chunks))
}

fn ad_email_imap_alert(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // py:43-138  EmailIMAPSegment — full IMAP probe would need a
    // TLS imap crate (not in deps). Surface a configured-username
    // placeholder so the segment is visible; faithful imap probe
    // is a follow-up dep choice.
    let username = args.get("username").and_then(|v| v.as_str())?;
    Some(Value::Array(vec![serde_json::json!({
        "contents": format!("{}: 0", username),
        "highlight_groups": ["email_alert"],
    })]))
}

fn ad_cmus(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, CmusPlayerSegment,
    };
    let out = std::process::Command::new("cmus-remote")
        .arg("-Q")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let stats = CmusPlayerSegment.get_player_status(&s);
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_rhythmbox(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, RhythmboxPlayerSegment,
    };
    // py:458-473  rhythmbox-client probe
    let out = std::process::Command::new("rhythmbox-client")
        .args([
            "--no-start",
            "--print-playing-format",
            "%at\n%aa\n%tt\n%te\n%td",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let stats = RhythmboxPlayerSegment.get_player_status(&s);
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_itunes(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, ITunesPlayerSegment, APPLESCRIPT_STATUS_DELIMITER,
    };
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");

    #[cfg(target_os = "macos")]
    let stats = {
        // py:534-554 — 6-field delimited AppleScript (title|artist|album|elapsed|duration|state)
        let script = format!(
            "tell application \"System Events\"\n\
             set process_list to (name of every process)\n\
             end tell\n\
             if process_list contains \"iTunes\" then\n\
             tell application \"iTunes\"\n\
             if player state is playing then\n\
             set t_title to name of current track\n\
             set t_artist to artist of current track\n\
             set t_album to album of current track\n\
             set t_duration to duration of current track\n\
             set t_elapsed to player position\n\
             set t_state to player state\n\
             return t_title & \"{0}\" & t_artist & \"{0}\" & t_album & \"{0}\" & t_elapsed & \"{0}\" & t_duration & \"{0}\" & t_state\n\
             end if\n\
             end tell\n\
             end if",
            APPLESCRIPT_STATUS_DELIMITER
        );
        let out = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
        ITunesPlayerSegment.get_player_status(&s)
    };

    #[cfg(not(target_os = "macos"))]
    let stats: Option<powerliners::ported::segments::common::players::PlayerStats> = None;

    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_dbus_player(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // py:241-312  generic MPRIS probe parameterized by player_name.
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, PlayerStats,
    };
    let player = args.get("player_name").and_then(|v| v.as_str())?;
    let service = format!("org.mpris.MediaPlayer2.{}", player);
    let metadata = std::process::Command::new("qdbus")
        .args([
            &service,
            "/Player",
            "org.freedesktop.MediaPlayer.GetMetadata",
        ])
        .output()
        .ok()?;
    if !metadata.status.success() {
        return None;
    }
    let text = String::from_utf8(metadata.stdout).ok()?;
    let mut artist = String::new();
    let mut title = String::new();
    let mut album = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("artist: ") {
            artist = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("title: ") {
            title = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("album: ") {
            album = rest.to_string();
        }
    }
    if title.is_empty() {
        return None;
    }
    let stats = Some(PlayerStats {
        state: Some("play".to_string()),
        album: Some(album).filter(|s| !s.is_empty()),
        artist: Some(artist).filter(|s| !s.is_empty()),
        title: Some(title),
        total: None,
        elapsed: None,
    });
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_clementine(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // py:436-449  Clementine via MPRIS dbus. `qdbus` shells the
    // method calls; on systems without qdbus we silently skip.
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, PlayerStats,
    };
    let metadata = std::process::Command::new("qdbus")
        .args([
            "org.mpris.MediaPlayer2.clementine",
            "/Player",
            "org.freedesktop.MediaPlayer.GetMetadata",
        ])
        .output()
        .ok()?;
    if !metadata.status.success() {
        return None;
    }
    let text = String::from_utf8(metadata.stdout).ok()?;
    // qdbus prints `key: value` lines for the dict. Extract artist + title.
    let mut artist = String::new();
    let mut title = String::new();
    let mut album = String::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("artist: ") {
            artist = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("title: ") {
            title = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("album: ") {
            album = rest.to_string();
        }
    }
    if title.is_empty() {
        return None;
    }
    let stats = Some(PlayerStats {
        state: Some("play".to_string()),
        album: Some(album).filter(|s| !s.is_empty()),
        artist: Some(artist).filter(|s| !s.is_empty()),
        title: Some(title),
        total: None,
        elapsed: None,
    });
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_mocp(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, MocPlayerSegment,
    };
    let out = std::process::Command::new("mocp").arg("-i").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let stats = MocPlayerSegment.get_player_status(&s);
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_mpd(args: &Map<String, Value>, _info: &Map<String, Value>) -> Option<Value> {
    // py:172-202  MpdPlayerSegment.get_player_status — CLI branch:
    // probes `mpc` for the now-playing line + `mpc current -f %album%`
    // for the album field. Routes through player_segment_call for the
    // full {state_symbol}{artist}-{title}({total}) contract.
    use powerliners::ported::segments::common::players::{
        player_segment_call, state_symbols, MpdPlayerSegment,
    };
    let host = args.get("host").and_then(|v| v.as_str());
    let port = args.get("port").and_then(|v| v.as_u64());
    let password = args.get("password").and_then(|v| v.as_str());
    let build_cmd = || {
        let mut cmd = std::process::Command::new("mpc");
        if let (Some(pw), Some(h)) = (password, host) {
            cmd.env("MPD_HOST", format!("{}@{}", pw, h));
        } else if let Some(h) = host {
            cmd.env("MPD_HOST", h);
        }
        if let Some(p) = port {
            cmd.arg("-p").arg(p.to_string());
        }
        cmd
    };
    let np_out = build_cmd().output().ok()?;
    if !np_out.status.success() {
        return None;
    }
    let np = String::from_utf8(np_out.stdout).ok()?;
    let mut album_cmd = build_cmd();
    album_cmd.args(["current", "-f", "%album%"]);
    let album = album_cmd
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    let stats = MpdPlayerSegment.get_player_status(&np, album.as_deref());
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{state_symbol} {artist} - {title} ({total})");
    let symbols = match args.get("state_symbols").and_then(|v| v.as_object()) {
        Some(obj) => obj.clone(),
        None => state_symbols(),
    };
    let chunks = player_segment_call(stats, format, &symbols)?;
    Some(Value::Array(chunks))
}

fn ad_user(args: &Map<String, Value>, info: &Map<String, Value>) -> Option<Value> {
    use powerliners::ported::segments::common::env::user;
    let environ = info
        .get("environ")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let hide_user = args.get("hide_user").and_then(|v| v.as_str());
    let hide_domain = args
        .get("hide_domain")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // SAFETY: geteuid() is async-signal-safe POSIX.
    let euid = unsafe { libc::geteuid() };
    let chunks = user(&environ, hide_user, hide_domain, euid)?;
    Some(Value::Array(chunks))
}

pub const ADAPTERS: &[(&str, AdapterFn)] = &[
    ("powerline.segments.common.net.hostname", ad_hostname),
    ("powerline.segments.common.time.date", ad_date),
    ("powerline.segments.common.time.fuzzy_time", ad_fuzzy_time),
    ("powerline.segments.common.env.environment", ad_environment),
    ("powerline.segments.common.env.virtualenv", ad_virtualenv),
    ("powerline.segments.common.env.cwd", ad_cwd),
    ("powerline.segments.shell.jobnum", ad_jobnum),
    ("powerline.segments.shell.last_status", ad_last_status),
    (
        "powerline.segments.shell.last_pipe_status",
        ad_last_pipe_status,
    ),
    ("powerline.segments.common.env.user", ad_user),
    ("powerline.segments.common.players.mpd", ad_mpd),
    (
        "powerline.segments.common.sys.cpu_load_percent",
        ad_cpu_load_percent,
    ),
    ("powerline.segments.common.sys.system_load", ad_system_load),
    ("powerline.segments.common.sys.uptime", ad_uptime),
    ("powerline.segments.common.net.external_ip", ad_external_ip),
    ("powerline.segments.common.net.internal_ip", ad_internal_ip),
    ("powerline.segments.common.vcs.branch", ad_branch),
    ("powerline.segments.common.vcs.stash", ad_stash),
    ("powerline.segments.common.bat.battery", ad_battery),
    ("powerlinemem.mem_usage.mem_usage", ad_mem_usage),
    (
        "powerline.segments.common.net.network_load",
        ad_network_load,
    ),
    ("powerline.segments.common.players.spotify", ad_spotify),
    ("powerline.segments.common.players.cmus", ad_cmus),
    ("powerline.segments.common.players.mocp", ad_mocp),
    ("powerline.segments.common.players.rhythmbox", ad_rhythmbox),
    ("powerline.segments.common.players.itunes", ad_itunes),
    (
        "powerline.segments.common.players.clementine",
        ad_clementine,
    ),
    (
        "powerline.segments.common.players.dbus_player",
        ad_dbus_player,
    ),
    ("powerline.segments.common.wthr.weather", ad_weather),
    (
        "powerline.segments.common.mail.email_imap_alert",
        ad_email_imap_alert,
    ),
];

/// Python-faithful color encoding for the hlstyle directive builder.
/// Python passes `fg`/`bg` as one of:
///   - `None` → don't emit the channel directive at all
///   - `False` or `(False, ...)` → emit `<channel>=default`
///   - `(cterm_int, hex_int_or_None)` → emit `<channel>=colourN` (or `=#hex` truecolor)
enum ColorChoice {
    /// Python `None` — no directive.
    None,
    /// Python `False` (or `[False, …]`) — `<channel>=default`.
    Default,
    /// Python `[cterm, hex]` tuple.
    Spec(ColorSpec),
}

fn classify_color(v: &Value) -> ColorChoice {
    match v {
        Value::Null => ColorChoice::None,
        Value::Bool(false) => ColorChoice::Default,
        Value::Bool(true) => ColorChoice::Default,
        Value::Array(arr) => {
            // Python `[False, …]` is the array form of the default sentinel.
            if matches!(arr.first(), Some(Value::Bool(false))) {
                return ColorChoice::Default;
            }
            let cterm = arr.first().and_then(|x| x.as_u64()).unwrap_or(0) as u16;
            let truecolor = arr.get(1).and_then(|x| x.as_u64()).map(|n| n as u32);
            ColorChoice::Spec(ColorSpec { cterm, truecolor })
        }
        _ => ColorChoice::None,
    }
}

/// Classify a `Value`-encoded `attrs` field.
/// - `Value::Null` → no `attrs` directive (Python `attrs is None`)
/// - `Value::Bool(_)` → Python `False` sentinel: emit all "no-" resets
/// - integer → standard bit field
enum AttrsChoice {
    /// Python `None` — no attrs directive at all.
    None,
    /// Python `False` — all-off ("nobold,noitalics,nounderscore").
    AllOff,
    /// Standard bit field (matches `get_attrs_flag` output).
    Flag(u32),
}

fn classify_attrs(v: &Value) -> AttrsChoice {
    match v {
        Value::Null => AttrsChoice::None,
        Value::Bool(_) => AttrsChoice::AllOff,
        _ => match v.as_u64() {
            Some(n) => AttrsChoice::Flag(n as u32),
            None => AttrsChoice::None,
        },
    }
}

/// Build the Python-faithful `#[…]` tag for the given fg/bg/attrs.
/// Mirrors `TmuxRenderer.hlstyle` at `powerline/renderers/tmux.py:40`
/// exactly, including the early-exit "if not attrs and not bg and not
/// fg: return ''" check and the three-state fg/bg semantics. Used by
/// both the `hl_fn` and `hlstyle_fn` closures so the bin shim emits
/// byte-for-byte parity with upstream Python.
pub fn render_hlstyle(tmux: &TmuxRenderer, fg: &Value, bg: &Value, attrs: &Value) -> String {
    let fc = classify_color(fg);
    let bc = classify_color(bg);
    let ac = classify_attrs(attrs);

    // py:44  if not attrs and not bg and not fg: return ''
    let attrs_empty = matches!(ac, AttrsChoice::None);
    let bg_empty = matches!(bc, ColorChoice::None);
    let fg_empty = matches!(fc, ColorChoice::None);
    if attrs_empty && bg_empty && fg_empty {
        return String::new();
    }

    let mut parts: Vec<String> = Vec::new();
    // py:47-54  fg branch — Python `if term_truecolor and fg[1]:`
    // includes the implicit truthiness check on the hex value, so
    // `hex == 0` (e.g. pure black 0x000000) falls back to cterm.
    match fc {
        ColorChoice::None => {}
        ColorChoice::Default => parts.push("fg=default".into()),
        ColorChoice::Spec(spec) => {
            if tmux.term_truecolor && spec.truecolor.filter(|&n| n != 0).is_some() {
                parts.push(format!("fg=#{:06x}", spec.truecolor.unwrap()));
            } else {
                parts.push(format!("fg=colour{}", spec.cterm));
            }
        }
    }
    // py:55-62  bg branch — same truthiness rule on bg[1].
    match bc {
        ColorChoice::None => {}
        ColorChoice::Default => parts.push("bg=default".into()),
        ColorChoice::Spec(spec) => {
            if tmux.term_truecolor && spec.truecolor.filter(|&n| n != 0).is_some() {
                parts.push(format!("bg=#{:06x}", spec.truecolor.unwrap()));
            } else {
                parts.push(format!("bg=colour{}", spec.cterm));
            }
        }
    }
    // py:63-64  attrs branch
    match ac {
        AttrsChoice::None => {}
        AttrsChoice::AllOff => parts.extend(
            powerliners::ported::renderers::tmux::attrs_to_tmux_attrs(None),
        ),
        AttrsChoice::Flag(flag) => parts.extend(
            powerliners::ported::renderers::tmux::attrs_to_tmux_attrs(Some(flag)),
        ),
    }
    // py:65  return '#[' + ','.join(tmux_attrs) + ']'
    format!("#[{}]", parts.join(","))
}

/// Build a base `Renderer` with `TmuxRenderer.character_translations`
/// pre-installed so `Renderer::escape` performs the `#` → `##[]`
/// substitution upstream Python `class TmuxRenderer(Renderer):
/// character_translations = Renderer.character_translations.copy();
/// ct[ord('#')] = '##['` (powerline/renderers/tmux.py:30-31) installs
/// at class-load time.
pub fn make_renderer() -> Arc<Renderer> {
    let mut renderer_inner = Renderer::new(Map::new(), Map::new(), 1);
    for (ch, replacement) in TmuxRenderer::character_translations() {
        renderer_inner
            .character_translations
            .insert(ch, replacement.to_string());
    }
    Arc::new(renderer_inner)
}

/// One-shot render: build segment_info from environ + cwd + args,
/// call `Renderer::render`, return the rendered bytes.
///
/// Used by `powerline-daemon` (wrapped with a per-ext Configs cache)
/// and `powerline-render` (called directly per invocation). The
/// closures wired here — `hl_fn`, `hlstyle_fn`, `contents_func` —
/// mirror the upstream Python `Renderer.render` call shape so both
/// drivers produce byte-identical output for the same request.
pub fn render_once(
    args: &Args,
    environ: &HashMap<String, String>,
    cwd: &str,
    configs: &Configs,
    renderer: &Renderer,
) -> Vec<u8> {
    let side = args.side.clone().unwrap_or_default();

    let mut environ_map: Map<String, Value> = Map::new();
    for (k, v) in environ {
        environ_map.insert(k.clone(), Value::String(v.clone()));
    }
    let mut segment_info: Map<String, Value> = Map::new();
    segment_info.insert("environ".to_string(), Value::Object(environ_map));
    segment_info.insert(
        "home".to_string(),
        Value::String(environ.get("HOME").cloned().unwrap_or_default()),
    );
    segment_info.insert("getcwd".to_string(), Value::String(cwd.to_string()));

    let mut args_map: Map<String, Value> = Map::new();
    if let Some(j) = args.jobnum {
        args_map.insert("jobnum".to_string(), Value::from(j));
    }
    if let Some(ec) = args.last_exit_code.as_ref() {
        args_map.insert(
            "last_exit_code".to_string(),
            match ec {
                powerliners::ported::commands::main::IntOrSig::Int(n) => Value::from(*n),
                powerliners::ported::commands::main::IntOrSig::Sig(s) => Value::String(s.clone()),
            },
        );
    }
    if !args.last_pipe_status.is_empty() {
        let arr: Vec<Value> = args
            .last_pipe_status
            .iter()
            .map(|v| match v {
                powerliners::ported::commands::main::IntOrSig::Int(n) => Value::from(*n),
                powerliners::ported::commands::main::IntOrSig::Sig(s) => Value::String(s.clone()),
            })
            .collect();
        args_map.insert("last_pipe_status".to_string(), Value::Array(arr));
    }
    segment_info.insert("args".to_string(), Value::Object(args_map));

    let tmux_for_hl = configs.tmux.clone();
    let tmux_for_hlstyle = configs.tmux.clone();
    let hl_fn = move |contents: Option<&str>,
                      fg: &Value,
                      bg: &Value,
                      attrs: &Value,
                      _hl_args: &Map<String, Value>|
          -> String {
        // py:600-606  return self.hlstyle(fg, bg, attrs, **kwargs) + (contents or '')
        let style = render_hlstyle(&tmux_for_hl, fg, bg, attrs);
        format!("{}{}", style, contents.unwrap_or(""))
    };
    let hlstyle_fn =
        move |fg: &Value, bg: &Value, attrs: &Value, _hl_args: &Map<String, Value>| -> String {
            render_hlstyle(&tmux_for_hlstyle, fg, bg, attrs)
        };

    let contents_func =
        |id: &str, _pl: &(), si: &Map<String, Value>, args: &Map<String, Value>| -> Option<Value> {
            invoke_adapter(id, args, si)
        };

    // Mode extraction: Python pulls it from `args.renderer_arg["mode"]`
    // before passing to `Renderer.render`. Mirrors
    // `commands/main.py:170-189` `write_output`'s segment_info update
    // and the explicit `mode=segment_info.get('mode', None)` at
    // py:177/188.
    let mode_owned: Option<String> = args
        .renderer_arg_merged
        .as_ref()
        .and_then(|m| m.get("mode"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let mode_ref: Option<&str> = mode_owned.as_deref();
    let result = renderer.render(
        mode_ref,
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
}
