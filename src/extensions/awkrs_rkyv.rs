// vim:fileencoding=utf-8:noet
//! awkrs rkyv-cache segment — reports the on-disk allocation of the
//! authoritative compiled-bytecode archive at
//! `<root>/scripts.rkyv`. awkrs (Rust awk reimplementation) writes
//! its bytecode store as a single rkyv archive; this segment shows
//! "how warm is the awkrs cache" at a glance.
//!
//! Pure filesystem probe — single `stat` of the archive file.
//!
//! Default resolution:
//! 1. `$AWKRS_RKYV_CACHE`           — explicit override
//! 2. `$AWKRS_HOME/scripts.rkyv`
//! 3. `$XDG_DATA_HOME/awkrs/scripts.rkyv`
//! 4. `~/.awkrs/scripts.rkyv`
//!
//! Returns `None` when the file doesn't exist.
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.awkrs.rkyv_cache",
//!   "args": {
//!     "path": "~/.awkrs/scripts.rkyv",
//!     "format": "{icon} {size}",
//!     "show_when_empty": false
//!   }
//! }
//! ```
//!
//! Format tokens (identical surface to zshrs/stryke segments):
//! - `{icon}`          — awkrs glyph
//! - `{size}`          — on-disk allocation, human-formatted (matches `du -sh`)
//! - `{bytes}`         — on-disk allocation, raw integer
//! - `{logical_size}`  — file content size, human-formatted
//! - `{logical_bytes}` — file content size, raw integer

use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RkyvStats {
    pub exists: bool,
    pub bytes: u64,
    pub disk_bytes: u64,
}

#[cfg(unix)]
fn block_bytes(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn block_bytes(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}

fn default_path() -> PathBuf {
    default_path_with(|k| std::env::var_os(k), |p| p.exists())
}

/// Pure-functional core of `default_path()` — takes env-var lookup and
/// path-existence predicates as parameters so the 4-level resolution
/// chain can be unit-tested without mutating the process env.
fn default_path_with(
    get_env: impl Fn(&str) -> Option<OsString>,
    path_exists: impl Fn(&std::path::Path) -> bool,
) -> PathBuf {
    if let Some(p) = get_env("AWKRS_RKYV_CACHE") {
        return PathBuf::from(p);
    }
    if let Some(home) = get_env("AWKRS_HOME") {
        return PathBuf::from(home).join("scripts.rkyv");
    }
    if let Some(xdg) = get_env("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg).join("awkrs").join("scripts.rkyv");
        if path_exists(&p) {
            return p;
        }
    }
    let home = get_env("HOME").unwrap_or_default();
    PathBuf::from(home).join(".awkrs").join("scripts.rkyv")
}

fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(p)
}

pub fn stat_file(p: &std::path::Path) -> Option<RkyvStats> {
    let meta = fs::metadata(p).ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(RkyvStats {
        exists: true,
        bytes: meta.len(),
        disk_bytes: block_bytes(&meta),
    })
}

pub fn human_bytes(n: u64) -> String {
    const SUFFIXES: &[&str] = &["B", "K", "M", "G", "T", "P"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i + 1 < SUFFIXES.len() {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n}{}", SUFFIXES[0])
    } else if v < 10.0 {
        format!("{:.2}{}", v, SUFFIXES[i])
    } else if v < 100.0 {
        format!("{:.1}{}", v, SUFFIXES[i])
    } else {
        format!("{:.0}{}", v, SUFFIXES[i])
    }
}

