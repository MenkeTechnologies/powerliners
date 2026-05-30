// vim:fileencoding=utf-8:noet
//! Port of `mKaloer/powerline_mem_segment::powerlinemem.mem_usage`.
//!
//! Source: <https://github.com/mKaloer/powerline_mem_segment> at
//! `powerlinemem/mem_usage.py`. Third-party powerline plugin that
//! upstream powerline-status itself does NOT bundle — but theme
//! authors reference it by name (`powerlinemem.mem_usage.mem_usage`
//! and three sibling fns). Ported here so users can reference those
//! dotted-path keys in their theme JSON without pip-installing the
//! Python plugin alongside the Rust daemon.
//!
//! Upstream uses `psutil.virtual_memory()` / `psutil.swap_memory()`
//! which read `vm_stat` on Darwin, `/proc/meminfo` on Linux, etc.
//! This port collects the same fields via subprocess probes so it
//! works without any Python or psutil dependency.

use serde_json::{json, Value};

/// Port of `_sizeof_fmt()` from `powerlinemem/mem_usage.py:3`.
///
/// Converts a byte value into a human-readable string. Matches
/// upstream's no-space formatting (`2.0KiB`, `3.4MiB`) — distinct
/// from `lib/humanize_bytes::humanize_bytes` which emits
/// `2.0 KiB` (space-separated, used by upstream powerline's own
/// `humanize_bytes`). The plugin's helper is preserved verbatim.
pub fn _sizeof_fmt(num: f64, short: bool, suffix: &str) -> String {
    // py:15-16  if short: suffix = ''
    let suffix: &str = if short { "" } else { suffix };
    // py:17  units long/short pairs
    let units: &[(&str, &str)] = &[
        ("", ""),
        ("Ki", "K"),
        ("Mi", "M"),
        ("Gi", "G"),
        ("Ti", "T"),
        ("Pi", "P"),
        ("Ei", "E"),
        ("Zi", "Z"),
    ];
    let mut n = num;
    for (long, short_u) in units {
        // py:18  if abs(num) < 1024.0:
        if n.abs() < 1024.0 {
            let unit = if short { *short_u } else { *long };
            // py:19  ("%3.1f%s%s" if num else "%d%s%s") % (num, unit, suffix)
            if n != 0.0 {
                return format!("{:3.1}{}{}", n, unit, suffix);
            } else {
                return format!("{}{}{}", n as i64, unit, suffix);
            }
        }
        // py:20  num /= 1024.0
        n /= 1024.0;
    }
    // py:21  Yi / Y fallback
    let unit = if short { "Y" } else { "Yi" };
    if n != 0.0 {
        format!("{:.1}{}{}", n, unit, suffix)
    } else {
        format!("{}{}{}", n as i64, unit, suffix)
    }
}

/// Stats returned by `virtual_memory()` / `swap_memory()`.
/// Field names mirror `psutil`'s `svmem` / `sswap` namedtuples.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemStats {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub available: u64,
}

impl MemStats {
    /// Port of `_get_mem_used()` from `powerlinemem/mem_usage.py:23`.
    ///
    /// Reads the `mem_type` field by name with fallback to `.used`.
    /// `mem_type` accepts `"used"` / `"free"` / `"available"` /
    /// `"total"`.
    pub fn _get_mem_used(&self, mem_type: &str) -> u64 {
        // py:24  getattr(mem_data, mem_type, None)
        match mem_type {
            "used" => self.used,
            "free" => self.free,
            "available" => self.available,
            "total" => self.total,
            // py:25-26  fallback to .used when attr missing
            _ => self.used,
        }
    }
}

