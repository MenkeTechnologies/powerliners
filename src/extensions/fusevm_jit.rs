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
//! - `{icon}`          — fusevm/JIT glyph
//! - `{entries}`       — file count under the cache root (recursive)
//! - `{size}`          — on-disk allocation, human-formatted (matches `du -sh`)
//! - `{bytes}`         — on-disk allocation, raw integer
//! - `{logical_size}`  — sum of file content sizes, human-formatted (`stat`-style)
//! - `{logical_bytes}` — sum of file content sizes, raw integer

use serde_json::{json, Value};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JitStats {
    pub entries: u64,
    /// Sum of logical content sizes (`meta.len()`). Matches what the
    /// fjit files were *written* to disk as.
    pub bytes: u64,
    /// Sum of on-disk block allocations (`meta.blocks() * 512` on Unix,
    /// rounded up to the filesystem block size). Matches `du -sh`,
    /// which is the user mental model for "how much disk is the cache
    /// using" — for many tiny files this is 10-40x larger than `bytes`
    /// because each file pads to one filesystem block (commonly 4 KB).
    /// On Windows, equals `bytes` (no `st_blocks` equivalent without
    /// FSCTL_QUERY_FILE_REGIONS).
    pub disk_bytes: u64,
}

#[cfg(unix)]
fn block_bytes(meta: &std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    // st_blocks is in POSIX-mandated 512-byte units regardless of the
    // filesystem's actual block size.
    meta.blocks() * 512
}

#[cfg(not(unix))]
fn block_bytes(meta: &std::fs::Metadata) -> u64 {
    meta.len()
}

fn default_root() -> PathBuf {
    default_root_with(|k| std::env::var_os(k))
}

