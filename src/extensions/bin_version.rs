// vim:fileencoding=utf-8:noet
//! Shared helper for the `<tool>_version` segments — runs
//! `<bin> --version`, parses the first SemVer-shaped token out of
//! stdout, and caches the result in-memory for `ttl` so the daemon
//! doesn't fork on every prompt tick.
//!
//! The cache is global (single `Mutex<HashMap>`) — keyed by the
//! resolved binary path + args. Bounded by the number of distinct
//! version segments wired in the theme, so no eviction policy is
//! needed.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct CacheEntry {
    version: String,
    captured_at: Instant,
}

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    static C: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Return the cached version for `bin args...` if fresher than `ttl`.
/// Hot path on every prompt tick when the version cache is warm.
pub fn cached(key: &str, ttl: Duration) -> Option<String> {
    let g = cache().lock().ok()?;
    let entry = g.get(key)?;
    if Instant::now().duration_since(entry.captured_at) > ttl {
        return None;
    }
    Some(entry.version.clone())
}

fn store(key: String, version: String) {
    if let Ok(mut g) = cache().lock() {
        g.insert(
            key,
            CacheEntry {
                version,
                captured_at: Instant::now(),
            },
        );
    }
}

/// Run `bin args...`, parse the first SemVer-shaped token from
/// stdout+stderr, cache the result keyed by `bin` + ` ` + args joined,
/// and return it.
///
/// Returns `None` when `bin` isn't on `PATH`, the process exits
/// non-zero, or no version-shaped token appears in the output.
pub fn get(bin: &str, args: &[&str], ttl: Duration) -> Option<String> {
    let key = format!("{bin} {}", args.join(" "));
    if let Some(v) = cached(&key, ttl) {
        return Some(v);
    }
    let out = Command::new(bin).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    // Some tools (awkrs --version with -V) emit on stderr.
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = if stdout.trim().is_empty() {
        stderr.to_string()
    } else {
        stdout.to_string()
    };
    let v = extract_version(&combined)?;
    store(key, v.clone());
    Some(v)
}

/// Pull the first `[v]?MAJOR.MINOR.PATCH[-prerelease]` token out of
/// arbitrary `--version` output. Handles the three live shapes:
///
/// - `"zshrs 0.11.26"`                                 → `0.11.26`
/// - `"This is stryke v0.16.8 — ..."`                  → `0.16.8`
/// - `"awkrs 0.4.13"`                                  → `0.4.13`
pub fn extract_version(out: &str) -> Option<String> {
    for raw in out.split_whitespace() {
        let token = raw.strip_prefix('v').unwrap_or(raw);
        if !token.chars().next()?.is_ascii_digit() {
            continue;
        }
        // SemVer-shape body: digits/dots/dashes/plusses/alphanumerics.
        // Captures `1.2.3`, `1.2.3-rc.4`, `1.2.3+build.7`, etc.
        let body_end = token
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+'))
            .unwrap_or(token.len());
        let body = &token[..body_end];
        // Require MAJOR.MINOR.PATCH — at least two dots inside the
        // numeric prefix (before any `-` or `+`).
        let numeric_prefix_end = body.find(['-', '+']).unwrap_or(body.len());
        if body[..numeric_prefix_end]
            .chars()
            .filter(|c| *c == '.')
            .count()
            < 2
        {
            continue;
        }
        let cleaned = body.trim_end_matches(['.', '-']);
        if cleaned.is_empty() {
            continue;
        }
        return Some(cleaned.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_handles_zshrs_shape() {
        assert_eq!(extract_version("zshrs 0.11.26"), Some("0.11.26".into()));
    }

    #[test]
    fn extract_version_handles_stryke_shape() {
        assert_eq!(
            extract_version("This is stryke v0.16.8 — A highly parallel Perl 5 interpreter (Rust)"),
            Some("0.16.8".into())
        );
    }

    #[test]
    fn extract_version_handles_awkrs_shape() {
        assert_eq!(extract_version("awkrs 0.4.13"), Some("0.4.13".into()));
    }

    #[test]
    fn extract_version_handles_v_prefix_with_newline() {
        assert_eq!(
            extract_version("\nfoo v1.2.3-rc.4 build 99\n"),
            Some("1.2.3-rc.4".into())
        );
    }

    #[test]
    fn extract_version_returns_none_on_no_version() {
        assert_eq!(extract_version("nothing version-shaped here"), None);
        assert_eq!(extract_version(""), None);
    }

    #[test]
    fn extract_version_skips_two_dot_words_with_no_digit_prefix() {
        // Edge: "abc.def.ghi" isn't a SemVer.
        assert_eq!(extract_version("abc.def.ghi"), None);
    }

    #[test]
    fn extract_version_picks_first_match() {
        // If multiple version-shaped tokens appear, take the first.
        assert_eq!(
            extract_version("zshrs 0.11.26 (libfoo 9.9.9)"),
            Some("0.11.26".into())
        );
    }

    #[test]
    fn cached_returns_none_on_cold_cache() {
        // Use a key that no other test would touch.
        assert!(cached("__never_set_key__", Duration::from_secs(60)).is_none());
    }

    #[test]
    fn cache_round_trip_under_ttl() {
        let key = "__bin_version_round_trip__";
        store(key.to_string(), "9.9.9".to_string());
        assert_eq!(
            cached(key, Duration::from_secs(60)),
            Some("9.9.9".to_string())
        );
    }

    #[test]
    fn cache_expires_past_ttl() {
        let key = "__bin_version_ttl__";
        store(key.to_string(), "1.0.0".to_string());
        // Zero TTL → already expired.
        assert!(cached(key, Duration::from_secs(0)).is_none());
    }

    #[test]
    fn get_missing_binary_returns_none() {
        let r = get(
            "/nonexistent/version-probe-xyz",
            &["--version"],
            Duration::from_secs(30),
        );
        assert!(r.is_none());
    }
}
