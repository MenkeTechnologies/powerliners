// vim:fileencoding=utf-8:noet
//! Non-port helpers for the weather segment.
//!
//! Upstream `powerline-status` resolves location via `urllib.request`
//! and a `location_query` arg only — no IP-based geolocation, no
//! on-disk cache. The Rust port adds those features so the segment
//! survives a 429 spell or a daemon restart without blanking the
//! prompt. Per `docs/PORT.md` the strict 1:1 port lives under
//! `src/ported/`; functions that have no upstream Python counterpart
//! live here in `src/extensions/`.
//!
//! Exposed surface:
//!
//! - [`urlencode`]            — minimal `application/x-www-form-urlencoded`
//!   for the `&q=…` location parameter.
//! - [`http_get`]             — short-timeout `ureq` GET that returns
//!   the response body as a `String`, `None` on any failure.
//! - [`ip_geolocate`]         — coordinator: try `ipinfo.io` → `ipapi.co`
//!   live, fall back to the on-disk cache when both fail; persist a
//!   live result for the next call.
//! - [`ipinfo_geolocate`]     — provider impl, parses `loc: "lat,lon"`.
//! - [`ipapi_geolocate`]      — provider impl, parses separate
//!   `latitude`/`longitude` keys.
//! - [`location_cache_path`]  — `~/.powerliners/location.json`.
//! - [`save_location_cache`]  — persist the latest `(lat, lon)`.
//! - [`load_location_cache`]  — read the persisted `(lat, lon)`.

use serde_json::Value;

/// Minimal `application/x-www-form-urlencoded` for the only chars
/// likely in a location query (`+ ,`). Saves pulling in a urlencoding
/// crate for one segment's worth of work.
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Short-timeout GET. Returns the response body as a `String`, or
/// `None` on any error (DNS, TLS, timeout, non-UTF8 body).
pub fn http_get(url: &str) -> Option<String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(5)))
        .build()
        .into();
    let mut resp = agent.get(url).call().ok()?;
    resp.body_mut().read_to_string().ok()
}

/// Probe order: `ipinfo.io` (returns `"loc": "lat,lon"`) → `ipapi.co`
/// (returns `latitude`/`longitude` as separate keys). The first
/// probe to give a usable answer wins; failures (rate limit, DNS,
/// etc.) fall through silently. Successful lookups are persisted to
/// disk via [`save_location_cache`] so a daemon restart or a 429
/// spell doesn't blank the weather segment — we use the last known
/// location until a fresh lookup succeeds.
pub fn ip_geolocate() -> Option<(f64, f64)> {
    let fresh = ipinfo_geolocate().or_else(ipapi_geolocate);
    if let Some(coords) = fresh {
        save_location_cache(coords);
        return Some(coords);
    }
    load_location_cache()
}

pub fn ipinfo_geolocate() -> Option<(f64, f64)> {
    let raw = http_get("https://ipinfo.io/json")?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let loc = v.get("loc")?.as_str()?;
    let (lat, lon) = loc.split_once(',')?;
    Some((lat.parse().ok()?, lon.parse().ok()?))
}

pub fn ipapi_geolocate() -> Option<(f64, f64)> {
    let raw = http_get("https://ipapi.co/json")?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let lat = v.get("latitude")?.as_f64()?;
    let lon = v.get("longitude")?.as_f64()?;
    Some((lat, lon))
}

pub fn location_cache_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut p = std::path::PathBuf::from(home);
    p.push(".powerliners");
    std::fs::create_dir_all(&p).ok()?;
    p.push("location.json");
    Some(p)
}

pub fn save_location_cache(coords: (f64, f64)) {
    if let Some(path) = location_cache_path() {
        let _ = std::fs::write(
            path,
            format!(r#"{{"lat":{},"lon":{}}}"#, coords.0, coords.1),
        );
    }
}

pub fn load_location_cache() -> Option<(f64, f64)> {
    let path = location_cache_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&raw).ok()?;
    let lat = v.get("lat")?.as_f64()?;
    let lon = v.get("lon")?.as_f64()?;
    Some((lat, lon))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_passes_through_safe_chars() {
        assert_eq!(urlencode("abc-DEF_123.~"), "abc-DEF_123.~");
    }

    #[test]
    fn urlencode_space_becomes_plus() {
        assert_eq!(urlencode("hello world"), "hello+world");
    }

    #[test]
    fn urlencode_percent_escapes_unsafe_bytes() {
        // ',' (0x2C), '+' (0x2B), '/' (0x2F) — all must percent-encode.
        assert_eq!(urlencode(","), "%2C");
        assert_eq!(urlencode("+"), "%2B");
        assert_eq!(urlencode("a/b"), "a%2Fb");
    }

    #[test]
    fn urlencode_combines_paths() {
        // City name with comma + country code, the typical OWM
        // `&q=` payload shape.
        assert_eq!(urlencode("San Francisco,US"), "San+Francisco%2CUS");
    }

    #[test]
    fn urlencode_empty_is_empty() {
        assert_eq!(urlencode(""), "");
    }

    #[test]
    fn urlencode_handles_high_bytes() {
        // Non-ASCII (UTF-8 multi-byte) must percent-encode each byte.
        // 'é' = 0xC3 0xA9.
        assert_eq!(urlencode("é"), "%C3%A9");
    }

    #[test]
    fn location_cache_path_returns_powerliners_dir_when_home_set() {
        // HOME mutation across parallel tests is unsafe; we only
        // assert the shape when HOME is already set in the env.
        if let Some(home) = std::env::var_os("HOME") {
            let p = location_cache_path().expect("HOME set → Some");
            let home_str = home.to_string_lossy();
            assert!(
                p.starts_with(home_str.as_ref()),
                "expected path under $HOME, got {}",
                p.display()
            );
            assert_eq!(
                p.file_name().and_then(|s| s.to_str()),
                Some("location.json")
            );
            assert!(
                p.parent()
                    .and_then(|s| s.file_name())
                    .and_then(|s| s.to_str())
                    == Some(".powerliners"),
                "expected .powerliners parent, got {}",
                p.display()
            );
        }
    }

    #[test]
    fn save_then_load_location_cache_round_trips() {
        // Same-process round-trip writes to ~/.powerliners/location.json
        // and reads back. We can't fully isolate from the user's real
        // cache without HOME mutation, so we (a) capture whatever's
        // there first, (b) overwrite with a sentinel, (c) verify the
        // sentinel reads back, (d) restore the original.
        if location_cache_path().is_none() {
            return; // HOME not set — nothing to round-trip.
        }
        let original = load_location_cache();
        let sentinel = (12.34_f64, -56.78_f64);
        save_location_cache(sentinel);
        let read_back = load_location_cache().expect("just wrote → Some");
        assert!((read_back.0 - sentinel.0).abs() < 1e-9);
        assert!((read_back.1 - sentinel.1).abs() < 1e-9);
        // Restore prior state so a parallel test doesn't see the
        // sentinel coords.
        match original {
            Some(prev) => save_location_cache(prev),
            None => {
                if let Some(path) = location_cache_path() {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }

    #[test]
    fn load_location_cache_returns_none_on_missing_file() {
        // Write garbage to the cache path then remove it — load must
        // be None. Restore behavior identical to round_trips test.
        if location_cache_path().is_none() {
            return;
        }
        let original = load_location_cache();
        if let Some(path) = location_cache_path() {
            let _ = std::fs::remove_file(&path);
        }
        assert!(load_location_cache().is_none());
        if let Some(prev) = original {
            save_location_cache(prev);
        }
    }
}