/// Port of `psutil.virtual_memory()`. Collects RAM stats by probing
/// the OS — `vm_stat` on macOS, `/proc/meminfo` on Linux.
pub fn virtual_memory() -> MemStats {
    #[cfg(target_os = "macos")]
    {
        read_vm_stat().unwrap_or_default()
    }
    #[cfg(target_os = "linux")]
    {
        read_proc_meminfo().unwrap_or_default()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    MemStats::default()
}

/// Port of `psutil.swap_memory()`.
pub fn swap_memory() -> MemStats {
    #[cfg(target_os = "macos")]
    {
        read_swapusage().unwrap_or_default()
    }
    #[cfg(target_os = "linux")]
    {
        read_proc_meminfo_swap().unwrap_or_default()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    MemStats::default()
}

#[cfg(target_os = "macos")]
fn read_vm_stat() -> Option<MemStats> {
    let out = std::process::Command::new("vm_stat").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut free: u64 = 0;
    let mut active: u64 = 0;
    let mut inactive: u64 = 0;
    let mut speculative: u64 = 0;
    let mut wired: u64 = 0;
    let mut compressed: u64 = 0;
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
        if let Some(n) = parse("Pages speculative:") {
            speculative = n;
        }
        if let Some(n) = parse("Pages wired down:") {
            wired = n;
        }
        if let Some(n) = parse("Pages occupied by compressor:") {
            compressed = n;
        }
    }
    // Total physical memory from sysctl (in bytes).
    let total = std::process::Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or_else(|| {
            (free + active + inactive + speculative + wired + compressed) * page_size
        });
    let used_pages = active + wired + compressed;
    let free_bytes = (free + speculative) * page_size;
    let available_bytes = (free + inactive + speculative) * page_size;
    Some(MemStats {
        total,
        used: used_pages * page_size,
        free: free_bytes,
        available: available_bytes,
    })
}

#[cfg(target_os = "macos")]
fn read_swapusage() -> Option<MemStats> {
    let out = std::process::Command::new("sysctl")
        .args(["-n", "vm.swapusage"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // e.g. "total = 2048.00M  used = 1023.50M  free = 1024.50M  (encrypted)"
    let parse_mb = |label: &str| -> Option<u64> {
        let idx = text.find(label)?;
        let rest = &text[idx + label.len()..];
        let val: f64 = rest.trim_start().split('M').next()?.trim().parse().ok()?;
        Some((val * 1024.0 * 1024.0) as u64)
    };
    let total = parse_mb("total = ").unwrap_or(0);
    let used = parse_mb("used = ").unwrap_or(0);
    let free = parse_mb("free = ").unwrap_or(0);
    Some(MemStats {
        total,
        used,
        free,
        available: free,
    })
}

#[cfg(target_os = "linux")]
fn read_proc_meminfo() -> Option<MemStats> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    let mut available_kb: u64 = 0;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("");
        let val: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        match key {
            "MemTotal:" => total_kb = val,
            "MemFree:" => free_kb = val,
            "MemAvailable:" => available_kb = val,
            _ => {}
        }
    }
    let kb = 1024_u64;
    Some(MemStats {
        total: total_kb * kb,
        used: total_kb.saturating_sub(available_kb) * kb,
        free: free_kb * kb,
        available: available_kb * kb,
    })
}

#[cfg(target_os = "linux")]
fn read_proc_meminfo_swap() -> Option<MemStats> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb: u64 = 0;
    let mut free_kb: u64 = 0;
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next().unwrap_or("");
        let val: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        match key {
            "SwapTotal:" => total_kb = val,
            "SwapFree:" => free_kb = val,
            _ => {}
        }
    }
    let kb = 1024_u64;
    let total = total_kb * kb;
    let free = free_kb * kb;
    Some(MemStats {
        total,
        used: total.saturating_sub(free),
        free,
        available: free,
    })
}

