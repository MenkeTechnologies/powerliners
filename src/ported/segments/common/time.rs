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
    // py:5  def date(pl, format='%Y-%m-%d', istime=False, timezone=None):
    // py:6-19  docstring
    // py:21  try:
    // py:22  tz = datetime.strptime(timezone, '%z').tzinfo if timezone else None
    // py:23  except ValueError:
    // py:24  tz = None
    // py:26  nw = datetime.now(tz)
    let ts = unsafe { libc::time(std::ptr::null_mut()) };
    // py:28  try:
    // py:29  contents = nw.strftime(format)
    // py:30  except UnicodeEncodeError:
    // py:31  contents = nw.strftime(format.encode('utf-8')).decode('utf-8')
    let contents = format_strftime(format, ts);
    // py:33  return [{
    // py:34  'contents': contents,
    // py:35  'highlight_groups': (['time'] if istime else []) + ['date'],
    // py:36  'divider_highlight_group': 'time:divider' if istime else None,
    // py:37  }]
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

/// Port of `UNICODE_TEXT_TRANSLATION` from
/// `powerline/segments/common/time.py:40-43`.
///
/// Maps ASCII apostrophe (`'`) → right single quotation mark
/// (U+2019) and ASCII hyphen-minus (`-`) → hyphen (U+2010).
pub fn unicode_text_translate(s: &str) -> String {
    // py:41-42
    s.replace('\'', "\u{2019}").replace('-', "\u{2010}")
}

/// Default `hour_str` list per `fuzzy_time` keyword default at
/// `powerline/segments/common/time.py:46-47`.
///
/// 12 entries starting with "twelve" (midnight) per py:71-72.
pub fn fuzzy_time_default_hour_str() -> Vec<&'static str> {
    vec![
        // py:46-47
        "twelve", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
        "eleven",
    ]
}

/// Default `minute_str` mapping per `fuzzy_time` keyword default at
/// `powerline/segments/common/time.py:47-50`.
///
/// Maps 5-minute increments (0..=55 stepped by 5) to format strings
/// containing `{hour_str}` interpolation. Note 30 splits the hour:
/// minutes 0..=30 use "past <hour>", 35..=55 use "to <hour+1>".
pub fn fuzzy_time_default_minute_str() -> std::collections::HashMap<u32, &'static str> {
    let mut m = std::collections::HashMap::new();
    // py:48-50
    m.insert(0, "{hour_str} o'clock");
    m.insert(5, "five past {hour_str}");
    m.insert(10, "ten past {hour_str}");
    m.insert(15, "quarter past {hour_str}");
    m.insert(20, "twenty past {hour_str}");
    m.insert(25, "twenty-five past {hour_str}");
    m.insert(30, "half past {hour_str}");
    m.insert(35, "twenty-five to {hour_str}");
    m.insert(40, "twenty to {hour_str}");
    m.insert(45, "quarter to {hour_str}");
    m.insert(50, "ten to {hour_str}");
    m.insert(55, "five to {hour_str}");
    m
}

/// Default `special_case_str` mapping per `fuzzy_time` keyword
/// default at `powerline/segments/common/time.py:51-58`.
///
/// Keys are `(hour, minute)` pairs; values are display strings.
/// Used for midnight/noon and the surrounding minute boundaries.
pub fn fuzzy_time_default_special_cases() -> std::collections::HashMap<(u32, u32), &'static str> {
    let mut m = std::collections::HashMap::new();
    // py:51-58
    m.insert((23, 58), "round about midnight");
    m.insert((23, 59), "round about midnight");
    m.insert((0, 0), "midnight");
    m.insert((0, 1), "round about midnight");
    m.insert((0, 2), "round about midnight");
    m.insert((12, 0), "noon");
    m
}

