// vim:fileencoding=utf-8:noet
//! fusevm JIT cache segment — reports the on-disk Cranelift JIT
//! cache size + entry count for the fusevm runtime used by zshrs and
//! stryke. Lets the prompt show "how warm is the JIT" at a glance:
//! a cold cache after `cargo clean` vs. a hot cache during normal
//! development.
//!
//! Pure filesystem probe — no fusevm dependency at compile time. The
//! cache layout is "one file per compiled artifact" so a recursive
//! `read_dir` walk gives both metrics in one pass.
//!
//! Default cache root resolution:
//! 1. `$FUSEVM_JIT_CACHE`
//! 2. `$XDG_CACHE_HOME/fusevm-jit`
//! 3. `~/.cache/fusevm-jit`
//!
//! Returns `None` when the cache root doesn't exist (cache never
//! warmed) — the segment is informational, not load-bearing.
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.fusevm.jit_cache",
//!   "args": {
//!     "path": "~/.cache/fusevm/jit",
//!     "format": "{icon} {entries} {size}",
//!     "show_when_empty": false
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{icon}`    — fusevm/JIT glyph
//! - `{entries}` — file count under the cache root (recursive)
//! - `{size}`    — total bytes, human-formatted (K/M/G suffix)
//! - `{bytes}`   — total bytes, raw integer (useful for custom format)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JitStats {
    pub entries: u64,
    pub bytes: u64,
}

fn default_root() -> PathBuf {
    if let Some(p) = std::env::var_os("FUSEVM_JIT_CACHE") {
        return PathBuf::from(p);
    }
    let xdg = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from);
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = xdg
        .or_else(|| home.map(|h| h.join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("fusevm-jit")
}

/// Expand a leading `~/` against `$HOME`. Themes write `~/...` paths
/// because the JSON config is hand-edited; expanding here keeps the
/// public API a single string.
fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(p)
}

/// Recursively tally files + bytes under `root`. Symlinks are NOT
/// followed (avoids infinite loops if the cache directory is
/// re-symlinked across filesystems). Permission errors and unreadable
/// entries are silently skipped — the prompt should never fail loud.
pub fn scan_dir(root: &Path) -> Option<JitStats> {
    if !root.exists() {
        return None;
    }
    let mut stats = JitStats::default();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(it) => it,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            // Use the no-traverse metadata so symlinks count as files
            // (size 0 if dangling) instead of being followed.
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let ft = meta.file_type();
            if ft.is_symlink() {
                stats.entries += 1;
                continue;
            }
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                stats.entries += 1;
                stats.bytes = stats.bytes.saturating_add(meta.len());
            }
        }
    }
    Some(stats)
}

/// Human-format a byte count with B/K/M/G/T suffixes (1024-base, two
/// decimal places under 100, one decimal place under 1000).
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

/// Render the segment. `path` is the theme-provided cache root; pass
/// an empty string to use the default resolution chain.
pub fn jit_cache(path: &str, format: &str, show_when_empty: bool) -> Option<Vec<Value>> {
    let root = if path.is_empty() {
        default_root()
    } else {
        expand_tilde(path)
    };
    let stats = match scan_dir(&root) {
        Some(s) => s,
        None if show_when_empty => JitStats::default(),
        None => return None,
    };
    if stats.entries == 0 && !show_when_empty {
        return None;
    }
    let contents = format
        .replace("{entries}", &stats.entries.to_string())
        .replace("{bytes}", &stats.bytes.to_string())
        .replace("{size}", &human_bytes(stats.bytes));
    Some(vec![json!({
        "contents": contents,
        // Fallback chain: theme-specific fusevm_jit_cache → fusevm
        // family group → information:regular (always defined in any
        // standard powerline colorscheme). The terminal fallback
        // means the chunk renders even when the user hasn't themed
        // fusevm explicitly — instead of being silently dropped by
        // the renderer for an unknown highlight group.
        "highlight_groups": ["fusevm_jit_cache", "fusevm", "information:regular"],
        "divider_highlight_group": "background:divider",
    })])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("powerliners-fusevm-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn writef(path: &Path, bytes: &[u8]) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(bytes).unwrap();
    }

    #[test]
    fn scan_dir_missing_root_returns_none() {
        assert!(scan_dir(Path::new("/nonexistent/fusevm-xyz-zzz")).is_none());
    }

    #[test]
    fn scan_dir_empty_root_returns_zero_stats() {
        let d = tmpdir("empty");
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 0);
        assert_eq!(s.bytes, 0);
    }

    #[test]
    fn scan_dir_counts_files_and_bytes() {
        let d = tmpdir("counts");
        writef(&d.join("a"), &[0u8; 100]);
        writef(&d.join("b"), &[0u8; 250]);
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 2);
        assert_eq!(s.bytes, 350);
    }

    #[test]
    fn scan_dir_recursive() {
        let d = tmpdir("recursive");
        fs::create_dir_all(d.join("nested/deep")).unwrap();
        writef(&d.join("a"), &[0u8; 10]);
        writef(&d.join("nested/b"), &[0u8; 20]);
        writef(&d.join("nested/deep/c"), &[0u8; 30]);
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 3);
        assert_eq!(s.bytes, 60);
    }

    #[test]
    fn human_bytes_renders_units() {
        assert_eq!(human_bytes(0), "0B");
        assert_eq!(human_bytes(512), "512B");
        assert_eq!(human_bytes(1024), "1.00K");
        assert_eq!(human_bytes(1536), "1.50K");
        assert_eq!(human_bytes(1024 * 1024), "1.00M");
        assert_eq!(human_bytes(1024u64.pow(3)), "1.00G");
    }

    #[test]
    fn human_bytes_large_values_drop_decimals() {
        // 150 KiB should render as "150K" (no decimal beyond 100).
        assert_eq!(human_bytes(150 * 1024), "150K");
    }

    #[test]
    fn jit_cache_empty_dir_no_show_returns_none() {
        let d = tmpdir("empty-noshow");
        let r = jit_cache(d.to_str().unwrap(), "{entries} {size}", false);
        assert!(r.is_none());
    }

    #[test]
    fn jit_cache_empty_dir_show_renders_zero() {
        let d = tmpdir("empty-show");
        let r = jit_cache(d.to_str().unwrap(), "{entries} {size}", true).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "0 0B");
    }

    #[test]
    fn jit_cache_format_tokens_render() {
        let d = tmpdir("format");
        writef(&d.join("a"), &[0u8; 2048]);
        let r = jit_cache(
            d.to_str().unwrap(),
            "{entries}/{size}/{bytes}",
            false,
        )
        .unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "1/2.00K/2048");
    }

    #[test]
    fn jit_cache_missing_root_returns_none_when_hidden() {
        let r = jit_cache(
            "/nonexistent/fusevm-zzz-yyy",
            "{entries} {size}",
            false,
        );
        assert!(r.is_none());
    }

    #[test]
    fn jit_cache_missing_root_renders_zero_when_show_when_empty() {
        // Pre-cache state — fusevm hasn't populated anything yet but
        // the user wants the segment slot visible. Render 0/0B.
        let r = jit_cache(
            "/nonexistent/fusevm-zzz-yyy",
            "{entries} {size}",
            true,
        )
        .unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "0 0B");
    }

    #[test]
    fn jit_cache_highlight_groups_have_neutral_fallback() {
        // Renderers silently drop chunks whose highlight group isn't
        // defined in the colorscheme. The chain must end in a group
        // ('information:regular') that exists in every standard
        // powerline colorscheme — otherwise the segment vanishes from
        // the bar in any theme that hasn't been customized for fusevm.
        let d = tmpdir("hl-fallback");
        writef(&d.join("a"), &[0u8; 1]);
        let r = jit_cache(d.to_str().unwrap(), "{entries}", false).unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        let last = groups.last().unwrap().as_str().unwrap();
        assert_eq!(last, "information:regular");
    }

    #[test]
    fn expand_tilde_expands_home() {
        if let Some(home) = std::env::var_os("HOME") {
            let p = expand_tilde("~/foo");
            assert_eq!(p, PathBuf::from(home).join("foo"));
        }
    }

    #[test]
    fn expand_tilde_passes_absolute_through() {
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }
}