/// Port of `mem_usage()` from `powerlinemem/mem_usage.py:29`.
///
/// Returns a 1-element Vec of the segment dict the daemon's renderer
/// consumes. `format` is a printf-style template — upstream uses
/// `"%s/%s"` to interleave `_sizeof_fmt(used)` and `_sizeof_fmt(total)`.
/// `short=true` swaps the unit table from long-form (`Ki/Mi/Gi`) to
/// short-form (`K/M/G`).
pub fn mem_usage(format: &str, mem_type: &str, short: bool) -> Vec<Value> {
    // py:30  mem_data = psutil.virtual_memory()
    let mem_data = virtual_memory();
    // py:31  mem_used = _get_mem_used(mem_data, mem_type)
    let mem_used = mem_data._get_mem_used(mem_type);
    // py:32  mem_percentage = (float(mem_used) / mem_data.total) * 100
    let mem_percentage = if mem_data.total > 0 {
        (mem_used as f64 / mem_data.total as f64) * 100.0
    } else {
        0.0
    };
    // py:35  format % (_sizeof_fmt(mem_used, short), _sizeof_fmt(mem_data.total, short))
    let contents = sprintf_two_strings(
        format,
        &_sizeof_fmt(mem_used as f64, short, "B"),
        &_sizeof_fmt(mem_data.total as f64, short, "B"),
    );
    // py:33-39  return [{contents, gradient_level, highlight_groups, divider_highlight_group}]
    vec![json!({
        "contents": contents,
        "gradient_level": mem_percentage,
        "highlight_groups": ["mem_usage_gradient", "mem_usage"],
        "divider_highlight_group": "background:divider",
    })]
}

/// Port of `mem_usage_percent()` from `powerlinemem/mem_usage.py:42`.
///
/// Returns the percentage form (default `"%d%%"`).
pub fn mem_usage_percent(format: &str, mem_type: &str) -> Vec<Value> {
    let mem_data = virtual_memory();
    let mem_used = mem_data._get_mem_used(mem_type);
    let mem_percentage = if mem_data.total > 0 {
        (mem_used as f64 / mem_data.total as f64) * 100.0
    } else {
        0.0
    };
    // py:48  format % (mem_percentage,)
    let contents = sprintf_one_float(format, mem_percentage);
    vec![json!({
        "contents": contents,
        "gradient_level": mem_percentage,
        "highlight_groups": ["mem_usage_gradient", "mem_usage"],
        "divider_highlight_group": "background:divider",
    })]
}

/// Port of `mem_swap()` from `powerlinemem/mem_usage.py:55`.
pub fn mem_swap(format: &str, mem_type: &str, short: bool) -> Vec<Value> {
    // py:56  mem_data = psutil.swap_memory()
    let mem_data = swap_memory();
    let mem_used = mem_data._get_mem_used(mem_type);
    // py:58  mem_percentage = ((float(mem_used) / mem_data.total) * 100) if mem_data.total else 0
    let mem_percentage = if mem_data.total > 0 {
        (mem_used as f64 / mem_data.total as f64) * 100.0
    } else {
        0.0
    };
    // py:60-62  try two-arg format; on TypeError fall back to single-arg
    let used_str = _sizeof_fmt(mem_used as f64, short, "B");
    let total_str = _sizeof_fmt(mem_data.total as f64, short, "B");
    let formatted = if format.matches("%s").count() >= 2 {
        sprintf_two_strings(format, &used_str, &total_str)
    } else {
        sprintf_two_strings(format, &used_str, "")
    };
    vec![json!({
        "contents": formatted,
        "gradient_level": mem_percentage,
        "highlight_groups": ["mem_usage_gradient", "mem_usage"],
        "divider_highlight_group": "background:divider",
    })]
}

/// Port of `mem_swap_percentage()` from
/// `powerlinemem/mem_usage.py:72`.
pub fn mem_swap_percentage(format: &str, mem_type: &str) -> Vec<Value> {
    let mem_data = swap_memory();
    let mem_used = mem_data._get_mem_used(mem_type);
    let mem_percentage = if mem_data.total > 0 {
        (mem_used as f64 / mem_data.total as f64) * 100.0
    } else {
        0.0
    };
    let contents = sprintf_one_float(format, mem_percentage);
    vec![json!({
        "contents": contents,
        "gradient_level": mem_percentage,
        "highlight_groups": ["mem_usage_gradient", "mem_usage"],
        "divider_highlight_group": "background:divider",
    })]
}

// printf-format helpers — the upstream plugin uses Python's `%`
// operator with `%s`, `%d`, `%d%%`. Replicate the two narrow shapes
// used so output is byte-identical without dragging in a printf crate.