/// Port of `fuzzy_time()` value-computation core from
/// `powerline/segments/common/time.py:101-122`.
///
/// `hour` and `minute` are the resolved 24-hour clock values; the
/// timezone resolution at py:83-87 is the caller's responsibility.
///
/// Returns the fuzzy-time string ("quarter past six", "noon", etc.).
/// Applies unicode_text_translate when `unicode_text=true` per
/// py:95-96/119-120.
pub fn fuzzy_time_compute(
    hour: u32,
    minute: u32,
    hour_str: &[&str],
    minute_str: &std::collections::HashMap<u32, &str>,
    special_cases: &std::collections::HashMap<(u32, u32), &str>,
    unicode_text: bool,
) -> String {
    // py:90  try:
    // py:91  # We don't want to enforce a special type of spaces/ alignment in the input
    // py:92  from ast import literal_eval
    // py:93  special_case_str = {literal_eval(x):special_case_str[x] for x in special_case_str}
    // py:94  result = special_case_str[(now.hour, now.minute)]
    // py:95  if unicode_text:
    // py:96  result = result.translate(UNICODE_TEXT_TRANSLATION)
    // py:97  return result
    // py:98  except KeyError:
    // py:99  pass
    if let Some(s) = special_cases.get(&(hour, minute)) {
        if unicode_text {
            return unicode_text_translate(s);
        }
        return s.to_string();
    }
    // py:101  hour = now.hour
    // py:102  if now.minute >= 32:
    // py:103  hour = hour + 1
    let mut h = hour;
    if minute >= 32 {
        h += 1;
    }
    // py:104  hour = hour % len(hour_str)
    let h = (h as usize) % hour_str.len();
    // py:106  min_dis = 100
    // py:107  min_pos = 0
    let mut min_dis: u32 = 100;
    let mut min_pos: u32 = 0;
    let mut keys: Vec<u32> = minute_str.keys().copied().collect();
    keys.sort();
    // py:109  for mn in minute_str:
    // py:110  mn = int(mn)
    for &mn in &keys {
        // py:111  if now.minute >= mn and now.minute - mn < min_dis:
        // py:112  min_dis = now.minute - mn
        // py:113  min_pos = mn
        // py:114  elif now.minute < mn and mn - now.minute < min_dis:
        // py:115  min_dis = mn - now.minute
        // py:116  min_pos = mn
        let dis = if minute >= mn {
            minute - mn
        } else {
            mn - minute
        };
        if dis < min_dis {
            min_dis = dis;
            min_pos = mn;
        }
    }
    // py:117  result = minute_str[str(min_pos)].format(hour_str=hour_str[hour])
    let template = minute_str.get(&min_pos).copied().unwrap_or("");
    let result = template.replace("{hour_str}", hour_str[h]);
    // py:119  if unicode_text:
    // py:120  result = result.translate(UNICODE_TEXT_TRANSLATION)
    // py:122  return result
    if unicode_text {
        unicode_text_translate(&result)
    } else {
        result
    }
}

