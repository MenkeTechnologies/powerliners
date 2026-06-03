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
    let _ = family;
    // Intel-Mac path: sysctl machdep.xcpm.cpu_thermal_level (1..=100
    // throttle level) is the only no-root option. 0=cool → 100=critical
    // maps linearly to 30..95 °C.
    if let Ok(out) = std::process::Command::new("sysctl")
        .args(["-n", "machdep.xcpm.cpu_thermal_level"])
        .output()
    {
        if out.status.success() {
            if let Ok(s) = String::from_utf8(out.stdout) {
                if let Ok(lvl) = s.trim().parse::<f64>() {
                    return Some(30.0 + (lvl.clamp(0.0, 100.0) / 100.0) * 65.0);
                }
            }
        }
    }
    // Apple Silicon path: AppleSmartBattery exposes a chip-derived
    // `VirtualTemperature` and battery-surface `Temperature` (both in
    // centi-°C) via ioreg, entitlement-free. VirtualTemperature tracks
    // CPU load and is the closer proxy for the user-visible "is it
    // hot?" question that this segment answers. Fall back to
    // Temperature when VirtualTemperature is absent.
    let out = std::process::Command::new("ioreg")
        .args(["-r", "-n", "AppleSmartBattery"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    parse_ioreg_centi_celsius(&text, "VirtualTemperature")
        .or_else(|| parse_ioreg_centi_celsius(&text, "Temperature"))
}

/// Extract a centi-°C value for `key` from `ioreg`'s text dump.
///
/// `ioreg -r -n AppleSmartBattery` emits lines like
/// `    "VirtualTemperature" = 3128` (centi-°C, here 31.28 °C).
/// Pure function — separated from `read_macos_temp` so the parse can
/// be exercised without spawning `ioreg`. Returns `None` when `key`
/// isn't present, when the value isn't a parseable integer, or when
/// the leading byte is a sign-not-followed-by-digits (rare but
/// possible with corrupt ioreg output).
pub fn parse_ioreg_centi_celsius(text: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{}\" = ", key);
    let idx = text.find(&needle)?;
    let rest = &text[idx + needle.len()..];
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    let n: i64 = num.parse().ok()?;
    Some(n as f64 / 100.0)
}

#[cfg(target_os = "macos")]
fn read_macos_fan() -> Option<u32> {
    // ioreg path — IOHIDSystem isn't fan data on modern macOS.
    // AppleSMC keys for fans (`F0Ac`, `F1Ac` …) require the SMC
    // kext. Without it we return None; theme renders `?`.
    None
}

/// Pure formatter for the thermal segment's `contents` field.
///
/// `format` is a `printf`-style template with up to two `%s` slots —
/// the first holds the temperature (`?` when unavailable), the second
/// holds fan RPM. When `fan_rpm` is `None`, everything from the second
/// `%s` onward is dropped (rather than rendering a literal `?RPM`),
/// and trailing whitespace before the second slot is trimmed so
/// `"%s°C %sRPM"` collapses cleanly to `"36°C"` instead of `"36°C "`.
pub fn format_thermal(temp_c: Option<f64>, fan_rpm: Option<u32>, format: &str) -> String {
    let temp = temp_c
        .map(|c| format!("{}", c as i64))
        .unwrap_or_else(|| "?".to_string());
    if let Some(rpm_val) = fan_rpm {
        format
            .replacen("%s", &temp, 1)
            .replacen("%s", &rpm_val.to_string(), 1)
    } else {
        let first_end = format.find("%s").map(|i| i + 2).unwrap_or(format.len());
        let head = match format[first_end..].find("%s") {
            Some(rel) => format[..first_end + rel].trim_end_matches([' ', '\t']),
            None => format,
        };
        head.replacen("%s", &temp, 1)
    }
}

