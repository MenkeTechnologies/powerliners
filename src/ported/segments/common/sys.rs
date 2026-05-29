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
    // py:16  def system_load(pl, format='{avg:.1f}', threshold_good=1, threshold_bad=2,
    // py:17  track_cpu_count=False, short=False):
    // py:18-45  docstring
    // py:46  global cpu_count
    // py:47  try:
    // py:48  cpu_num = cpu_count = _cpu_count() if cpu_count is None or track_cpu_count else cpu_count
    // py:49  except NotImplementedError:
    // py:50  pl.warn('Unable to get CPU count: method is not implemented')
    // py:51  return None
    let mut cpu_num = cpu_count.load(Ordering::Relaxed);
    if cpu_num == 0 || track_cpu_count {
        cpu_num = _cpu_count()?;
        cpu_count.store(cpu_num, Ordering::Relaxed);
    }
    let cpu_num = cpu_num as f64;

    // py:52  ret = []
    let mut ret: Vec<Value> = Vec::new();

    // py:53  for avg in os.getloadavg():
    let loads = _getloadavg()?;
    for avg in loads {
        // py:54  normalized = avg / cpu_num
        let normalized = avg / cpu_num;
        // py:55  if normalized < threshold_good:
        // py:56  gradient_level = 0
        // py:57  elif normalized < threshold_bad:
        // py:58  gradient_level = (normalized - threshold_good) * 100.0 / (threshold_bad - threshold_good)
        // py:59  else:
        // py:60  gradient_level = 100
        let gradient_level = if normalized < threshold_good {
            0.0
        } else if normalized < threshold_bad {
            (normalized - threshold_good) * 100.0 / (threshold_bad - threshold_good)
        } else {
            100.0
        };
        // py:61  ret.append({
        // py:62  'contents': format.format(avg=avg),
        // py:63  'highlight_groups': ['system_load_gradient', 'system_load'],
        // py:64  'divider_highlight_group': 'background:divider',
        // py:65  'gradient_level': gradient_level,
        // py:66  })
        let contents = render_load_format(format, avg);
        ret.push(json!({
            "contents": contents,
            "highlight_groups": ["system_load_gradient", "system_load"],
            "divider_highlight_group": "background:divider",
            "gradient_level": gradient_level,
        }));
        // py:68  if short:
        // py:69  return ret
        if short {
            return Some(ret);
        }
    }

    // py:71  ret[0]['contents'] += ' '
    // py:72  ret[1]['contents'] += ' '
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
    // py:73  return ret
    Some(ret)
}

