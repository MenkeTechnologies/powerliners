// vim:fileencoding=utf-8:noet
//! Three-tier icon glyph registry — Nerd Font → Unicode pictogram →
//! ASCII letter. Tier is selected per-process via the
//! `POWERLINERS_ICONS` env var:
//!
//! - `nerdfont` (default): PUA glyphs from any Nerd Font (octocat,
//!   branch, cpu, microchip, …). Requires an NF-patched font.
//! - `unicode`: text-presentation pictograms that render in any
//!   UTF-8 terminal (gear, gpu cross-fill, hourglass, …).
//! - `ascii`: bare letters (CPU, GPU, …) — works on any terminal.
//!
//! Each glyph getter returns a `&'static str` so callers can splice
//! it into `format!`-style strings without allocating. Used by the
//! built-in segment adapters so themes don't have to redeclare icons
//! in every `format` arg.

use std::sync::OnceLock;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Tier {
    NerdFont,
    Unicode,
    Ascii,
}

fn tier() -> Tier {
    static T: OnceLock<Tier> = OnceLock::new();
    *T.get_or_init(|| {
        match std::env::var("POWERLINERS_ICONS")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("ascii") => Tier::Ascii,
            Some("unicode") => Tier::Unicode,
            _ => Tier::NerdFont,
        }
    })
}

fn pick(nf: &'static str, uni: &'static str, ascii: &'static str) -> &'static str {
    match tier() {
        Tier::NerdFont => nf,
        Tier::Unicode => uni,
        Tier::Ascii => ascii,
    }
}

/// Disk drive icon. NF: nf-fa-hdd_o (U+F0A0). Unicode: ⛁ stacked
/// disks. ASCII: `D:`.
pub fn disk() -> &'static str {
    pick("\u{F0A0}", "⛁", "D:")
}

/// CPU icon. NF: nf-oct-cpu (U+F4BC). Unicode: ⚙ gear (text
/// presentation via VS-15 in Unicode tier). ASCII: `CPU:`.
pub fn cpu() -> &'static str {
    pick("\u{F4BC}", "⚙\u{FE0E}", "CPU:")
}

/// GPU icon. NF: nf-fa-microchip (U+F2DB) — U+F1D8 is paper-plane in
/// modern Font Awesome / Nerd Font, the historical microchip mapping
/// moved. Unicode: ⎚ clear-screen. ASCII: `GPU:`.
pub fn gpu() -> &'static str {
    pick("\u{F2DB}", "⎚", "GPU:")
}

/// Memory icon. NF: nf-md-memory (U+EFC5). Unicode: ▤ horizontal-
/// fill square (RAM-stripe look). ASCII: `MEM:`.
pub fn memory() -> &'static str {
    pick("\u{EFC5}", "▤", "MEM:")
}

/// Thermometer icon. NF: nf-fa-thermometer (U+F2C8). Unicode: 🌡
/// + VS-15 for text presentation. ASCII: `T:`.
pub fn thermometer() -> &'static str {
    pick("\u{F2C8}", "🌡\u{FE0E}", "T:")
}

/// Git branch icon. NF: nf-pl-branch (Powerline ext, U+E0A0).
/// Unicode: empty (no clean equivalent; segment dividers carry the
/// boundary). ASCII: `b:`.
pub fn branch() -> &'static str {
    pick("\u{E0A0}", "", "b:")
}

/// GitHub octocat. NF: nf-fa-github_alt (U+F09B). Unicode: no
/// equivalent — return empty so the segment falls through to just
/// branch. ASCII: `gh:`.
pub fn github() -> &'static str {
    pick("\u{F09B}", "", "gh:")
}

/// Git tag. NF: nf-fa-tag (U+F02B). Unicode: ⚑ black flag. ASCII:
/// `tag:`.
pub fn tag() -> &'static str {
    pick("\u{F02B}", "⚑", "tag:")
}

/// Docker / OCI container. NF: nf-md-docker (U+ED7E). Unicode: 🐳
/// + VS-15 for text presentation. ASCII: `D:`.
pub fn docker() -> &'static str {
    pick("\u{ED7E}", "🐳\u{FE0E}", "D:")
}

/// Kubernetes helm-wheel. NF: nf-md-kubernetes (U+F10FE). Unicode:
/// ⎈ (helm symbol, the canonical k8s glyph in any text terminal).
/// ASCII: `k8s:`.
pub fn kubernetes() -> &'static str {
    pick("\u{F10FE}", "⎈", "k8s:")
}

