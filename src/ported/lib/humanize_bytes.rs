// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/humanize_bytes.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// (No Rust analogue.)

// from math import log                              // py:4
// Rust uses f64::log(self, base) — see usage below.

/// Port of module constant `unit_list` from `powerline/lib/humanize_bytes.py:7`.
///
/// `unit_list = tuple(zip(['', 'k', 'M', 'G', 'T', 'P'], [0, 0, 1, 2, 2, 2]))`
///
/// Each entry is `(unit_prefix, decimals_to_display)`.
#[allow(non_upper_case_globals)]
pub const unit_list: [(&str, usize); 6] = [
    // py:7
    ("", 0),
    ("k", 0),
    ("M", 1),
    ("G", 2),
    ("T", 2),
    ("P", 2),
];

/// Port of `humanize_bytes()` from `powerline/lib/humanize_bytes.py:10`.
///
/// Return a human friendly byte representation.
///
/// Modified version from <http://stackoverflow.com/questions/1094841>
pub fn humanize_bytes(num: f64, suffix: &str, si_prefix: bool) -> String {
    // py-default: suffix='B', si_prefix=False — call sites must pass these.
    if num == 0.0 {
        // py:15
        return format!("0 {}", suffix); // py:16
    }
    let div: f64 = if si_prefix { 1000.0 } else { 1024.0 }; // py:17
                                                            // py:18  exponent = min(int(log(num, div)) if num else 0, len(unit_list) - 1)
    let exponent: usize = {
        let raw = if num != 0.0 { num.log(div) as i64 } else { 0 };
        let max = (unit_list.len() as i64) - 1;
        raw.min(max).max(0) as usize
    };
    let quotient: f64 = num / div.powi(exponent as i32); // py:19
    let (unit, decimals) = unit_list[exponent]; // py:20  unit, decimals = unit_list[exponent]
    let mut unit: String = unit.to_string(); // shadow to owned form so py:22 can reassign
    if !unit.is_empty() && !si_prefix {
        // py:21  if unit and not si_prefix:
        unit = format!("{}i", unit.to_uppercase()); // py:22  unit = unit.upper() + 'i'
    }
    // py:23-25  return ('{quotient:.{decimals}f} {unit}{suffix}'
    //              .format(decimals=decimals)
    //              .format(quotient=quotient, unit=unit, suffix=suffix))
    format!("{:.*} {}{}", decimals, quotient, unit, suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors the upstream `unit_list` shape: 6 entries, prefixes in order.
    #[test]
    fn unit_list_matches_upstream_shape() {
        assert_eq!(unit_list.len(), 6);
        assert_eq!(unit_list[0], ("", 0));
        assert_eq!(unit_list[1], ("k", 0));
        assert_eq!(unit_list[2], ("M", 1));
        assert_eq!(unit_list[3], ("G", 2));
        assert_eq!(unit_list[4], ("T", 2));
        assert_eq!(unit_list[5], ("P", 2));
    }

    /// Zero-byte case: returns `'0 ' + suffix` per py:15-16.
    #[test]
    fn zero_returns_suffix_only() {
        assert_eq!(humanize_bytes(0.0, "B", false), "0 B");
        assert_eq!(humanize_bytes(0.0, "iB", true), "0 iB");
    }

    /// Binary-prefix path: 1024 → "1 KiB", 1024^2 → "1.0 MiB", 1024^3 → "1.00 GiB".
    /// Matches upstream Python output for the same inputs at decimals=0/1/2.
    #[test]
    fn binary_prefix_matches_upstream() {
        assert_eq!(humanize_bytes(1024.0, "B", false), "1 KiB");
        assert_eq!(humanize_bytes(1024.0 * 1024.0, "B", false), "1.0 MiB");
        assert_eq!(humanize_bytes(1024.0_f64.powi(3), "B", false), "1.00 GiB");
    }

    /// SI-prefix path: 1000 → "1 kB", 1_000_000 → "1.0 MB".
    #[test]
    fn si_prefix_matches_upstream() {
        assert_eq!(humanize_bytes(1000.0, "B", true), "1 kB");
        assert_eq!(humanize_bytes(1_000_000.0, "B", true), "1.0 MB");
    }
}
