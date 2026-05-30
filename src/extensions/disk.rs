// vim:fileencoding=utf-8:noet
//! Disk segments — capacity per mountpoint and live IO rates.
//!
//! Two public entrypoints map cleanly onto two distinct theme slots:
//!
//! - [`disk_usage`] — `USED/TOTAL` (or `NN%`) for a single mount.
//!   Backed by `df -k -P` portably across macOS / Linux / BSD.
//! - [`disk_io`] — `R MB/s W MB/s` from a 500 ms delta sample of
//!   `iostat -dKw 1 -c 2` on macOS / `/proc/diskstats` on Linux.
//!
//! No upstream powerline equivalent (`powerline.segments.common`
//! ships no disk segment at all). Lives under `extensions` per
//! `docs/PORT.md`'s sanctioned non-port location.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, Default)]
pub struct DiskUsage {
    pub total: u64,
    pub used: u64,
    pub free: u64,
}

impl DiskUsage {
    pub fn percent(&self) -> f64 {
        // df's Capacity column = used / (used + avail). On APFS the
        // raw `total` from statvfs includes reserved space that isn't
        // available to userland, so `used/total` underreports vs what
        // `df` and Finder show. Match df's denominator instead so the
        // segment agrees with the tool the user actually runs.
        let denom = self.used + self.free;
        if denom == 0 {
            0.0
        } else {
            100.0 * self.used as f64 / denom as f64
        }
    }
}

