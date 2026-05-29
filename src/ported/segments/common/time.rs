// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/time.py`.
//!
//! Date/time segments. Upstream uses Python's `datetime.strftime`;
//! Rust port uses `libc::strftime` over `libc::localtime_r`. The
//! `fuzzy_time` segment is large enough to deserve its own port pass
//! and is deferred.

// from __future__                                  // (implicit)
// from datetime import datetime, timezone          // py:2

use serde_json::{json, Value};

/// Format a time_t as strftime would for the given format spec.
fn format_strftime(fmt: &str, ts: libc::time_t) -> String {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: localtime_r writes into tm, which we own. Returns NULL
    // on failure; we propagate empty in that case.
    let p = unsafe { libc::localtime_r(&ts, &mut tm) };
    if p.is_null() {
        return String::new();
    }
    // Allocate a reasonable buffer for the formatted output.
    let mut buf = vec![0i8; 256];
    let cfmt = std::ffi::CString::new(fmt).unwrap_or_default();
    // SAFETY: cfmt is a NUL-terminated C string, buf has 256 bytes,
    // tm is a populated struct tm.
    let n = unsafe { libc::strftime(buf.as_mut_ptr(), buf.len(), cfmt.as_ptr(), &tm) };
    if n == 0 {
        return String::new();
    }
    // Convert i8 → u8 + take only n bytes.
    let bytes: Vec<u8> = buf[..n].iter().map(|&b| b as u8).collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Port of `date()` from `powerline/segments/common/time.py:5`.
///
/// Return the current date.
///
/// :param format: strftime-style date format string.
/// :param istime: If true then segment uses `time` highlight group.
/// :param timezone: Specify a timezone to use as `+HHMM` or `-HHMM`.
///     Defaults to system defaults. Currently ignored by the Rust
///     port (the underlying syscall always returns localtime).
///
/// Highlight groups used: `time` or `date`. Divider highlight group:
/// `time:divider`.
pub fn date(_pl: &(), format: &str, istime: bool, _timezone: Option<&str>) -> Vec<Value> {
    // py:23  nw = datetime.now(tz)
    let ts = unsafe { libc::time(std::ptr::null_mut()) };
    // py:25-29  contents = nw.strftime(format)
    let contents = format_strftime(format, ts);
    // py:31-35  return [{contents, highlight_groups, divider_highlight_group}]
    let highlight_groups: Vec<Value> = if istime {
        vec![Value::String("time".into()), Value::String("date".into())]
    } else {
        vec![Value::String("date".into())]
    };
    let divider_highlight_group = if istime {
        json!("time:divider")
    } else {
        Value::Null
    };
    vec![json!({
        "contents": contents,
        "highlight_groups": highlight_groups,
        "divider_highlight_group": divider_highlight_group,
    })]
}

// `fuzzy_time()` (py:40-) ports separately — large hour/minute string
// table + special-case handling for noon/midnight + nbsp + unicode
// text translation. Out of scope for this pass.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_returns_non_empty_contents() {
        let segments = date(&(), "%Y-%m-%d", false, None);
        assert_eq!(segments.len(), 1);
        let contents = segments[0]["contents"].as_str().unwrap();
        // YYYY-MM-DD is 10 chars
        assert_eq!(contents.len(), 10);
        // Should be parseable as a date-like string with two dashes
        assert_eq!(contents.matches('-').count(), 2);
    }

    #[test]
    fn date_istime_uses_time_highlight() {
        let segments = date(&(), "%H:%M:%S", true, None);
        let hl = segments[0]["highlight_groups"].as_array().unwrap();
        // py:33  ['time'] if istime else []) + ['date']
        assert_eq!(hl[0], "time");
        assert_eq!(hl[1], "date");
    }

    #[test]
    fn date_istime_emits_time_divider() {
        let segments = date(&(), "%H:%M", true, None);
        assert_eq!(segments[0]["divider_highlight_group"], "time:divider");
    }

    #[test]
    fn date_not_istime_omits_time_divider() {
        let segments = date(&(), "%Y", false, None);
        assert_eq!(segments[0]["divider_highlight_group"], Value::Null);
        assert_eq!(segments[0]["highlight_groups"].as_array().unwrap().len(), 1);
        assert_eq!(segments[0]["highlight_groups"][0], "date");
    }

    #[test]
    fn date_yyyymmdd_format() {
        let segments = date(&(), "%Y", false, None);
        let contents = segments[0]["contents"].as_str().unwrap();
        // Year should be 4 digits, all numeric
        assert_eq!(contents.len(), 4);
        assert!(contents.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn date_h_format_emits_two_digit_hour() {
        let segments = date(&(), "%H", false, None);
        let contents = segments[0]["contents"].as_str().unwrap();
        assert_eq!(contents.len(), 2);
        assert!(contents.chars().all(|c| c.is_ascii_digit()));
        let h: i32 = contents.parse().unwrap();
        assert!((0..24).contains(&h));
    }

    #[test]
    fn format_strftime_basic_unix_epoch() {
        // Unix epoch in UTC = 1970-01-01 00:00:00; in localtime varies.
        // Just verify the call succeeds and returns 10 chars for %Y-%m-%d.
        let s = format_strftime("%Y-%m-%d", 0);
        assert_eq!(s.len(), 10);
    }
}