/// Pure helper: clamp `100 * temp / temp_max` into the `[0, 100]`
/// gradient band used by powerline's `gradient_level`. Returns 0 when
/// the temperature is unavailable or `temp_max <= 0` (defensive: the
/// theme JSON could specify a non-positive ceiling).
pub fn thermal_gradient(temp_c: Option<f64>, temp_max: f64) -> f64 {
    match temp_c {
        Some(c) if temp_max > 0.0 => (100.0 * c / temp_max).clamp(0.0, 100.0),
        _ => 0.0,
    }
}

/// Render `<temp>°C <rpm>RPM`.
/// Theme JSON: `{"function": "powerliners.thermal.thermal",
///   "args": {"family": "cpu", "format": "%s°C %sRPM"}}`
pub fn thermal(family: &str, format: &str, temp_max: f64) -> Vec<Value> {
    let r = read_thermal(family);
    let contents = format_thermal(r.temp_c, r.fan_rpm, format);
    let gradient = thermal_gradient(r.temp_c, temp_max);
    vec![json!({
        "contents": contents,
        "gradient_level": gradient,
        "highlight_groups": [
            "thermal_gradient",
            "thermal",
            "system_load_gradient",
            "system_load",
        ],
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
    fn thermal_drops_rpm_tail_when_fan_unavailable() {
        // No CI host exposes fan RPM via the entitlement-free probes
        // (macOS SMC kext gated, Linux test sandboxes lack hwmon fan
        // files). The "%sRPM" tail must collapse cleanly instead of
        // leaving a literal "?RPM" suffix dangling.
        let out = thermal("cpu", "%s°C %sRPM", 100.0);
        assert_eq!(out.len(), 1);
        let c = out[0]["contents"].as_str().unwrap_or("");
        assert!(c.contains("°C"), "lost the temp half: {:?}", c);
        assert!(!c.contains("RPM"), "RPM tail not stripped: {:?}", c);
        assert!(!c.ends_with(' '), "trailing separator left behind: {:?}", c);
    }

    #[test]
    fn thermal_gradient_clamps_at_max() {
        // No real probe in the test env; gradient must still be a
        // finite 0..=100 number regardless of platform.
        let out = thermal("cpu", "%s/%s", 50.0);
        let g = out[0]["gradient_level"].as_f64().unwrap_or(-1.0);
        assert!((0.0..=100.0).contains(&g));
    }

    // ---- format_thermal: pure formatter ----

    #[test]
    fn format_thermal_both_tokens_render_when_fan_available() {
        let s = format_thermal(Some(48.0), Some(2400), "%s°C %sRPM");
        assert_eq!(s, "48°C 2400RPM");
    }

    #[test]
    fn format_thermal_temp_truncates_to_integer() {
        // The `c as i64` cast in format_thermal truncates toward zero —
        // 48.9 °C renders as `48`, not `49`. Pinned so a future
        // rounding tweak is explicit.
        let s = format_thermal(Some(48.9), Some(1000), "%s/%s");
        assert_eq!(s, "48/1000");
    }

    #[test]
    fn format_thermal_missing_temp_renders_question_mark() {
        let s = format_thermal(None, Some(1500), "%s/%s");
        assert_eq!(s, "?/1500");
    }

    #[test]
    fn format_thermal_missing_fan_drops_rpm_tail() {
        // The headline bug this codepath exists to prevent: rendering
        // `?RPM` when fan probing is unavailable. Tail must be dropped
        // and the trailing separator trimmed.
        let s = format_thermal(Some(36.0), None, "%s°C %sRPM");
        assert_eq!(s, "36°C");
    }

    #[test]
    fn format_thermal_missing_fan_tab_separator_trimmed() {
        // Trims `\t` as well as `' '` — the trim_end_matches closure
        // includes both. Pin the tab variant so a future cleanup
        // doesn't quietly drop it.
        let s = format_thermal(Some(36.0), None, "%s°C\t\t%sRPM");
        assert_eq!(s, "36°C");
    }

    #[test]
    fn format_thermal_single_slot_format() {
        // Theme that doesn't care about RPM — only one `%s`. Both
        // branches (fan present / absent) should produce the same
        // output because the second-slot logic is short-circuited.
        let s_with = format_thermal(Some(50.0), Some(1200), "%s°C");
        let s_without = format_thermal(Some(50.0), None, "%s°C");
        assert_eq!(s_with, "50°C");
        assert_eq!(s_without, "50°C");
    }

    #[test]
    fn format_thermal_no_slots_passes_format_through() {
        // No `%s` at all — string passes through unchanged.
        let s = format_thermal(Some(50.0), Some(1200), "literal text");
        assert_eq!(s, "literal text");
    }

    // ---- thermal_gradient: ratio math + clamps ----

    #[test]
    fn thermal_gradient_ratio_is_percentage_of_max() {
        // 50/100 → 50%, exact float equality is fine here (1 / 2).
        assert_eq!(thermal_gradient(Some(50.0), 100.0), 50.0);
    }

    #[test]
    fn thermal_gradient_clamps_overshoot_to_100() {
        // Reading above temp_max → clamped to 100, never panicking.
        assert_eq!(thermal_gradient(Some(150.0), 100.0), 100.0);
    }

    #[test]
    fn thermal_gradient_returns_zero_when_temp_absent() {
        assert_eq!(thermal_gradient(None, 100.0), 0.0);
    }

    #[test]
    fn thermal_gradient_returns_zero_when_max_nonpositive() {
        // Defensive — theme JSON could pass `temp_max: 0` or a
        // negative value; we must not divide by zero or report NaN.
        assert_eq!(thermal_gradient(Some(50.0), 0.0), 0.0);
        assert_eq!(thermal_gradient(Some(50.0), -10.0), 0.0);
    }

    // ---- parse_ioreg_centi_celsius: AppleSmartBattery dump parser ----

    #[test]
    fn parse_ioreg_extracts_named_centi_celsius_value() {
        // Realistic shape of an ioreg AppleSmartBattery line: 4-space
        // indent, quoted key, ` = `, integer in centi-°C (3128 →
        // 31.28 °C). Pinned with floating-point tolerance because the
        // n/100 division isn't exactly representable.
        let text = r#"    | |   "VirtualTemperature" = 3128
    | |   "Temperature" = 3010
"#;
        let got = parse_ioreg_centi_celsius(text, "VirtualTemperature").unwrap();
        assert!((got - 31.28).abs() < 1e-9, "got {got}");
    }

    #[test]
    fn parse_ioreg_falls_back_via_two_calls() {
        // Mirrors the production call: try VirtualTemperature first,
        // then Temperature. When the first key is absent we still
        // resolve via the second.
        let text = "    \"Temperature\" = 2500\n";
        assert!(parse_ioreg_centi_celsius(text, "VirtualTemperature").is_none());
        let got = parse_ioreg_centi_celsius(text, "Temperature").unwrap();
        assert!((got - 25.0).abs() < 1e-9, "got {got}");
    }

    #[test]
    fn parse_ioreg_missing_key_returns_none() {
        let text = "    \"NotTheKey\" = 1234\n";
        assert!(parse_ioreg_centi_celsius(text, "VirtualTemperature").is_none());
    }

    #[test]
    fn parse_ioreg_handles_negative_value() {
        // ioreg can emit signed values for chilled / cold-soak readings;
        // the `*c == '-'` guard in take_while must let the sign through.
        let text = "    \"Temperature\" = -250\n";
        let got = parse_ioreg_centi_celsius(text, "Temperature").unwrap();
        assert!((got - (-2.5)).abs() < 1e-9, "got {got}");
    }

    #[test]
    fn parse_ioreg_value_without_digits_returns_none() {
        // Defensive: corrupt key with no parseable digit run → None,
        // not a panic. Reproduces the take_while-then-parse path
        // returning empty.
        let text = "    \"Temperature\" = xx99\n";
        assert!(parse_ioreg_centi_celsius(text, "Temperature").is_none());
    }
}
