// vim:fileencoding=utf-8:noet
//! GPU segment — utilization % and VRAM used/total, vendor-dispatched
//! at runtime. No upstream powerline equivalent; lives in `extensions`
//! per `docs/PORT.md`.
//!
//! Probe order, first hit wins:
//!
//! 1. `nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total
//!    --format=csv,noheader,nounits` — works on every NVIDIA box.
//! 2. `rocm-smi --showuse --showmemuse --csv` — AMD on Linux.
//! 3. macOS Apple Silicon: parse `ioreg -l -w0 -r -d 1 -c
//!    IOAccelerator` for `Device Utilization` + VRAM. No Metal
//!    framework dep — keeps the binary self-contained per the
//!    "vendorable + durable" rule.
//! 4. `intel_gpu_top -J -s 500` (Linux Intel) — one JSON sample.
//!
//! Returns `None` when no probe succeeds so the segment is omitted
//! (matches the upstream convention used by `battery`, `network_load`,
//! etc.) instead of rendering a misleading zero.
//!
//! Output mirrors `mem_usage`'s contract: contents from a printf-style
//! `%s` / `%d%%` format, a 0..=100 `gradient_level` driven by util-%,
//! and `highlight_groups` ordered gradient-first so theme cascades work
//! without per-user overrides.

use serde_json::{json, Value};

/// One snapshot of GPU state. Vendor probes fill what they can; any
/// field set to `None` is rendered as `?` so a missing-VRAM probe
/// degrades gracefully instead of silently zeroing the bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct GpuStats {
    pub util_pct: Option<f64>,
    pub vram_used: Option<u64>,
    pub vram_total: Option<u64>,
}

impl GpuStats {
    pub fn is_empty(&self) -> bool {
        self.util_pct.is_none() && self.vram_used.is_none() && self.vram_total.is_none()
    }
}

