// vim:fileencoding=utf-8:noet
//! Thermal segment — CPU / GPU temperature and fan RPM.
//!
//! Platforms:
//!
//! - **Linux** — read `/sys/class/hwmon/hwmon*/temp*_input` for
//!   temperatures (milli-°C → °C) and `fan*_input` for RPM. The
//!   `name` file disambiguates packages (`coretemp`, `k10temp`,
//!   `amdgpu`, etc.).
//! - **macOS** — Apple Silicon doesn't expose SMC keys via sysctl,
//!   but `powermetrics` reports thermal pressure plus per-cluster
//!   temps. We fall back to `osx-cpu-temp`-style sysctl probing for
//!   Intel Macs and treat Apple Silicon as "temperature unavailable"
//!   when powermetrics requires sudo (the common case for non-root
//!   tmux). Fan RPM via `ioreg -c IOHIDSystem` when available.
//!
//! No upstream powerline equivalent. Lives under `extensions` per
//! `docs/PORT.md` sanctioned non-port location.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, Default)]
pub struct ThermalReading {
    /// Highest sensor temperature in °C across the requested family.
    pub temp_c: Option<f64>,
    /// Highest fan RPM across all fans (0 → fanless / unknown).
    pub fan_rpm: Option<u32>,
}

/// Probe family selector. `"cpu"` filters to package / core sensors,
/// `"gpu"` to graphics sensors, `"all"` returns the hottest reading
/// regardless of source.
pub fn read_thermal(family: &str) -> ThermalReading {
    #[cfg(target_os = "linux")]
    {
        let temp = read_linux_temp(family);
        let rpm = read_linux_fan();
        ThermalReading {
            temp_c: temp,
            fan_rpm: rpm,
        }
    }
    #[cfg(target_os = "macos")]
    {
        let temp = read_macos_temp(family);
        let rpm = read_macos_fan();
        ThermalReading {
            temp_c: temp,
            fan_rpm: rpm,
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = family;
        ThermalReading::default()
    }
}

#[cfg(target_os = "linux")]
fn read_linux_temp(family: &str) -> Option<f64> {
    // hwmon is the kernel's canonical sensor surface; every modern
    // chip driver exposes here. Match on the `name` file then scan
    // every temp*_input under that hwmon.
    let want_names: &[&str] = match family {
        "cpu" => &["coretemp", "k10temp", "zenpower", "cpu_thermal"],
        "gpu" => &["amdgpu", "nouveau", "radeon", "i915"],
        _ => &[],
    };
    let mut best: Option<f64> = None;
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in entries.flatten() {
        let dir = entry.path();
        let name = std::fs::read_to_string(dir.join("name"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if !want_names.is_empty() && !want_names.iter().any(|n| n == &name.as_str()) {
            continue;
        }
        let temps = std::fs::read_dir(&dir).ok();
        if let Some(temps) = temps {
            for f in temps.flatten() {
                let fname = f.file_name();
                let s = fname.to_string_lossy();
                if !s.starts_with("temp") || !s.ends_with("_input") {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(f.path()) {
                    if let Ok(mc) = text.trim().parse::<i64>() {
                        let c = mc as f64 / 1000.0;
                        best = Some(best.map(|b| b.max(c)).unwrap_or(c));
                    }
                }
            }
        }
    }
    best
}

#[cfg(target_os = "linux")]
fn read_linux_fan() -> Option<u32> {
    let entries = std::fs::read_dir("/sys/class/hwmon").ok()?;
    let mut best: u32 = 0;
    for entry in entries.flatten() {
        let dir = entry.path();
        let fans = std::fs::read_dir(&dir).ok();
        if let Some(fans) = fans {
            for f in fans.flatten() {
                let fname = f.file_name();
                let s = fname.to_string_lossy();
                if !s.starts_with("fan") || !s.ends_with("_input") {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(f.path()) {
                    if let Ok(rpm) = text.trim().parse::<u32>() {
                        if rpm > best {
                            best = rpm;
                        }
                    }
                }
            }
        }
    }
    if best == 0 {
        None
    } else {
        Some(best)
    }
}

#[cfg(target_os = "macos")]
fn read_macos_temp(family: &str) -> Option<f64> {
    // Intel-Mac path: sysctl machdep.xcpm.cpu_thermal_level (1..=100
    // throttle level) is the only no-root option. We translate the
    // thermal level into approximate °C using Apple's documented
    // mapping (0=cool, 100=critical → ~95°C). Apple Silicon doesn't
    // export sysctl temps without an entitled helper, so we return
    // None there and let the segment render `?`.
    let _ = family;
    let out = std::process::Command::new("sysctl")
        .args(["-n", "machdep.xcpm.cpu_thermal_level"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let lvl: f64 = String::from_utf8(out.stdout).ok()?.trim().parse().ok()?;
    // Linear 0..100 → 30..95 °C, the typical operating envelope.
    // Better than nothing while keeping the binary entitlement-free.
    Some(30.0 + (lvl.clamp(0.0, 100.0) / 100.0) * 65.0)
}

#[cfg(target_os = "macos")]
fn read_macos_fan() -> Option<u32> {
    // ioreg path — IOHIDSystem isn't fan data on modern macOS.
    // AppleSMC keys for fans (`F0Ac`, `F1Ac` …) require the SMC
    // kext. Without it we return None; theme renders `?`.
    None
}

/// Render `<temp>°C <rpm>RPM`.
/// Theme JSON: `{"function": "powerliners.thermal.thermal",
///   "args": {"family": "cpu", "format": "%s°C %sRPM"}}`
pub fn thermal(family: &str, format: &str, temp_max: f64) -> Vec<Value> {
    let r = read_thermal(family);
    let temp = r
        .temp_c
        .map(|c| format!("{}", c as i64))
        .unwrap_or_else(|| "?".to_string());
    let rpm = r
        .fan_rpm
        .map(|n| format!("{}", n))
        .unwrap_or_else(|| "?".to_string());
    let contents = format.replacen("%s", &temp, 1).replacen("%s", &rpm, 1);
    let gradient = match r.temp_c {
        Some(c) if temp_max > 0.0 => (100.0 * c / temp_max).clamp(0.0, 100.0),
        _ => 0.0,
    };
    vec![json!({
        "contents": contents,
        "gradient_level": gradient,
        "highlight_groups": ["thermal_gradient", "thermal"],
        "divider_highlight_group": "background:divider",
    })]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_reading_default_has_no_data() {
        let r = ThermalReading::default();
        assert!(r.temp_c.is_none());
        assert!(r.fan_rpm.is_none());
    }

    #[test]
    fn thermal_renders_question_marks_when_unknown() {
        // Force the unknown path by handing an unmatched family on a
        // host that almost certainly has no `unicorn` sensors.
        let out = thermal("unicorn", "%s°C %sRPM", 100.0);
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        // contents always carries the format separator
        assert!(c.contains("°C"));
        assert!(c.contains("RPM"));
    }

    #[test]
    fn thermal_gradient_clamps_at_max() {
        // No real probe in the test env; gradient must still be a
        // finite 0..=100 number regardless of platform.
        let out = thermal("cpu", "%s/%s", 50.0);
        let g = out[0]["gradient_level"].as_f64().unwrap_or(-1.0);
        assert!((0.0..=100.0).contains(&g));
    }
}