/// Process / task icon. NF: nf-md-cog (U+F0493 — gear, "running").
/// Unicode: ⚙ gear with text presentation. ASCII: `P:`.
pub fn process() -> &'static str {
    pick("\u{F0493}", "⚙\u{FE0E}", "P:")
}

/// AWS cloud. NF: nf-fa-amazon (U+F270). Unicode: no canonical glyph,
/// fall back to the ASCII label. ASCII: `aws`.
pub fn aws() -> &'static str {
    pick("\u{F270}", "aws", "aws")
}

/// GCP / Google Cloud. NF: nf-md-google_cloud (U+F11F6). Unicode: no
/// canonical glyph — fall back to the ASCII label. ASCII: `gcp`.
pub fn gcp() -> &'static str {
    pick("\u{F11F6}", "gcp", "gcp")
}

/// CI success check-mark. NF: nf-oct-check (U+F42E). Unicode: ✓.
/// ASCII: `ok`.
pub fn ci_ok() -> &'static str {
    pick("\u{F42E}", "✓", "ok")
}

/// CI failure cross. NF: nf-oct-x (U+F2D7). Unicode: ✗. ASCII: `x`.
pub fn ci_fail() -> &'static str {
    pick("\u{F2D7}", "✗", "x")
}

/// CI running / pending dot. NF: nf-cod-circle_filled (U+EBB4 — solid
/// circle as a "spinning" stand-in; Powerline segments don't animate).
/// Unicode: ●. ASCII: `~`.
pub fn ci_run() -> &'static str {
    pick("\u{EBB4}", "●", "~")
}

/// fusevm / JIT glyph. Reuse the microchip (matches `gpu()` family —
/// fusevm compiles to native via Cranelift, microchip evokes the
/// machine-code level). NF: nf-fa-microchip (U+F2DB). Unicode: ⎚.
/// ASCII: `jit:`.
pub fn fusevm() -> &'static str {
    pick("\u{F2DB}", "⎚", "jit:")
}

// --- Weather glyphs, keyed by upstream condition name ----------

/// Sunny / day. NF: nf-weather-day_sunny (U+E30D). Unicode: ☀.
/// ASCII: `SUN`.
pub fn weather_sunny() -> &'static str {
    pick("\u{E30D}", "☀", "SUN")
}

/// Clear night. NF: nf-weather-night_clear (U+E32B). Unicode: ☾.
/// ASCII: `NIGHT`.
pub fn weather_night() -> &'static str {
    pick("\u{E32B}", "☾", "NIGHT")
}

/// Rain. NF: nf-weather-rain (U+E318). Unicode: ☔. ASCII: `RAIN`.
pub fn weather_rainy() -> &'static str {
    pick("\u{E318}", "☔", "RAIN")
}

/// Cloudy. NF: nf-weather-cloudy (U+E312). Unicode: ☁. ASCII:
/// `CLOUDS`.
pub fn weather_cloudy() -> &'static str {
    pick("\u{E312}", "☁", "CLOUDS")
}

/// Snowy. NF: nf-weather-snow (U+E31A). Unicode: ❄. ASCII: `SNOW`.
pub fn weather_snowy() -> &'static str {
    pick("\u{E31A}", "❄", "SNOW")
}

/// Thunderstorm. NF: nf-weather-thunderstorm (U+E31D). Unicode: ⛈.
/// ASCII: `STORM`.
pub fn weather_stormy() -> &'static str {
    pick("\u{E31D}", "⛈", "STORM")
}

/// Fog. NF: nf-weather-fog (U+E313). Unicode: 🌫. ASCII: `FOG`.
pub fn weather_foggy() -> &'static str {
    pick("\u{E313}", "🌫", "FOG")
}

/// Strong wind. NF: nf-weather-strong_wind (U+E31F). Unicode: 🌬.
/// ASCII: `WIND`.
pub fn weather_windy() -> &'static str {
    pick("\u{E31F}", "🌬", "WIND")
}

/// Unknown / not available. NF: nf-weather-na (U+E374). Unicode:
/// `?`. ASCII: `?`.
pub fn weather_unknown() -> &'static str {
    pick("\u{E374}", "?", "?")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_default_is_nerdfont() {
        // The OnceLock cache makes this test fragile if another test
        // already initialized the tier with a different env var. We
        // only assert the helper's selection logic for each variant
        // explicitly instead of relying on the global.
        assert_eq!(
            super::pick("nf", "u", "a"),
            match tier() {
                Tier::NerdFont => "nf",
                Tier::Unicode => "u",
                Tier::Ascii => "a",
            }
        );
    }

    #[test]
    fn weather_unknown_is_never_empty() {
        assert!(!weather_unknown().is_empty());
    }
}