/// Probe one mountpoint via `df -k -P <mount>`. Returns bytes (not KB).
/// Falls back to `None` when df fails or the mount is missing.
pub fn read_disk_usage(mount: &str) -> Option<DiskUsage> {
    // APFS quirk on macOS: `/` is the read-only sealed system volume
    // (~12 GiB of OS). User data lives on `/System/Volumes/Data`.
    // Probing `/` reports a misleadingly empty disk. Redirect when the
    // caller asked for the root and the data volume is mounted —
    // matches what Finder's "About This Mac" → Storage reports.
    #[cfg(target_os = "macos")]
    let mount = if mount == "/" && std::path::Path::new("/System/Volumes/Data").exists() {
        "/System/Volumes/Data"
    } else {
        mount
    };
    let out = std::process::Command::new("df")
        .args(["-k", "-P", mount])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    // POSIX df -P layout:
    //   Filesystem  1024-blocks  Used  Available  Capacity  Mounted on
    let data_line = text.lines().nth(1)?;
    let cols: Vec<&str> = data_line.split_whitespace().collect();
    let total_kb: u64 = cols.get(1)?.parse().ok()?;
    let used_kb: u64 = cols.get(2)?.parse().ok()?;
    let free_kb: u64 = cols.get(3)?.parse().ok()?;
    let kib = 1024u64;
    Some(DiskUsage {
        total: total_kb * kib,
        used: used_kb * kib,
        free: free_kb * kib,
    })
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DiskIo {
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
}

/// Sample `/proc/diskstats` (Linux) or `iostat` (macOS) twice with a
/// 500 ms gap and return the per-second rate. `device` is matched
/// against the kernel device name (`disk0`, `nvme0n1`, etc.); pass
/// `"auto"` to aggregate every block device.
pub fn read_disk_io(device: &str) -> Option<DiskIo> {
    #[cfg(target_os = "linux")]
    {
        read_disk_io_linux(device)
    }
    #[cfg(target_os = "macos")]
    {
        read_disk_io_macos(device)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = device;
        None
    }
}

#[cfg(target_os = "linux")]
fn read_disk_io_linux(device: &str) -> Option<DiskIo> {
    // /proc/diskstats columns (kernel ≥4.18): major minor name
    //   reads_completed reads_merged sectors_read time_reading
    //   writes_completed writes_merged sectors_written time_writing ...
    // Sector size is conventionally 512 B.
    let snap = || -> Option<(f64, u64, u64)> {
        let text = std::fs::read_to_string("/proc/diskstats").ok()?;
        let mut r = 0u64;
        let mut w = 0u64;
        for line in text.lines() {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 11 {
                continue;
            }
            let name = cols[2];
            if device != "auto" && name != device {
                continue;
            }
            let sectors_read: u64 = cols[5].parse().unwrap_or(0);
            let sectors_written: u64 = cols[9].parse().unwrap_or(0);
            r += sectors_read * 512;
            w += sectors_written * 512;
        }
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs_f64();
        Some((t, r, w))
    };
    let (t1, r1, w1) = snap()?;
    std::thread::sleep(std::time::Duration::from_millis(500));
    let (t2, r2, w2) = snap()?;
    let dt = (t2 - t1).max(0.001);
    Some(DiskIo {
        read_bytes_per_sec: r2.saturating_sub(r1) as f64 / dt,
        write_bytes_per_sec: w2.saturating_sub(w1) as f64 / dt,
    })
}

#[cfg(target_os = "macos")]
fn read_disk_io_macos(_device: &str) -> Option<DiskIo> {
    // `iostat -d` on macOS only reports combined MB/s — no R/W split
    // without sudo. `ioreg -c IOBlockStorageDriver -r` exposes per-
    // device cumulative `Bytes (Read)` / `Bytes (Write)` counters
    // entitlement-free, but the kernel updates them in batches — a
    // 500ms snap-sleep-snap window during a 2GB/s write commonly
    // reports delta=0. Cache the previous snapshot across calls and
    // compute the rate vs whatever time the daemon's render cadence
    // actually gives us; the granularity then matches the user's
    // tmux refresh, which is the true sample window anyway.
    use std::sync::Mutex;
    use std::time::Instant;
    static LAST: std::sync::OnceLock<Mutex<Option<(Instant, u64, u64)>>> =
        std::sync::OnceLock::new();
    let cell = LAST.get_or_init(|| Mutex::new(None));
    let (r, w) = read_ioreg_block_storage_totals()?;
    let now = Instant::now();
    let mut guard = cell.lock().ok()?;
    let rate = match *guard {
        Some((t0, r0, w0)) => {
            let dt = now.duration_since(t0).as_secs_f64();
            if dt > 0.0 {
                Some(DiskIo {
                    read_bytes_per_sec: r.saturating_sub(r0) as f64 / dt,
                    write_bytes_per_sec: w.saturating_sub(w0) as f64 / dt,
                })
            } else {
                Some(DiskIo::default())
            }
        }
        None => Some(DiskIo::default()),
    };
    *guard = Some((now, r, w));
    rate
}

#[cfg(target_os = "macos")]
fn read_ioreg_block_storage_totals() -> Option<(u64, u64)> {
    let out = std::process::Command::new("ioreg")
        .args(["-c", "IOBlockStorageDriver", "-r"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut r = 0u64;
    let mut w = 0u64;
    for line in text.lines() {
        if !line.contains("\"Statistics\"") {
            continue;
        }
        if let Some(v) = extract_ioreg_kv(line, "Bytes (Read)") {
            r += v;
        }
        if let Some(v) = extract_ioreg_kv(line, "Bytes (Write)") {
            w += v;
        }
    }
    Some((r, w))
}

#[cfg(target_os = "macos")]
fn extract_ioreg_kv(line: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{}\"=", key);
    let idx = line.find(&needle)?;
    let rest = &line[idx + needle.len()..];
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

/// Format `n` bytes the same way mem_usage / gpu do.
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

fn fmt_rate(bps: f64, short: bool) -> String {
    let s = fmt_bytes(bps as u64, short);
    format!("{}/s", s)
}

/// Render `USED/TOTAL` for a mountpoint.
/// Theme JSON: `{"function": "powerliners.disk.disk_usage",
///   "args": {"mount": "/", "format": "%s/%s"}}`
pub fn disk_usage(mount: &str, format: &str, short: bool) -> Vec<Value> {
    let stats = read_disk_usage(mount).unwrap_or_default();
    let used = fmt_bytes(stats.used, short);
    let total = fmt_bytes(stats.total, short);
    let contents = format.replacen("%s", &used, 1).replacen("%s", &total, 1);
    vec![json!({
        "contents": contents,
        "gradient_level": stats.percent(),
        "highlight_groups": [
            "disk_usage_gradient",
            "disk_usage",
            "mem_usage_gradient",
            "mem_usage",
        ],
        "divider_highlight_group": "background:divider",
    })]
}

/// Render `NN%` for a mountpoint.
pub fn disk_usage_percent(mount: &str, format: &str) -> Vec<Value> {
    let stats = read_disk_usage(mount).unwrap_or_default();
    let pct = stats.percent();
    let contents = if format.contains("{0:d}") {
        format.replace("{0:d}", &format!("{}", pct as i64))
    } else if format.contains("%d") {
        format.replace("%d", &format!("{}", pct as i64))
    } else {
        format!("{}%", pct as i64)
    };
    vec![json!({
        "contents": contents,
        "gradient_level": pct,
        "highlight_groups": [
            "disk_usage_gradient",
            "disk_usage",
            "mem_usage_gradient",
            "mem_usage",
        ],
        "divider_highlight_group": "background:divider",
    })]
}

/// Render `R <rate> W <rate>` for a single device (or `auto`).
/// `recv_max` / `sent_max` follow the network_load convention for
/// gradient_level normalization.
pub fn disk_io(
    device: &str,
    format: &str,
    short: bool,
    recv_max: f64,
    sent_max: f64,
) -> Vec<Value> {
    let io = read_disk_io(device).unwrap_or_default();
    let r = fmt_rate(io.read_bytes_per_sec, short);
    let w = fmt_rate(io.write_bytes_per_sec, short);
    let contents = format.replacen("%s", &r, 1).replacen("%s", &w, 1);
    let max_total = (recv_max + sent_max).max(1.0);
    let pct = 100.0 * (io.read_bytes_per_sec + io.write_bytes_per_sec) / max_total;
    let gradient = pct.clamp(0.0, 100.0);
    vec![json!({
        "contents": contents,
        "gradient_level": gradient,
        "highlight_groups": [
            "disk_io_gradient",
            "disk_io",
            "network_load_gradient",
            "network_load",
        ],
        "divider_highlight_group": "background:divider",
    })]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_handles_zero_total() {
        assert_eq!(DiskUsage::default().percent(), 0.0);
    }

    #[test]
    fn percent_basic() {
        let u = DiskUsage {
            total: 100,
            used: 25,
            free: 75,
        };
        assert_eq!(u.percent(), 25.0);
    }

    #[test]
    fn fmt_bytes_basic() {
        assert_eq!(fmt_bytes(0, false), "0B");
        assert_eq!(fmt_bytes(1024, false), "1KiB");
        assert_eq!(fmt_bytes(1024 * 1024, true), "1M");
    }

    #[test]
    fn fmt_rate_appends_per_second() {
        assert_eq!(fmt_rate(1024.0, true), "1K/s");
    }

    #[test]
    fn disk_usage_produces_one_chunk_with_format_subs() {
        // df is portable so this exercises the real probe + format pipe;
        // / always exists on darwin/linux. We assert shape, not value.
        let out = disk_usage("/", "%s/%s", true);
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        assert!(c.contains('/'));
        assert!(!c.contains("%s"), "format substitution failed: {c}");
    }

    #[test]
    fn disk_usage_percent_d_form() {
        let out = disk_usage_percent("/", "{0:d}%");
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        assert!(c.ends_with('%'), "expected NN%, got {c}");
    }
}