/// Pure-functional core of `default_root()` — takes the env-var lookup
/// as a parameter so the 3-level resolution chain
/// (`$FUSEVM_JIT_CACHE` → `$XDG_CACHE_HOME/fusevm-jit`
/// → `~/.cache/fusevm-jit`) can be unit-tested without mutating the
/// process env. Mirrors the seam in the rkyv-cache sibling modules.
fn default_root_with(get_env: impl Fn(&str) -> Option<OsString>) -> PathBuf {
    if let Some(p) = get_env("FUSEVM_JIT_CACHE") {
        return PathBuf::from(p);
    }
    let xdg = get_env("XDG_CACHE_HOME").map(PathBuf::from);
    let home = get_env("HOME").map(PathBuf::from);
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
                stats.disk_bytes = stats.disk_bytes.saturating_add(block_bytes(&meta));
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
    // {size} and {bytes} report on-disk allocation (matches `du -sh`),
    // not logical content sum — for caches of many tiny files the
    // logical sum can be 10-40x smaller than disk usage and the
    // resulting "23K" reads as fake when the user can see `du` says
    // 972K. {logical_size}/{logical_bytes} expose the old behavior
    // for callers that want it.
    let contents = format
        .replace("{entries}", &stats.entries.to_string())
        .replace("{bytes}", &stats.disk_bytes.to_string())
        .replace("{size}", &human_bytes(stats.disk_bytes))
        .replace("{logical_bytes}", &stats.bytes.to_string())
        .replace("{logical_size}", &human_bytes(stats.bytes));
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
    fn scan_dir_counts_files_and_logical_bytes() {
        let d = tmpdir("counts");
        writef(&d.join("a"), &[0u8; 100]);
        writef(&d.join("b"), &[0u8; 250]);
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 2);
        // .bytes is the sum of `meta.len()` — deterministic across
        // filesystems.
        assert_eq!(s.bytes, 350);
    }

    #[test]
    fn scan_dir_disk_bytes_is_at_least_logical_bytes() {
        // .disk_bytes counts block allocation, which on every real
        // filesystem rounds up to the block size — so for any
        // non-empty file with content, disk_bytes >= bytes. The exact
        // multiplier depends on the FS (4K blocks → 100B file padded
        // to 4096 disk bytes); we only assert the invariant here so
        // the test is portable.
        let d = tmpdir("disk-vs-logical");
        writef(&d.join("a"), &[0u8; 100]);
        writef(&d.join("b"), &[0u8; 250]);
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 2);
        assert!(
            s.disk_bytes >= s.bytes,
            "disk_bytes ({}) must be ≥ logical bytes ({})",
            s.disk_bytes,
            s.bytes
        );
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
    fn jit_cache_size_token_renders_disk_allocation() {
        // {size} must match the du-style on-disk allocation, not the
        // logical content sum — the user reported the segment was
        // "fake" because it showed 23K when `du -sh` said 972K.
        // Pin: for any populated cache, the rendered {size} reflects
        // disk_bytes (≥ logical bytes).
        let d = tmpdir("size-token");
        writef(&d.join("a"), &[0u8; 100]);
        let r = jit_cache(d.to_str().unwrap(), "{bytes}/{logical_bytes}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        let (disk, logical) = s.split_once('/').unwrap();
        let disk: u64 = disk.parse().unwrap();
        let logical: u64 = logical.parse().unwrap();
        assert_eq!(logical, 100);
        assert!(
            disk >= logical,
            "disk ({}) must be ≥ logical ({})",
            disk,
            logical
        );
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
        // Drive through {logical_size}/{logical_bytes} so the test is
        // independent of filesystem block size. {size}/{bytes} are
        // covered by the dedicated disk-allocation test.
        let d = tmpdir("format");
        writef(&d.join("a"), &[0u8; 2048]);
        let r = jit_cache(
            d.to_str().unwrap(),
            "{entries}/{logical_size}/{logical_bytes}",
            false,
        )
        .unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert_eq!(s, "1/2.00K/2048");
    }

    #[test]
    fn jit_cache_missing_root_returns_none_when_hidden() {
        let r = jit_cache("/nonexistent/fusevm-zzz-yyy", "{entries} {size}", false);
        assert!(r.is_none());
    }

    #[test]
    fn jit_cache_missing_root_renders_zero_when_show_when_empty() {
        // Pre-cache state — fusevm hasn't populated anything yet but
        // the user wants the segment slot visible. Render 0/0B.
        let r = jit_cache("/nonexistent/fusevm-zzz-yyy", "{entries} {size}", true).unwrap();
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

    // ---- default_root resolution chain (pure-functional via closure) ----

    fn env_map(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let owned: Vec<(String, OsString)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), OsString::from(*v)))
            .collect();
        move |k: &str| owned.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn default_root_fusevm_jit_cache_wins_over_everything() {
        let g = env_map(&[
            ("FUSEVM_JIT_CACHE", "/explicit/cache/root"),
            ("XDG_CACHE_HOME", "/xdg/cache"),
            ("HOME", "/home/u"),
        ]);
        assert_eq!(default_root_with(g), PathBuf::from("/explicit/cache/root"));
    }

    #[test]
    fn default_root_xdg_cache_home_wins_over_home() {
        let g = env_map(&[("XDG_CACHE_HOME", "/xdg/cache"), ("HOME", "/home/u")]);
        assert_eq!(default_root_with(g), PathBuf::from("/xdg/cache/fusevm-jit"));
    }

    #[test]
    fn default_root_falls_back_to_home_dot_cache() {
        let g = env_map(&[("HOME", "/home/u")]);
        assert_eq!(
            default_root_with(g),
            PathBuf::from("/home/u/.cache/fusevm-jit")
        );
    }

    #[test]
    fn default_root_with_no_env_falls_back_to_tmp() {
        // Last-resort path when neither XDG_CACHE_HOME nor HOME is set —
        // documents the actual behavior (vs leaving a `None` that would
        // crash the prompt later).
        let g = env_map(&[]);
        assert_eq!(default_root_with(g), PathBuf::from("/tmp/fusevm-jit"));
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
        // <10 → 2 decimals, <100 → 1 decimal, else → 0 decimals
        assert_eq!(human_bytes(9 * 1024), "9.00K");
        assert_eq!(human_bytes(10 * 1024), "10.0K");
        assert_eq!(human_bytes(99 * 1024), "99.0K");
        assert_eq!(human_bytes(100 * 1024), "100K");
        assert_eq!(human_bytes(1023 * 1024), "1023K");
    }

    #[test]
    fn human_bytes_full_suffix_chain() {
        assert_eq!(human_bytes(1024 * 1024), "1.00M");
        assert_eq!(human_bytes(1024 * 1024 * 1024), "1.00G");
        assert_eq!(human_bytes(1024u64.pow(4)), "1.00T");
        assert_eq!(human_bytes(1024u64.pow(5)), "1.00P");
    }

    // ---- expand_tilde edge cases ----

    #[test]
    fn expand_tilde_passes_relative_through() {
        assert_eq!(expand_tilde("relative/dir"), PathBuf::from("relative/dir"));
    }

    #[test]
    fn expand_tilde_does_not_expand_bare_tilde() {
        // Lone `~` (no trailing slash) is not expanded — matches what
        // the function actually does, vs the shell's HOME expansion.
        assert_eq!(expand_tilde("~"), PathBuf::from("~"));
    }

    // ---- scan_dir edge cases ----

    #[test]
    fn scan_dir_counts_empty_file_as_entry_but_zero_bytes() {
        let d = tmpdir("empty-file");
        writef(&d.join("zero"), &[]);
        let s = scan_dir(&d).unwrap();
        assert_eq!(s.entries, 1);
        assert_eq!(s.bytes, 0);
    }

    #[cfg(unix)]
    #[test]
    fn scan_dir_counts_symlink_as_entry_without_traversing() {
        // Symlinks must count as a single entry and never be followed —
        // the comment in scan_dir notes this is to avoid infinite loops
        // across re-symlinked cache dirs.
        let d = tmpdir("symlink");
        writef(&d.join("real"), &[0u8; 50]);
        std::os::unix::fs::symlink(d.join("real"), d.join("link")).unwrap();
        let s = scan_dir(&d).unwrap();
        // 1 real file + 1 symlink entry; symlink contributes 0 bytes
        // because it isn't followed.
        assert_eq!(s.entries, 2);
        assert_eq!(s.bytes, 50);
    }

    // ---- format-token rendering ----

    #[test]
    fn jit_cache_size_token_renders_human_formatted_disk_allocation() {
        // {size} is the human-formatted form of disk_bytes. For any
        // populated cache disk_bytes >= logical, so {size} must carry a
        // suffix from the B/K/M/G chain.
        let d = tmpdir("size-human");
        writef(&d.join("a"), &[0u8; 4096]);
        let r = jit_cache(d.to_str().unwrap(), "{size}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(
            s.ends_with('B') || s.ends_with('K') || s.ends_with('M'),
            "{{size}} must end with a human suffix, got {s:?}"
        );
    }

    #[test]
    fn jit_cache_all_five_tokens_render_together() {
        // The full token surface: entries + 4 size-flavored tokens.
        let d = tmpdir("all-tokens");
        writef(&d.join("a"), &[0u8; 4096]);
        let r = jit_cache(
            d.to_str().unwrap(),
            "[{entries}|{size}|{bytes}|{logical_size}|{logical_bytes}]",
            false,
        )
        .unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(
            s.starts_with('[') && s.ends_with(']'),
            "literal brackets preserved: {s:?}"
        );
        assert!(s.contains("|1|") || s.starts_with("[1|"), "entries=1: {s}");
        assert!(s.contains("|4.00K|"), "logical_size=4.00K: {s}");
        assert!(s.contains("|4096]"), "logical_bytes=4096: {s}");
    }

    #[test]
    fn jit_cache_icon_token_passes_through_unsubstituted() {
        // The {icon} token is substituted downstream by the render
        // runtime, not by this module — it must reach the caller
        // untouched.
        let d = tmpdir("icon");
        writef(&d.join("a"), &[0u8; 1]);
        let r = jit_cache(d.to_str().unwrap(), "{icon} {entries}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(s.starts_with("{icon} "), "icon token preserved: {s:?}");
    }

    #[test]
    fn jit_cache_unknown_token_passes_through_unchanged() {
        let d = tmpdir("unknown");
        writef(&d.join("a"), &[0u8; 1]);
        let r = jit_cache(d.to_str().unwrap(), "{wat}/{entries}", false).unwrap();
        let s = r[0]["contents"].as_str().unwrap();
        assert!(s.starts_with("{wat}/"), "unknown tokens preserved: {s:?}");
    }

    #[test]
    fn jit_cache_divider_highlight_group_is_background_divider() {
        let d = tmpdir("divider");
        writef(&d.join("a"), &[0u8; 1]);
        let r = jit_cache(d.to_str().unwrap(), "{entries}", false).unwrap();
        assert_eq!(
            r[0]["divider_highlight_group"].as_str().unwrap(),
            "background:divider"
        );
    }

    #[test]
    fn jit_cache_highlight_group_chain_is_fusevm_specific() {
        // The first two groups identify the segment in user themes —
        // part of the public theming contract, not just the neutral
        // fallback tail. The chain is documented inline at the call
        // site and must not drift.
        let d = tmpdir("hl-chain");
        writef(&d.join("a"), &[0u8; 1]);
        let r = jit_cache(d.to_str().unwrap(), "{entries}", false).unwrap();
        let groups = r[0]["highlight_groups"].as_array().unwrap();
        assert_eq!(groups[0].as_str().unwrap(), "fusevm_jit_cache");
        assert_eq!(groups[1].as_str().unwrap(), "fusevm");
    }
}