/// Port of `fuzzy_time()` from
/// `powerline/segments/common/time.py:46-122`.
///
/// Reads the current local time and dispatches to `fuzzy_time_compute`.
/// The `_format` parameter is preserved at py:62-63 (unused in Python).
pub fn fuzzy_time(
    _format: Option<&str>,
    unicode_text: bool,
    _timezone: Option<&str>,
    hour_str: Option<&[&str]>,
    minute_str: Option<&std::collections::HashMap<u32, &str>>,
    special_cases: Option<&std::collections::HashMap<(u32, u32), &str>>,
) -> String {
    // py:46  def fuzzy_time(pl, format=None, unicode_text=False, timezone=None, hour_str=[...], minute_str={...}, special_case_str={...}):
    // py:60-81  docstring
    // py:83  try:
    // py:84  tz = datetime.strptime(timezone, '%z').tzinfo if timezone else None
    // py:85  except ValueError:
    // py:86  tz = None
    // py:88  now = datetime.now(tz)
    let ts = unsafe { libc::time(std::ptr::null_mut()) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let p = unsafe { libc::localtime_r(&ts, &mut tm) };
    if p.is_null() {
        return String::new();
    }
    let hour = tm.tm_hour as u32;
    let minute = tm.tm_min as u32;

    // Resolve the optional keyword args to defaults per py:46-58
    let default_hour = fuzzy_time_default_hour_str();
    let h: &[&str] = hour_str.unwrap_or(&default_hour);

    let default_minute = fuzzy_time_default_minute_str();
    let m_ref: &std::collections::HashMap<u32, &str> = minute_str.unwrap_or(&default_minute);

    let default_special = fuzzy_time_default_special_cases();
    let s_ref: &std::collections::HashMap<(u32, u32), &str> =
        special_cases.unwrap_or(&default_special);

    fuzzy_time_compute(hour, minute, h, m_ref, s_ref, unicode_text)
}

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

    #[test]
    fn unicode_text_translate_replaces_apostrophe_and_hyphen() {
        // py:40-43
        assert_eq!(unicode_text_translate("o'clock"), "o\u{2019}clock");
        assert_eq!(unicode_text_translate("twenty-five"), "twenty\u{2010}five");
    }

    #[test]
    fn unicode_text_translate_no_op_on_plain_text() {
        assert_eq!(unicode_text_translate("noon"), "noon");
    }

    #[test]
    fn fuzzy_time_default_hour_str_has_12_entries() {
        // py:46-47
        let h = fuzzy_time_default_hour_str();
        assert_eq!(h.len(), 12);
        assert_eq!(h[0], "twelve");
        assert_eq!(h[11], "eleven");
    }

    #[test]
    fn fuzzy_time_default_minute_str_has_12_entries() {
        // py:48-50  0..=55 stepped by 5
        let m = fuzzy_time_default_minute_str();
        assert_eq!(m.len(), 12);
        for key in [0, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50, 55] {
            assert!(m.contains_key(&key));
        }
    }

    #[test]
    fn fuzzy_time_default_minute_str_quarter_past() {
        let m = fuzzy_time_default_minute_str();
        assert_eq!(*m.get(&15).unwrap(), "quarter past {hour_str}");
    }

    #[test]
    fn fuzzy_time_default_special_cases_includes_noon_and_midnight() {
        // py:51-58
        let s = fuzzy_time_default_special_cases();
        assert_eq!(*s.get(&(0, 0)).unwrap(), "midnight");
        assert_eq!(*s.get(&(12, 0)).unwrap(), "noon");
        assert_eq!(*s.get(&(23, 58)).unwrap(), "round about midnight");
    }

    #[test]
    fn fuzzy_time_compute_quarter_past_six() {
        // 6:15 → "quarter past six"
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(
            fuzzy_time_compute(6, 15, &h, &m, &s, false),
            "quarter past six"
        );
    }

    #[test]
    fn fuzzy_time_compute_o_clock() {
        // 3:00 → "three o'clock"
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(fuzzy_time_compute(3, 0, &h, &m, &s, false), "three o'clock");
    }

    #[test]
    fn fuzzy_time_compute_special_case_noon() {
        // py:93-99
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(fuzzy_time_compute(12, 0, &h, &m, &s, false), "noon");
    }

    #[test]
    fn fuzzy_time_compute_special_case_midnight() {
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(fuzzy_time_compute(0, 0, &h, &m, &s, false), "midnight");
    }

    #[test]
    fn fuzzy_time_compute_special_case_round_about_midnight() {
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(
            fuzzy_time_compute(23, 59, &h, &m, &s, false),
            "round about midnight"
        );
    }

    #[test]
    fn fuzzy_time_compute_rounds_up_after_32() {
        // py:101-104  if minute >= 32: hour += 1
        // 5:40 → "twenty to six"
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(
            fuzzy_time_compute(5, 40, &h, &m, &s, false),
            "twenty to six"
        );
    }

    #[test]
    fn fuzzy_time_compute_unicode_text_translates_apostrophe() {
        // py:119-120
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        let r = fuzzy_time_compute(3, 0, &h, &m, &s, true);
        assert_eq!(r, "three o\u{2019}clock");
    }

    #[test]
    fn fuzzy_time_compute_unicode_text_translates_hyphen_in_special() {
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let mut s = std::collections::HashMap::new();
        s.insert((10, 10), "ten-thirty");
        let r = fuzzy_time_compute(10, 10, &h, &m, &s, true);
        assert!(r.contains('\u{2010}'));
    }

    #[test]
    fn fuzzy_time_compute_modulo_24_hour_wraps_to_12_label() {
        // 18:00 → hour 18 % 12 = 6 → "six o'clock"
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(fuzzy_time_compute(18, 0, &h, &m, &s, false), "six o'clock");
    }

    #[test]
    fn fuzzy_time_compute_closest_minute_for_unaligned_input() {
        // 6:13 → closest is 15 (dist=2) vs 10 (dist=3) → "quarter past six"
        let h = fuzzy_time_default_hour_str();
        let m = fuzzy_time_default_minute_str();
        let s = fuzzy_time_default_special_cases();
        assert_eq!(
            fuzzy_time_compute(6, 13, &h, &m, &s, false),
            "quarter past six"
        );
    }

    #[test]
    fn fuzzy_time_smoke_test_uses_localtime() {
        // Verify the wrapper succeeds and returns non-empty string.
        let r = fuzzy_time(None, false, None, None, None, None);
        assert!(!r.is_empty());
    }
}