pub fn rkyv_cache(path: &str, format: &str, show_when_empty: bool) -> Option<Vec<Value>> {
    let target = if path.is_empty() {
        default_path()
    } else {
        expand_tilde(path)
    };
    let stats = match stat_file(&target) {
        Some(s) => s,
        None if show_when_empty => RkyvStats::default(),
        None => return None,
    };
    let contents = format
        .replace("{bytes}", &stats.disk_bytes.to_string())
        .replace("{size}", &human_bytes(stats.disk_bytes))
        .replace("{logical_bytes}", &stats.bytes.to_string())
        .replace("{logical_size}", &human_bytes(stats.bytes));
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": ["awkrs_rkyv_cache", "awkrs", "information:regular"],
        "divider_highlight_group": "background:divider",
    })])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    fn tmpfile(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("powerliners-awkrs-{name}.rkyv"));
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn stat_missing_returns_none() {
        assert!(stat_file(Path::new("/nonexistent/awkrs-xyz.rkyv")).is_none());
    }

    #[test]
    fn stat_file_returns_size() {
        let p = tmpfile("stat", &[0u8; 4096]);
        let s = stat_file(&p).unwrap();
        assert!(s.exists);
        assert_eq!(s.bytes, 4096);
        assert!(s.disk_bytes >= s.bytes);
    }

    #[test]
    fn rkyv_cache_missing_returns_none_when_hidden() {
        let r = rkyv_cache("/nonexistent/awkrs-zzz.rkyv", "{size}", false);
        assert!(r.is_none());
    }

    #[test]
    fn rkyv_cache_missing_renders_zero_when_show_when_empty() {
        let r = rkyv_cache("/nonexistent/awkrs-zzz.rkyv", "{size}", true).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "0B");
    }

    #[test]
    fn rkyv_cache_format_tokens_render() {
        let p = tmpfile("format", &[0u8; 2048]);
        let r = rkyv_cache(p.to_str().unwrap(), "{logical_size}/{logical_bytes}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "2.00K/2048");
    }

    #[test]
    fn rkyv_cache_highlight_groups_have_neutral_fallback() {
        let p = tmpfile("hl", &[0u8; 1]);
        let r = rkyv_cache(p.to_str().unwrap(), "{size}", false).unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(
            groups.last().unwrap().as_str().unwrap(),
            "information:regular"
        );
    }

    // ---- default_path resolution chain (pure-functional via closures) ----

    fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let owned: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), OsString::from(*v)))
            .collect();
        move |k: &str| owned.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn default_path_awkrs_rkyv_cache_wins_over_everything() {
        let g = env_map(&[
            ("AWKRS_RKYV_CACHE", "/explicit/override.rkyv"),
            ("AWKRS_HOME", "/awkrs/home"),
            ("XDG_DATA_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(
            default_path_with(g, |_| true),
            PathBuf::from("/explicit/override.rkyv")
        );
    }

    #[test]
    fn default_path_awkrs_home_wins_over_xdg_and_home() {
        let g = env_map(&[
            ("AWKRS_HOME", "/awkrs/home"),
            ("XDG_DATA_HOME", "/xdg"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(
            default_path_with(g, |_| true),
            PathBuf::from("/awkrs/home/scripts.rkyv")
        );
    }

    #[test]
    fn default_path_uses_xdg_when_archive_exists() {
        let g = env_map(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(
            default_path_with(g, |_| true),
            PathBuf::from("/xdg/awkrs/scripts.rkyv")
        );
    }

    #[test]
    fn default_path_skips_xdg_when_archive_missing() {
        let g = env_map(&[("XDG_DATA_HOME", "/xdg"), ("HOME", "/home/u")]);
        assert_eq!(
            default_path_with(g, |_| false),
            PathBuf::from("/home/u/.awkrs/scripts.rkyv")
        );
    }

    #[test]
    fn default_path_falls_back_to_home_dot_awkrs() {
        let g = env_map(&[("HOME", "/home/u")]);
        assert_eq!(
            default_path_with(g, |_| false),
            PathBuf::from("/home/u/.awkrs/scripts.rkyv")
        );
    }

    #[test]
    fn default_path_with_no_env_returns_relative_dot_awkrs() {
        let g = env_map(&[]);
        assert_eq!(
            default_path_with(g, |_| false),
            PathBuf::from(".awkrs/scripts.rkyv")
        );
    }

    // ---- human_bytes scaling boundaries ----

    #[test]
    fn human_bytes_zero_is_plain_bytes() {
        assert_eq!(human_bytes(0), "0B");
    }

    #[test]
    fn human_bytes_under_kib_stays_in_bytes() {
        assert_eq!(human_bytes(1), "1B");
        assert_eq!(human_bytes(1023), "1023B");
    }

    #[test]
    fn human_bytes_kib_boundary() {
        assert_eq!(human_bytes(1024), "1.00K");
        assert_eq!(human_bytes(1025), "1.00K");
        assert_eq!(human_bytes(1536), "1.50K");
    }

    #[test]
    fn human_bytes_precision_tiers() {
        assert_eq!(human_bytes(9 * 1024), "9.00K");
        assert_eq!(human_bytes(10 * 1024), "10.0K");
        assert_eq!(human_bytes(99 * 1024), "99.0K");
        assert_eq!(human_bytes(100 * 1024), "100K");
        assert_eq!(human_bytes(1023 * 1024), "1023K");
    }

    #[test]
    fn human_bytes_mib_gib_tib() {
        assert_eq!(human_bytes(1024 * 1024), "1.00M");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.00G");
        assert_eq!(human_bytes(1024u64.pow(4)), "1.00T");
        assert_eq!(human_bytes(1024u64.pow(5)), "1.00P");
    }

    // ---- expand_tilde ----

    #[test]
    fn expand_tilde_resolves_home_prefix() {
        let home = std::env::var_os("HOME").expect("HOME must be set for this test");
        let got = expand_tilde("~/sub/dir.rkyv");
        assert_eq!(got, PathBuf::from(home).join("sub/dir.rkyv"));
    }

    #[test]
    fn expand_tilde_passes_absolute_path_through() {
        assert_eq!(
            expand_tilde("/absolute/path.rkyv"),
            PathBuf::from("/absolute/path.rkyv")
        );
    }

    #[test]
    fn expand_tilde_passes_relative_path_through() {
        assert_eq!(
            expand_tilde("relative/path.rkyv"),
            PathBuf::from("relative/path.rkyv")
        );
    }

    #[test]
    fn expand_tilde_does_not_expand_bare_tilde() {
        assert_eq!(expand_tilde("~"), PathBuf::from("~"));
    }

    // ---- format-token rendering ----

    #[test]
    fn rkyv_cache_disk_tokens_render_against_real_file() {
        let p = tmpfile("disk", &[0u8; 2048]);
        let r = rkyv_cache(p.to_str().unwrap(), "{size}|{bytes}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        let (size, bytes) = s.split_once('|').expect("token separator present");
        assert!(
            size.ends_with('K') || size.ends_with('B'),
            "{size} should carry a human suffix"
        );
        let n: u64 = bytes.parse().expect("{bytes} must be a raw integer");
        assert!(
            n >= 2048,
            "disk bytes must be >= logical bytes 2048, got {n}"
        );
    }

    #[test]
    fn rkyv_cache_all_four_size_tokens_render_together() {
        let p = tmpfile("mixed", &[0u8; 4096]);
        let r = rkyv_cache(
            p.to_str().unwrap(),
            "[{size}|{bytes}|{logical_size}|{logical_bytes}]",
            false,
        )
        .unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(
            s.starts_with('[') && s.ends_with(']'),
            "literal brackets preserved"
        );
        assert!(s.contains("|4.00K|"), "logical_size token rendered: {s}");
        assert!(s.contains("|4096]"), "logical_bytes token rendered: {s}");
    }

    #[test]
    fn rkyv_cache_icon_token_passes_through_unsubstituted() {
        let p = tmpfile("icon", &[0u8; 1]);
        let r = rkyv_cache(p.to_str().unwrap(), "{icon} {size}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(s.starts_with("{icon} "), "icon token preserved: {s:?}");
    }

    #[test]
    fn rkyv_cache_unknown_token_passes_through_unchanged() {
        let p = tmpfile("unknown", &[0u8; 1]);
        let r = rkyv_cache(p.to_str().unwrap(), "{wat}/{size}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(s.starts_with("{wat}/"), "unknown tokens preserved: {s:?}");
    }

    #[test]
    fn rkyv_cache_divider_highlight_group_is_background_divider() {
        let p = tmpfile("div", &[0u8; 1]);
        let r = rkyv_cache(p.to_str().unwrap(), "{size}", false).unwrap();
        assert_eq!(
            r[0]["divider_highlight_group"].as_str().unwrap(),
            "background:divider"
        );
    }

    #[test]
    fn rkyv_cache_empty_file_renders_zero_size() {
        let p = tmpfile("empty", &[]);
        let r = rkyv_cache(p.to_str().unwrap(), "{logical_size}/{logical_bytes}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "0B/0");
    }

    #[test]
    fn rkyv_cache_highlight_group_chain_is_awkrs_specific() {
        let p = tmpfile("hl2", &[0u8; 1]);
        let r = rkyv_cache(p.to_str().unwrap(), "{size}", false).unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0].as_str().unwrap(), "awkrs_rkyv_cache");
        assert_eq!(groups[1].as_str().unwrap(), "awkrs");
    }
}