/// Port of `CPULoadPercentSegment.render()` from
/// `powerline/segments/common/sys.py:92`.
///
/// Public alias `render` rather than `cpu_load_percent_render` to match
/// the Python method name (the only call-site is `class
/// CPULoadPercentSegment.render` per py:92).
pub fn render(cpu_percent: f64, format: &str) -> Vec<Value> {
    // py:79  class CPULoadPercentSegment(ThreadedSegment):
    // py:80  interval = 1
    // py:82  def update(self, old_cpu):
    // py:83  return psutil.cpu_percent(interval=None)
    // py:85  def run(self):
    // py:86  while not self.shutdown_event.is_set():
    // py:87  try:
    // py:88  self.update_value = psutil.cpu_percent(interval=self.interval)
    // py:89  except Exception as e:
    // py:90  self.exception('Exception while calculating cpu_percent: {0}', str(e))
    // py:92  def render(self, cpu_percent, format='{0:.0f}%', **kwargs):
    // py:93  return [{
    // py:94  'contents': format.format(cpu_percent),
    // py:95  'gradient_level': cpu_percent,
    // py:96  'highlight_groups': ['cpu_load_percent_gradient', 'cpu_load_percent'],
    // py:97  }]
    // Inline `{0[:[width].[prec]f]}` substitution to mirror
    // `format.format(cpu_percent)`. Supports the upstream defaults
    // (`{0:.0f}%`) and the user-config variants (`{0:2.0f}%`).
    let mut contents = String::new();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' && chars.peek() == Some(&'0') {
            chars.next(); // consume '0'
            let mut width: Option<usize> = None;
            let mut prec: Option<usize> = None;
            let mut is_float = false;
            if chars.peek() == Some(&':') {
                chars.next();
                // parse [width][.precision][f]
                let mut width_buf = String::new();
                while let Some(&p) = chars.peek() {
                    if p.is_ascii_digit() {
                        width_buf.push(p);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if !width_buf.is_empty() {
                    width = width_buf.parse().ok();
                }
                if chars.peek() == Some(&'.') {
                    chars.next();
                    let mut prec_buf = String::new();
                    while let Some(&p) = chars.peek() {
                        if p.is_ascii_digit() {
                            prec_buf.push(p);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    prec = prec_buf.parse().ok();
                }
                if chars.peek() == Some(&'f') {
                    chars.next();
                    is_float = true;
                }
            }
            // consume closing '}'
            if chars.peek() == Some(&'}') {
                chars.next();
            }
            let rendered = match (width, prec, is_float) {
                (Some(w), Some(p), true) => format!("{:>1$.2$}", cpu_percent, w, p),
                (None, Some(p), true) => format!("{:.1$}", cpu_percent, p),
                (Some(w), None, _) => format!("{:>1$}", cpu_percent, w),
                (None, None, false) => format!("{}", cpu_percent),
                (None, None, true) => format!("{}", cpu_percent),
                (Some(w), Some(p), false) => format!("{:>1$.2$}", cpu_percent, w, p),
                _ => format!("{}", cpu_percent),
            };
            contents.push_str(&rendered);
        } else {
            contents.push(c);
        }
    }
    vec![json!({
        "contents": contents,
        "gradient_level": cpu_percent,
        "highlight_groups": ["cpu_load_percent_gradient", "cpu_load_percent"],
    })]
}

/// Port of `_get_uptime()` from `powerline/segments/common/sys.py:133-147`.
pub fn _get_uptime() -> Option<u64> {
    // py:132  if os.path.exists('/proc/uptime'):
    // py:133  def _get_uptime():
    // py:134  with open('/proc/uptime', 'r') as f:
    // py:135  return int(float(f.readline().split()[0]))
    // py:136  elif 'psutil' in globals():
    // py:137  from time import time
    // py:139  if hasattr(psutil, 'boot_time'):
    // py:140  def _get_uptime():
    // py:141  return int(time() - psutil.boot_time())
    // py:142  else:
    // py:143  def _get_uptime():
    // py:144  return int(time() - psutil.BOOT_TIME)
    // py:145  else:
    // py:146  def _get_uptime():
    // py:147  raise NotImplementedError
    // py:132-135  Linux /proc/uptime path
    if let Ok(content) = std::fs::read_to_string("/proc/uptime") {
        if let Some(first) = content.split_whitespace().next() {
            if let Ok(uptime) = first.parse::<f64>() {
                return Some(uptime as u64);
            }
        }
    }
    // py:136-144  psutil.boot_time() equivalent — sysctl kern.boottime
    // on darwin/BSD. The Rust port reads it via libc::sysctlbyname when
    // /proc/uptime is unavailable (mirrors the psutil fallback chain).
    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "netbsd",
        target_os = "openbsd"
    ))]
    {
        if let Ok(name) = std::ffi::CString::new("kern.boottime") {
            let mut tv: [libc::time_t; 2] = [0, 0];
            let mut size = std::mem::size_of::<[libc::time_t; 2]>();
            // SAFETY: sysctlbyname writes into &mut tv with size bound;
            // struct timeval layout = (time_t sec, suseconds_t usec); on
            // 64-bit darwin both fields are 8 bytes so the [time_t; 2]
            // array overlays correctly for the sec field.
            let rc = unsafe {
                libc::sysctlbyname(
                    name.as_ptr(),
                    tv.as_mut_ptr() as *mut libc::c_void,
                    &mut size,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if rc == 0 && tv[0] > 0 {
                // SAFETY: time(NULL) is async-signal-safe.
                let now = unsafe { libc::time(std::ptr::null_mut()) };
                if now > tv[0] {
                    return Some((now - tv[0]) as u64);
                }
            }
        }
    }
    None
}

/// Port of `uptime()` from `powerline/segments/common/sys.py:151`.
pub fn uptime(
    _pl: &(),
    days_format: &str,
    hours_format: &str,
    minutes_format: &str,
    seconds_format: &str,
    shorten_len: usize,
) -> Option<String> {
    // py:150  @add_divider_highlight_group('background:divider')
    // py:151  def uptime(pl, days_format='{days:d}d', hours_format=' {hours:d}h', minutes_format=' {minutes:02d}m',
    // py:152  seconds_format=' {seconds:02d}s', shorten_len=3):
    // py:153-167  docstring
    // py:168  try:
    // py:169  seconds = _get_uptime()
    // py:170  except NotImplementedError:
    // py:171  pl.warn('Unable to get uptime. You should install psutil module')
    // py:172  return None
    let total_seconds = _get_uptime()?;
    // py:173  minutes, seconds = divmod(seconds, 60)
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    // py:174  hours, minutes = divmod(minutes, 60)
    let hours = minutes / 60;
    let minutes = minutes % 60;
    // py:175  days, hours = divmod(hours, 24)
    let days = hours / 24;
    let hours = hours % 24;
    // py:176  time_formatted = list(filter(None, [
    // py:177  days_format.format(days=days) if days_format else None,
    // py:178  hours_format.format(hours=hours) if hours_format else None,
    // py:179  minutes_format.format(minutes=minutes) if minutes_format else None,
    // py:180  seconds_format.format(seconds=seconds) if seconds_format else None,
    // py:181  ]))
    let parts: Vec<(u64, &str)> = vec![
        (days, days_format),
        (hours, hours_format),
        (minutes, minutes_format),
        (seconds, seconds_format),
    ];
    // py:182  first_non_zero = next((i for i, x in enumerate([days, hours, minutes, seconds]) if x != 0))
    let first_non_zero = parts.iter().position(|(v, _)| *v != 0)?;
    // py:183  time_formatted = time_formatted[first_non_zero:first_non_zero + shorten_len]
    let end = (first_non_zero + shorten_len).min(parts.len());
    // py:184  return ''.join(time_formatted).strip()
    let formatted: String = parts[first_non_zero..end]
        .iter()
        .map(|(v, fmt)| {
            // Inline Python format substitution: handles {name},
            // {name:d}, {name:0Nd} for any name. Mirrors
            // `fmt.format(days=days)` etc. at py:177-180.
            let mut out = String::with_capacity(fmt.len());
            let mut chars = fmt.chars().peekable();
            while let Some(c) = chars.next() {
                if c != '{' {
                    out.push(c);
                    continue;
                }
                let mut name = String::new();
                while let Some(&p) = chars.peek() {
                    if p == '}' || p == ':' {
                        break;
                    }
                    name.push(p);
                    chars.next();
                }
                let mut spec = String::new();
                if chars.peek() == Some(&':') {
                    chars.next();
                    while let Some(&p) = chars.peek() {
                        if p == '}' {
                            break;
                        }
                        spec.push(p);
                        chars.next();
                    }
                }
                if chars.peek() == Some(&'}') {
                    chars.next();
                }
                // Always substitute with the current `v` regardless of
                // name — every fmt only references one of d/h/m/s.
                let render = if spec == "d" || spec.is_empty() {
                    v.to_string()
                } else if let Some(zero_pad) =
                    spec.strip_prefix('0').and_then(|s| s.strip_suffix('d'))
                {
                    let width: usize = zero_pad.parse().unwrap_or(0);
                    format!("{:0width$}", v, width = width)
                } else {
                    v.to_string()
                };
                let _ = name;
                out.push_str(&render);
            }
            out
        })
        .collect();
    Some(formatted.trim().to_string())
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
