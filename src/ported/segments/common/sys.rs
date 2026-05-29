// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/sys.py`.
//!
//! System-stat segments: 1/5/15-min load average + CPU percent +
//! uptime. Load average uses `libc::getloadavg`; CPU percent stub
//! requires a Rust analog of psutil.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from multiprocessing import cpu_count as _cpu_count                                     // py:6
// from powerline.lib.threaded import ThreadedSegment                                      // py:8
// from powerline.lib import add_divider_highlight_group                                   // py:9
// from powerline.segments import with_docstring                                           // py:10

use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Port of module-level binding `cpu_count` from
/// `powerline/segments/common/sys.py:13`.
///
/// Python: `cpu_count = None` — module-level cache populated on first
/// `system_load` call. Rust port uses an atomic to model the lazy fill
/// without a Mutex.
#[allow(non_upper_case_globals)]
pub static cpu_count: AtomicUsize = AtomicUsize::new(0);

/// Returns the CPU count, caching the result on first call.
fn _cpu_count() -> Option<usize> {
    std::thread::available_parallelism().ok().map(|n| n.get())
}

/// Returns the (1-min, 5-min, 15-min) load averages.
///
/// Uses `libc::getloadavg` on Unix; returns `None` on non-Unix or when
/// the syscall fails (e.g. sandboxed environments).
fn _getloadavg() -> Option<[f64; 3]> {
    #[cfg(unix)]
    {
        let mut buf = [0.0_f64; 3];
        // SAFETY: libc::getloadavg writes up to 3 doubles into buf.
        let n = unsafe { libc::getloadavg(buf.as_mut_ptr(), 3) };
        if n == 3 {
            Some(buf)
        } else {
            None
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Port of `system_load()` from `powerline/segments/common/sys.py:16`.
///
/// Return system load average. Highlights using `system_load_good`,
/// `system_load_bad` and `system_load_ugly` highlight groups, depending
/// on the thresholds passed.
///
/// :param format: format string, receives `avg` argument.
/// :param threshold_good: threshold for gradient level 0.
/// :param threshold_bad: threshold for gradient level 100.
/// :param track_cpu_count: continuously poll the system to detect
///     changes in CPU count.
/// :param short: if true only the 1-min average is returned.
pub fn system_load(
    _pl: &(),
    format: &str,
    threshold_good: f64,
    threshold_bad: f64,
    track_cpu_count: bool,
    short: bool,
) -> Option<Vec<Value>> {
    // py:46-49  cpu_num = cpu_count = _cpu_count() if cpu_count is None or track_cpu_count else cpu_count
    let mut cpu_num = cpu_count.load(Ordering::Relaxed);
    if cpu_num == 0 || track_cpu_count {
        cpu_num = _cpu_count()?;
        cpu_count.store(cpu_num, Ordering::Relaxed);
    }
    let cpu_num = cpu_num as f64;

    // py:51  ret = []
    let mut ret: Vec<Value> = Vec::new();

    // py:52-66  for avg in os.getloadavg():
    let loads = _getloadavg()?;
    for avg in loads {
        let normalized = avg / cpu_num;
        // py:54-59  gradient_level dispatch
        let gradient_level = if normalized < threshold_good {
            0.0
        } else if normalized < threshold_bad {
            (normalized - threshold_good) * 100.0 / (threshold_bad - threshold_good)
        } else {
            100.0
        };
        // py:60-66  segment dict (with format string substitution)
        let contents = render_load_format(format, avg);
        ret.push(json!({
            "contents": contents,
            "highlight_groups": ["system_load_gradient", "system_load"],
            "divider_highlight_group": "background:divider",
            "gradient_level": gradient_level,
        }));
        // py:68-69  if short: return ret
        if short {
            return Some(ret);
        }
    }

    // py:71-72  ret[0]['contents'] += ' '; ret[1]['contents'] += ' '
    // (Python appends a trailing space to the first two of the three
    //  load averages so they line up alongside dividers.)
    for r in ret.iter_mut().take(2) {
        if let Some(v) = r
            .get_mut("contents")
            .and_then(|v| v.as_str().map(String::from))
        {
            r.as_object_mut()
                .unwrap()
                .insert("contents".to_string(), Value::String(format!("{} ", v)));
        }
    }
    Some(ret)
}

/// Helper: render the Python format string `{avg:.Nf}` against a load
/// value.
///
/// Supports only the `{avg:.Nf}` shape upstream uses by default
/// (`{avg:.1f}` etc.); other format spec types fall through to a
/// simple `{}` rendering.
fn render_load_format(format: &str, avg: f64) -> String {
    if let Some(rest) = format.strip_prefix("{avg:.") {
        if let Some(idx) = rest.find("f}") {
            if let Ok(n) = rest[..idx].parse::<usize>() {
                return format!("{:.*}", n, avg);
            }
        }
    }
    format.replace("{avg}", &format!("{}", avg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_load_format_one_decimal() {
        assert_eq!(render_load_format("{avg:.1f}", 1.234), "1.2");
    }

    #[test]
    fn render_load_format_two_decimal() {
        assert_eq!(render_load_format("{avg:.2f}", 1.2345), "1.23");
    }

    #[test]
    fn render_load_format_zero_decimal() {
        assert_eq!(render_load_format("{avg:.0f}", 3.7), "4");
    }

    #[test]
    fn render_load_format_plain_avg() {
        assert!(render_load_format("{avg}", 1.5).starts_with("1.5"));
    }

    #[test]
    fn cpu_count_helper_returns_at_least_one() {
        let n = _cpu_count().unwrap();
        assert!(n >= 1);
    }

    /// On Unix, getloadavg should return three non-negative numbers.
    #[cfg(unix)]
    #[test]
    fn getloadavg_returns_three_values_on_unix() {
        let loads = _getloadavg().unwrap();
        for v in loads {
            assert!(v >= 0.0);
        }
    }

    /// system_load returns a Vec of length 3 (or 1 in short mode) on
    /// any platform with getloadavg.
    #[test]
    fn system_load_short_returns_one_entry() {
        if let Some(result) = system_load(&(), "{avg:.1f}", 1.0, 2.0, false, true) {
            assert_eq!(result.len(), 1);
        }
    }

    #[test]
    fn system_load_full_returns_three_entries_or_none() {
        // None when not on Unix or when getloadavg fails.
        if let Some(result) = system_load(&(), "{avg:.1f}", 1.0, 2.0, false, false) {
            assert_eq!(result.len(), 3);
            // First two should end with trailing space (py:71-72)
            assert!(result[0]["contents"].as_str().unwrap().ends_with(' '));
            assert!(result[1]["contents"].as_str().unwrap().ends_with(' '));
        }
    }

    #[test]
    fn system_load_gradient_level_thresholds() {
        // With thresholds (-1.0, 0.0): any non-negative load lands above
        // both bounds → gradient_level should be 100.
        if let Some(result) = system_load(&(), "{avg:.1f}", -1.0, 0.0, false, true) {
            assert_eq!(result[0]["gradient_level"], 100.0);
        }
        // With thresholds (1e9, 2e9): any sane load is below threshold_good →
        // gradient_level should be 0.
        if let Some(result) = system_load(&(), "{avg:.1f}", 1e9, 2e9, false, true) {
            assert_eq!(result[0]["gradient_level"], 0.0);
        }
    }
}
