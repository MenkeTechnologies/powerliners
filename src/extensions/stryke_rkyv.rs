// vim:fileencoding=utf-8:noet
//! stryke rkyv-cache segment — reports the on-disk allocation of the
//! authoritative compiled-bytecode archive at
//! `<root>/scripts.rkyv`. stryke (Perl-superset + Clojure-threading +
//! Scala-underscore + Ruby-syntax synthesis, Cranelift-JIT'd via the
//! shared fusevm runtime) writes its bytecode store as a single rkyv
//! archive; this segment shows "how warm is the stryke cache" at a
//! glance.
//!
//! Pure filesystem probe — single `stat` of the archive file.
//!
//! Default resolution:
//! 1. `$STRYKE_RKYV_CACHE`           — explicit override
//! 2. `$STRYKE_HOME/scripts.rkyv`
//! 3. `$XDG_DATA_HOME/stryke/scripts.rkyv`
//! 4. `~/.stryke/scripts.rkyv`
//!
//! Returns `None` when the file doesn't exist (stryke never ran on
//! this machine).
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.stryke.rkyv_cache",
//!   "args": {
//!     "path": "~/.stryke/scripts.rkyv",
//!     "format": "{icon} {size}",
//!     "show_when_empty": false
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{icon}`          — stryke glyph
//! - `{size}`          — file on-disk allocation, human-formatted (matches `du -sh`)
//! - `{bytes}`         — file on-disk allocation, raw integer
//! - `{logical_size}`  — file content size (`stat .st_size`), human-formatted
//! - `{logical_bytes}` — file content size, raw integer

use serde_json::{json, Value};
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
    if let Some(p) = std::env::var_os("STRYKE_RKYV_CACHE") {
        return PathBuf::from(p);
    }
    if let Some(home) = std::env::var_os("STRYKE_HOME") {
        return PathBuf::from(home).join("scripts.rkyv");
    }
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg).join("stryke").join("scripts.rkyv");
        if p.exists() {
            return p;
        }
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".stryke").join("scripts.rkyv")
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
        "highlight_groups": ["stryke_rkyv_cache", "stryke", "information:regular"],
        "divider_highlight_group": "background:divider",
    })])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;

    fn tmpfile(name: &str, bytes: &[u8]) -> PathBuf {
        let p = std::env::temp_dir().join(format!("powerliners-stryke-{name}.rkyv"));
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(bytes).unwrap();
        p
    }

    #[test]
    fn stat_missing_returns_none() {
        assert!(stat_file(Path::new("/nonexistent/stryke-xyz.rkyv")).is_none());
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
        let r = rkyv_cache("/nonexistent/stryke-zzz.rkyv", "{size}", false);
        assert!(r.is_none());
    }

    #[test]
    fn rkyv_cache_missing_renders_zero_when_show_when_empty() {
        let r = rkyv_cache("/nonexistent/stryke-zzz.rkyv", "{size}", true).unwrap();
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
}