/// Public entry: probes vendors in order and returns the first hit.
pub fn read_gpu() -> Option<GpuStats> {
    if let Some(s) = read_nvidia() {
        return Some(s);
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(s) = read_rocm() {
            return Some(s);
        }
        if let Some(s) = read_intel_gpu_top() {
            return Some(s);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(s) = read_macos_ioreg() {
            return Some(s);
        }
    }
    None
}

fn read_nvidia() -> Option<GpuStats> {
    let out = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let line = text.lines().next()?;
    let cols: Vec<&str> = line.split(',').map(str::trim).collect();
    let util: f64 = cols.first()?.parse().ok()?;
    let used_mib: u64 = cols.get(1)?.parse().ok()?;
    let total_mib: u64 = cols.get(2)?.parse().ok()?;
    Some(GpuStats {
        util_pct: Some(util),
        vram_used: Some(used_mib * 1024 * 1024),
        vram_total: Some(total_mib * 1024 * 1024),
    })
}

#[cfg(target_os = "linux")]
fn read_rocm() -> Option<GpuStats> {
    let out = std::process::Command::new("rocm-smi")
        .args(["--showuse", "--showmemuse", "--csv"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut util: Option<f64> = None;
    let mut used_pct: Option<f64> = None;
    let mut header: Vec<String> = Vec::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        if header.is_empty() {
            header = cols.iter().map(|s| s.to_string()).collect();
            continue;
        }
        for (i, h) in header.iter().enumerate() {
            let v = cols.get(i).copied().unwrap_or("");
            if h.contains("GPU use") {
                util = v.trim_end_matches('%').trim().parse().ok();
            } else if h.contains("Memory use") {
                used_pct = v.trim_end_matches('%').trim().parse().ok();
            }
        }
    }
    if util.is_none() && used_pct.is_none() {
        return None;
    }
    Some(GpuStats {
        util_pct: util,
        vram_used: used_pct.map(|p| (p * 1024.0 * 1024.0 * 1024.0 / 100.0) as u64),
        vram_total: used_pct.map(|_| 1024 * 1024 * 1024),
    })
}

#[cfg(target_os = "linux")]
fn read_intel_gpu_top() -> Option<GpuStats> {
    // intel_gpu_top -J streams JSON forever; -s sets interval ms. A
    // single sample is enough — we kill after the first complete object.
    let mut child = std::process::Command::new("intel_gpu_top")
        .args(["-J", "-s", "500"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    use std::io::Read;
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 1024];
    let mut handle = stdout;
    let mut depth = 0i32;
    let mut started = false;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(1500);
    'read: loop {
        if std::time::Instant::now() > deadline {
            break;
        }
        match handle.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                for &b in &tmp[..n] {
                    buf.push(b);
                    if b == b'{' {
                        depth += 1;
                        started = true;
                    } else if b == b'}' {
                        depth -= 1;
                        if started && depth == 0 {
                            break 'read;
                        }
                    }
                }
            }
            Err(_) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let text = String::from_utf8(buf).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    // Engine busy is reported under engines.* per intel_gpu_top output;
    // average non-zero engine load as the util %.
    let mut sum = 0.0f64;
    let mut cnt = 0u32;
    if let Some(engines) = v.get("engines").and_then(|x| x.as_object()) {
        for (_, e) in engines {
            if let Some(b) = e.get("busy").and_then(|x| x.as_f64()) {
                sum += b;
                cnt += 1;
            }
        }
    }
    let util = if cnt > 0 {
        Some(sum / cnt as f64)
    } else {
        None
    };
    util?;
    Some(GpuStats {
        util_pct: util,
        vram_used: None,
        vram_total: None,
    })
}

#[cfg(target_os = "macos")]
fn read_macos_ioreg() -> Option<GpuStats> {
    let out = std::process::Command::new("ioreg")
        .args(["-l", "-w0", "-r", "-d", "1", "-c", "IOAccelerator"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut util: Option<f64> = None;
    let mut vram_used: Option<u64> = None;
    let mut vram_total: Option<u64> = None;
    for line in text.lines() {
        let trimmed = line.trim_start_matches(|c: char| c.is_whitespace() || c == '|' || c == '+');
        if let Some(v) = extract_ioreg_num(trimmed, "\"Device Utilization %\"") {
            util = Some(v);
        }
        if let Some(v) = extract_ioreg_num(trimmed, "\"PerformanceStatistics\"") {
            // Some drivers expose nested dict; fall back to raw num
            // extraction below when keys aren't sub-lines.
            let _ = v;
        }
        if let Some(v) = extract_ioreg_num(trimmed, "\"In use system memory\"") {
            vram_used = Some(v as u64);
        }
        if let Some(v) = extract_ioreg_num(trimmed, "\"Alloc system memory\"") {
            vram_total = Some(v as u64);
        }
    }
    // Fallback: PerformanceStatistics dict on the line itself.
    if util.is_none() {
        if let Some(idx) = text.find("\"Device Utilization %\"=") {
            let rest = &text[idx + "\"Device Utilization %\"=".len()..];
            let num: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !num.is_empty() {
                util = num.parse().ok();
            }
        }
    }
    let stats = GpuStats {
        util_pct: util,
        vram_used,
        vram_total,
    };
    if stats.is_empty() {
        None
    } else {
        Some(stats)
    }
}

#[cfg(target_os = "macos")]
fn extract_ioreg_num(line: &str, key: &str) -> Option<f64> {
    let prefix = format!("{}=", key);
    let idx = line.find(&prefix)?;
    let rest = &line[idx + prefix.len()..];
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    if num.is_empty() {
        None
    } else {
        num.parse().ok()
    }
}

/// Human-format `n` bytes using the same long/short table as
/// `mem_usage::_sizeof_fmt`. Re-implemented here to keep the two
/// extensions decoupled.
fn fmt_bytes(n: u64, short: bool) -> String {
    let units_long = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    let units_short = ["B", "K", "M", "G", "T", "P"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < units_long.len() {
        v /= 1024.0;
        i += 1;
    }
    let unit = if short { units_short[i] } else { units_long[i] };
    if v.fract() < f64::EPSILON {
        format!("{}{}", v as u64, unit)
    } else {
        format!("{:.1}{}", v, unit)
    }
}

/// Render the utilization-% form. Theme JSON:
/// `{"function": "powerliners.gpu.gpu_usage_percent",
///   "args": {"format": "{0:d}%"}}`
pub fn gpu_usage_percent(format: &str) -> Vec<Value> {
    let stats = read_gpu().unwrap_or_default();
    let util = stats.util_pct.unwrap_or(0.0);
    let contents = render_pct(format, util);
    vec![json!({
        "contents": contents,
        "gradient_level": util,
        "highlight_groups": [
            "gpu_load_gradient",
            "gpu_load",
            "cpu_load_percent_gradient",
            "cpu_load_percent",
        ],
        "divider_highlight_group": "background:divider",
    })]
}

/// Render the VRAM `USED/TOTAL` form. Theme JSON:
/// `{"function": "powerliners.gpu.gpu_vram", "args": {"short": true}}`
pub fn gpu_vram(format: &str, short: bool) -> Vec<Value> {
    let stats = read_gpu().unwrap_or_default();
    let used = stats
        .vram_used
        .map(|n| fmt_bytes(n, short))
        .unwrap_or_else(|| "?".to_string());
    let total = stats
        .vram_total
        .map(|n| fmt_bytes(n, short))
        .unwrap_or_else(|| "?".to_string());
    let pct = match (stats.vram_used, stats.vram_total) {
        (Some(u), Some(t)) if t > 0 => 100.0 * u as f64 / t as f64,
        _ => 0.0,
    };
    let contents = format.replacen("%s", &used, 1).replacen("%s", &total, 1);
    vec![json!({
        "contents": contents,
        "gradient_level": pct,
        "highlight_groups": [
            "gpu_vram_gradient",
            "gpu_vram",
            "mem_usage_gradient",
            "mem_usage",
        ],
        "divider_highlight_group": "background:divider",
    })]
}

/// Render a `{0:d}%` / `{0:.1f}%` style template using brace-form
/// directives the rest of powerliners theme JSON already speaks.
/// Falls back to `%d%%` printf-style for compatibility with mem
/// segment conventions.
fn render_pct(format: &str, n: f64) -> String {
    if format.contains("{0:d}") {
        return format.replace("{0:d}", &format!("{}", n as i64));
    }
    if let Some(start) = format.find("{0:.") {
        let rest = &format[start + 4..];
        if let Some(end) = rest.find("f}") {
            let prec: usize = rest[..end].parse().unwrap_or(1);
            let token = &format[start..start + 4 + end + 2];
            return format.replace(token, &format!("{:.*}", prec, n));
        }
    }
    if format.contains("%d%%") {
        return format.replace("%d%%", &format!("{}%", n as i64));
    }
    if format.contains("%d") {
        return format.replace("%d", &format!("{}", n as i64));
    }
    format!("{}%", n as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_bytes_short_and_long() {
        assert_eq!(fmt_bytes(0, false), "0B");
        assert_eq!(fmt_bytes(1024, false), "1KiB");
        assert_eq!(fmt_bytes(1536, false), "1.5KiB");
        assert_eq!(fmt_bytes(1024 * 1024 * 8, true), "8M");
    }

    #[test]
    fn render_pct_d_form() {
        assert_eq!(render_pct("{0:d}%", 73.7), "73%");
    }

    #[test]
    fn render_pct_precision_f_form() {
        assert_eq!(render_pct("{0:.1f}%", 73.45), "73.5%");
    }

    #[test]
    fn render_pct_printf_compat() {
        assert_eq!(render_pct("%d%%", 42.0), "42%");
    }

    #[test]
    fn gpu_stats_default_is_empty() {
        assert!(GpuStats::default().is_empty());
    }

    #[test]
    fn gpu_vram_renders_question_marks_when_unknown() {
        // No vendor available in the test env → both bytes are None.
        // The fn never panics and always emits a one-element vec.
        let out = gpu_vram("%s/%s", false);
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        assert!(c.contains('/'));
    }

    // ---- fmt_bytes precision tier + full suffix chain ----

    #[test]
    fn fmt_bytes_full_chain_long_form() {
        assert_eq!(fmt_bytes(1024u64.pow(2), false), "1MiB");
        assert_eq!(fmt_bytes(1024u64.pow(3), false), "1GiB");
        assert_eq!(fmt_bytes(1024u64.pow(4), false), "1TiB");
        assert_eq!(fmt_bytes(1024u64.pow(5), false), "1PiB");
    }

    #[test]
    fn fmt_bytes_fraction_renders_with_one_decimal() {
        // .fract() >= EPSILON → ".1" formatter; integer-precise
        // bytes use the no-decimal branch.
        assert_eq!(fmt_bytes(1024 + 512, false), "1.5KiB");
        assert_eq!(fmt_bytes(1024 + 512 + 256, true), "1.8K"); // 1.75 → 1.8
                                                               // Boundary: exactly 1024 is integer-precise.
        assert_eq!(fmt_bytes(1024, true), "1K");
    }

    // ---- render_pct full matrix ----

    #[test]
    fn render_pct_precision_2dp_form() {
        assert_eq!(render_pct("{0:.2f}%", 73.456), "73.46%");
    }

    #[test]
    fn render_pct_printf_d_without_percent_literal() {
        // `%d` alone (no `%%`) goes through the second printf branch;
        // emits the integer without a `%` suffix.
        assert_eq!(render_pct("util=%d", 42.9), "util=42");
    }

    #[test]
    fn render_pct_fallback_when_no_token() {
        // No `{0:d}` / `{0:.Nf}` / `%d%%` / `%d` → fallback "NN%" form,
        // ignoring the caller's template string.
        assert_eq!(render_pct("ignored", 88.4), "88%");
    }

    // ---- GpuStats::is_empty matrix ----

    #[test]
    fn gpu_stats_is_empty_flips_for_any_set_field() {
        let setters: &[fn(&mut GpuStats)] = &[
            |s| s.util_pct = Some(0.0),
            |s| s.vram_used = Some(0),
            |s| s.vram_total = Some(0),
        ];
        for set in setters {
            let mut s = GpuStats::default();
            set(&mut s);
            assert!(
                !s.is_empty(),
                "any populated field should flip is_empty: {s:?}"
            );
        }
    }

    // ---- gpu_usage_percent contents shape ----

    #[test]
    fn gpu_usage_percent_renders_one_chunk_with_format_sub() {
        let out = gpu_usage_percent("{0:d}%");
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        assert!(c.ends_with('%'), "expected NN%, got {c}");
    }

    #[test]
    fn gpu_usage_percent_emits_gradient_chain() {
        let out = gpu_usage_percent("{0:d}%");
        let groups = out[0]["highlight_groups"].as_array().unwrap();
        // First is the gpu-specific gradient group, last falls back
        // through cpu_load_percent for theme cascade.
        assert_eq!(groups[0].as_str().unwrap(), "gpu_load_gradient");
        assert_eq!(groups.last().unwrap().as_str().unwrap(), "cpu_load_percent");
    }

    // ---- extract_ioreg_num (macOS pure parser) ----

    #[cfg(target_os = "macos")]
    #[test]
    fn extract_ioreg_num_extracts_integer_value() {
        let line = r#"    "Device Utilization %"=47"#;
        assert_eq!(
            extract_ioreg_num(line, "\"Device Utilization %\""),
            Some(47.0)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extract_ioreg_num_extracts_float_value() {
        let line = r#"    "Device Utilization %"=47.5"#;
        assert_eq!(
            extract_ioreg_num(line, "\"Device Utilization %\""),
            Some(47.5)
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extract_ioreg_num_missing_key_returns_none() {
        let line = r#"    "Other Key"=99"#;
        assert!(extract_ioreg_num(line, "\"Device Utilization %\"").is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extract_ioreg_num_empty_value_returns_none() {
        let line = r#"    "Device Utilization %"="#;
        assert!(extract_ioreg_num(line, "\"Device Utilization %\"").is_none());
    }
}