fn sprintf_two_strings(format: &str, a: &str, b: &str) -> String {
    let mut out = String::with_capacity(format.len() + a.len() + b.len());
    let mut chars = format.chars().peekable();
    let mut subs = [a, b].into_iter();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.peek() {
                Some('s') => {
                    chars.next();
                    if let Some(arg) = subs.next() {
                        out.push_str(arg);
                    }
                }
                Some('%') => {
                    chars.next();
                    out.push('%');
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn sprintf_one_float(format: &str, n: f64) -> String {
    // Supports the four shapes the plugin actually uses with `%d%%`,
    // `%d`, `%.Nf%%`, `%.Nf`, `%s`. The plugin never combines them so
    // the first match wins.
    let bytes = format.as_bytes();
    let mut out = String::with_capacity(format.len() + 8);
    let mut i = 0;
    let mut consumed = false;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'd' => {
                    if !consumed {
                        out.push_str(&format!("{}", n as i64));
                        consumed = true;
                    }
                    i += 2;
                    continue;
                }
                b's' => {
                    if !consumed {
                        out.push_str(&format!("{}", n));
                        consumed = true;
                    }
                    i += 2;
                    continue;
                }
                b'%' => {
                    out.push('%');
                    i += 2;
                    continue;
                }
                b'.' => {
                    // Parse `.Nf` precision spec.
                    let mut j = i + 2;
                    let mut prec_str = String::new();
                    while j < bytes.len() && bytes[j].is_ascii_digit() {
                        prec_str.push(bytes[j] as char);
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'f' {
                        let prec: usize = prec_str.parse().unwrap_or(1);
                        if !consumed {
                            out.push_str(&format!("{:.*}", prec, n));
                            consumed = true;
                        }
                        i = j + 1;
                        continue;
                    }
                    // Unrecognized `.X` — emit '%' literal and advance.
                    out.push('%');
                    i += 1;
                    continue;
                }
                _ => {
                    out.push('%');
                    i += 1;
                    continue;
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizeof_fmt_zero_uses_d_format() {
        // py:19  '%d%s%s' when num is 0
        assert_eq!(_sizeof_fmt(0.0, false, "B"), "0B");
        assert_eq!(_sizeof_fmt(0.0, true, "B"), "0");
    }

    #[test]
    fn sizeof_fmt_one_byte_under_kib() {
        // py:19  '%3.1f%s%s' for nonzero under 1024
        assert_eq!(_sizeof_fmt(512.0, false, "B"), "512.0B");
    }

    #[test]
    fn sizeof_fmt_one_kib_long_form() {
        assert_eq!(_sizeof_fmt(1024.0, false, "B"), "1.0KiB");
    }

    #[test]
    fn sizeof_fmt_one_mib_short_form() {
        assert_eq!(_sizeof_fmt(1024.0 * 1024.0, true, "B"), "1.0M");
    }

    #[test]
    fn get_mem_used_dispatches_on_field_name() {
        let s = MemStats {
            total: 1000,
            used: 700,
            free: 200,
            available: 300,
        };
        assert_eq!(s._get_mem_used("used"), 700);
        assert_eq!(s._get_mem_used("free"), 200);
        assert_eq!(s._get_mem_used("available"), 300);
        assert_eq!(s._get_mem_used("total"), 1000);
        // unknown field falls back to .used per py:25-26
        assert_eq!(s._get_mem_used("frobnicate"), 700);
    }

    #[test]
    fn sprintf_two_strings_handles_default_format() {
        assert_eq!(
            sprintf_two_strings("%s/%s", "512MiB", "8GiB"),
            "512MiB/8GiB"
        );
    }

    #[test]
    fn sprintf_one_float_handles_percent_d_percent() {
        assert_eq!(sprintf_one_float("%d%%", 73.0), "73%");
    }

    #[test]
    fn sprintf_one_float_handles_precision_f() {
        // 73.5 → "73.5" (no rounding needed); banker's rounding is
        // covered by an explicit case below.
        assert_eq!(sprintf_one_float("%.1f", 73.5), "73.5");
    }

    #[test]
    fn sprintf_one_float_uses_bankers_rounding_matching_python() {
        // Python: "%.1f" % 73.25 == "73.2" (round half to even).
        // Rust's format! also uses round_ties_even since 1.77.
        assert_eq!(sprintf_one_float("%.1f", 73.25), "73.2");
    }
}
