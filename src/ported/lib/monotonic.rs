// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/monotonic.py`.
//!
//! The upstream file is a 100-line tower of Python 2/3 + Windows / macOS
//! / Linux compatibility shims selecting the best available monotonic
//! clock source. The whole file boils down to one logical operation:
//! "give me a monotonic timestamp in seconds (float)."
//!
//! Rust's `std::time::Instant` is the canonical monotonic clock — it
//! is portable across Unix, macOS, and Windows, uses the OS's best
//! monotonic source automatically (`CLOCK_MONOTONIC` on Linux,
//! `mach_absolute_time` on macOS, `QueryPerformanceCounter` on
//! Windows), and requires zero conditional-compilation.
//!
//! Behaviour notes:
//!
//! - Upstream's Python 3.3+ `time.monotonic()` path corresponds to
//!   Rust's `Instant::now()`. Both are monotonic and don't go
//!   backwards.
//! - Upstream's `CLOCK_MONOTONIC_RAW` preference (when available)
//!   gives a NTP-unaffected counter. Rust does not expose `_RAW`
//!   directly on Linux (`Instant` uses `CLOCK_MONOTONIC`); the
//!   difference is negligible (~ms drift over hours of NTP slewing)
//!   for the segment-render use case powerline-status needs this for.

use std::sync::OnceLock;
use std::time::Instant;

/// Storage for the program-start instant. `monotonic()` returns the
/// number of seconds elapsed since this instant — matching the units
/// of every Python branch in `monotonic.py` (seconds as f64).
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Port of module-level binding `monotonic` from `powerline/lib/monotonic.py:14`,
/// `:17`, `:28`, `:64`, `:93`, or `:100` (one of seven `monotonic`
/// definitions depending on platform).
///
/// Returns the number of seconds since the first call to `monotonic()`
/// in this process — a monotonic, non-decreasing wall-clock-independent
/// timestamp suitable for measuring elapsed durations.
///
/// All Python branches return `float` (seconds, sub-millisecond
/// resolution); the Rust port returns `f64` for the same shape.
pub fn monotonic() -> f64 {
    let epoch = EPOCH.get_or_init(Instant::now);
    let elapsed = epoch.elapsed();
    elapsed.as_secs_f64()
}

/// Port of the darwin-branch helper `mach_timebase_info()` from
/// `powerline/lib/monotonic.py:56-59`.
///
/// Python wraps the macOS `mach_timebase_info` syscall to obtain the
/// (numer, denom) rational that converts `mach_absolute_time()` ticks
/// to nanoseconds: `ns = ticks * numer / denom`.
///
/// The Rust port calls `libc::mach_timebase_info` directly. Used as a
/// support helper for the darwin-specific `monotonic()` body at
/// py:64-65; the canonical `monotonic()` above uses
/// `std::time::Instant` which internally calls the same syscall on
/// macOS, so this helper is exposed for parity rather than to drive
/// the runtime clock.
#[cfg(target_os = "macos")]
#[allow(deprecated)]
pub fn mach_timebase_info() -> (u32, u32) {
    // py:57  timebase = mach_timebase_info_data_t()
    let mut info = libc::mach_timebase_info { numer: 0, denom: 0 };
    // py:58  _mach_timebase_info(ctypes.byref(timebase))
    unsafe {
        libc::mach_timebase_info(&mut info);
    }
    // py:59  return (timebase.numer, timebase.denom)
    (info.numer, info.denom)
}

/// Non-darwin fallback for `mach_timebase_info()` from
/// `powerline/lib/monotonic.py:56`. Python only defines this on darwin
/// (inside the `elif sys.platform == 'darwin':` branch at py:31); the
/// Rust port surfaces a stub on other platforms that mirrors Python's
/// "function not defined here" behaviour — calling it on Linux is a
/// programming error and the panic is the equivalent of Python's
/// `NameError`.
#[cfg(not(target_os = "macos"))]
pub fn mach_timebase_info() -> (u32, u32) {
    panic!("mach_timebase_info is darwin-only (py:56-59 lives inside the elif sys.platform == 'darwin' branch)")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `monotonic()` is non-decreasing across consecutive calls.
    #[test]
    fn monotonic_is_non_decreasing() {
        let a = monotonic();
        let b = monotonic();
        assert!(b >= a, "monotonic went backward: {} -> {}", a, b);
    }

    /// `monotonic()` advances over a measurable sleep interval.
    #[test]
    fn monotonic_advances() {
        let a = monotonic();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let b = monotonic();
        assert!(
            b - a >= 0.005,
            "monotonic did not advance ≥5ms after sleep(10ms): {} -> {}",
            a,
            b
        );
    }

    /// `mach_timebase_info()` returns a positive (numer, denom) pair on
    /// macOS — the rational scaling `mach_absolute_time` ticks to
    /// nanoseconds. Both fields must be > 0 for the ratio to be defined.
    #[cfg(target_os = "macos")]
    #[test]
    fn mach_timebase_info_returns_positive_rational() {
        let (numer, denom) = mach_timebase_info();
        assert!(numer > 0, "numer should be positive, got {numer}");
        assert!(denom > 0, "denom should be positive, got {denom}");
    }
}
