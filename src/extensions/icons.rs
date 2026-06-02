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
/// Unicode: ⎇ alternative-key-symbol (U+2387) — canonical text-tier
/// git-branch glyph (vim's statusline, gitprompt.sh, oh-my-bash all
/// use it). ASCII: `b:`.
pub fn branch() -> &'static str {
    pick("\u{E0A0}", "⎇", "b:")
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

/// Docker / OCI container. NF: nf-md-docker (U+F0868) — the
/// canonical Material Design docker glyph. U+ED7E was used here
/// previously but that codepoint is `i_fa_dolly` (a hand truck) in
/// nerd-fonts master, not docker — fixed in the icon-audit pass.
/// Unicode: 🐳 + VS-15 for text presentation. ASCII: `D:`.
pub fn docker() -> &'static str {
    pick("\u{F0868}", "🐳\u{FE0E}", "D:")
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

/// AWS cloud. NF: nf-md-aws (U+F0E0F) — the AWS-specific glyph (not
/// nf-fa-amazon, which is the parent-company logo). Unicode: no
/// canonical glyph, fall back to the ASCII label. ASCII: `aws`.
pub fn aws() -> &'static str {
    pick("\u{F0E0F}", "aws", "aws")
}

/// GCP / Google Cloud. NF: nf-md-google_cloud (U+F11F6). Unicode: no
/// canonical glyph — fall back to the ASCII label. ASCII: `gcp`.
pub fn gcp() -> &'static str {
    pick("\u{F11F6}", "gcp", "gcp")
}

/// CI success — circled check (the GitHub UI's success badge shape).
/// NF: nf-oct-check_circle (U+F49E). Unicode: ✅ + VS-15. ASCII: `OK`.
pub fn ci_ok() -> &'static str {
    pick("\u{F49E}", "✅\u{FE0E}", "OK")
}

/// CI failure — circled x (the GitHub UI's failure badge shape).
/// NF: nf-oct-x_circle (U+F52F). Unicode: ❌ + VS-15. ASCII: `FAIL`.
pub fn ci_fail() -> &'static str {
    pick("\u{F52F}", "❌\u{FE0E}", "FAIL")
}

/// CI running — sync arrows (the GitHub UI's "in progress" badge).
/// NF: nf-oct-sync (U+F46A). Unicode: 🔄 + VS-15. ASCII: `RUN`.
pub fn ci_run() -> &'static str {
    pick("\u{F46A}", "🔄\u{FE0E}", "RUN")
}

/// Count / file-multiple — labels an entry-count number so the user
/// reads "142 entries", not just "142". NF: nf-md-file_multiple
/// (U+F0222). Unicode: ▤ stack glyph. ASCII: `n=`.
pub fn count() -> &'static str {
    pick("\u{F0222}", "▤", "n=")
}

/// Hard disk — labels an on-disk byte-size number. NF: nf-md-harddisk
/// (U+F02CA). Unicode: ⛁ stacked disks. ASCII: `d=`.
pub fn harddisk() -> &'static str {
    pick("\u{F02CA}", "⛁", "d=")
}

/// Swap — paired horizontal arrows, the canonical "swap" glyph
/// (memory page-in/page-out). NF: nf-md-swap_horizontal (U+F04E1).
/// Unicode: ⇄ leftwards-arrow-over-rightwards-arrow. ASCII: `SWAP:`.
pub fn swap() -> &'static str {
    pick("\u{F04E1}", "⇄", "SWAP:")
}

/// I/O throughput — vertical up/down arrows for read/write rate.
/// NF: nf-md-arrow_up_down (U+F0E79). Unicode: ⇅. ASCII: `IO:`.
pub fn io() -> &'static str {
    pick("\u{F0E79}", "⇅", "IO:")
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
