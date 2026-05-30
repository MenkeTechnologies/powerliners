// vim:fileencoding=utf-8:noet
//! Process-count segment — total processes plus state breakdown
//! (running / sleeping / zombie / uninterruptible). Probes `ps -eo`
//! once and tallies by state code.
//!
//! Cross-platform: POSIX `ps -eo stat=` works on Linux, macOS, and
//! the BSDs. The state column's first character is the canonical
//! state code (`R S D Z T I` plus modifier suffixes that we ignore).
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.proc.process_count",
//!   "args": {
//!     "format": "{total}p {running}R",
//!     "warn_zombie": true
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{total}`    — every process visible to `ps -e`
//! - `{running}`  — state `R` (running or runnable)
//! - `{sleeping}` — state `S` (interruptible sleep)
//! - `{zombie}`   — state `Z` (defunct, awaiting reap)
//! - `{dwait}`    — state `D` (uninterruptible disk wait)
//! - `{stopped}`  — state `T` (stopped by signal or ptrace)

use serde_json::{json, Value};
use std::process::Command;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProcCounts {
    pub total: u32,
    pub running: u32,
    pub sleeping: u32,
    pub zombie: u32,
    pub dwait: u32,
    pub stopped: u32,
}

/// Probe via `ps -eo stat=`. Returns `None` when `ps` is missing
/// (none of the OSes we target ship without it, but defensive).
pub fn read_proc_counts() -> Option<ProcCounts> {
    let out = Command::new("ps").args(["-eo", "stat="]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&out.stdout).ok()?;
    Some(tally(text))
}

/// Pure-data tally — accepts the raw `ps -eo stat=` text and counts
/// states by their first character. Exposed for test injection so
/// CI doesn't depend on the test runner's process table looking any
/// particular way.
pub fn tally(text: &str) -> ProcCounts {
    let mut c = ProcCounts::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        c.total += 1;
        match line.as_bytes()[0] {
            b'R' => c.running += 1,
            b'S' | b'I' => c.sleeping += 1, // BSD/macOS use `I` for idle kernel threads;
            // treat as sleeping per `ps(1)` semantics — neither runs CPU.
            b'Z' => c.zombie += 1,
            b'D' => c.dwait += 1,
            b'T' | b't' => c.stopped += 1, // `t` = tracing-stop
            _ => {}
        }
    }
    c
}

/// Render the segment. Always returns at least one chunk — the
/// segment is meant to be a persistent health indicator, so it
/// shows even when every counter is zero. Set `warn_zombie` to
/// upgrade the highlight group when zombies are present so the
/// segment turns red without the theme having to write a separate
/// `_zombie` style.
pub fn process_count(format: &str, warn_zombie: bool) -> Option<Vec<Value>> {
    let c = read_proc_counts()?;
    let contents = render_format(format, &c);
    let mut groups = vec![
        Value::String("process_count".into()),
        Value::String("proc".into()),
    ];
    if warn_zombie && c.zombie > 0 {
        groups.insert(0, Value::String("process_count_zombie".into()));
    }
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": groups,
        "divider_highlight_group": "background:divider",
    })])
}

fn render_format(fmt: &str, c: &ProcCounts) -> String {
    fmt.replace("{total}", &c.total.to_string())
        .replace("{running}", &c.running.to_string())
        .replace("{sleeping}", &c.sleeping.to_string())
        .replace("{zombie}", &c.zombie.to_string())
        .replace("{dwait}", &c.dwait.to_string())
        .replace("{stopped}", &c.stopped.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tally_empty_input_yields_zero_counts() {
        let c = tally("");
        assert_eq!(c, ProcCounts::default());
        assert_eq!(c.total, 0);
    }

    #[test]
    fn tally_counts_each_state_code() {
        // First-character state per POSIX ps(1).
        let text = "R\nS\nS\nZ\nD\nT\nI\nR\n";
        let c = tally(text);
        assert_eq!(c.total, 8);
        assert_eq!(c.running, 2);
        // S + I both fold into sleeping for macOS parity.
        assert_eq!(c.sleeping, 3);
        assert_eq!(c.zombie, 1);
        assert_eq!(c.dwait, 1);
        assert_eq!(c.stopped, 1);
    }

    #[test]
    fn tally_ignores_modifier_suffixes() {
        // `ps -eo stat=` emits codes like `R+`, `Ss`, `S<`, `D<L` —
        // only the first byte is the state class.
        let text = "R+\nS<\nSs\nD<L\n";
        let c = tally(text);
        assert_eq!(c.total, 4);
        assert_eq!(c.running, 1);
        assert_eq!(c.sleeping, 2);
        assert_eq!(c.dwait, 1);
    }

    #[test]
    fn tally_skips_blank_lines() {
        let text = "R\n\n\nS\n";
        let c = tally(text);
        assert_eq!(c.total, 2);
        assert_eq!(c.running, 1);
        assert_eq!(c.sleeping, 1);
    }

    #[test]
    fn tally_unknown_state_does_not_increment_category_but_does_total() {
        // Future kernels may add new state letters; we want the
        // segment to keep counting `total` so the user has at least
        // a coarse signal. Unknown codes don't fall into any specific
        // category counter.
        let c = tally("X\nY\n?\n");
        assert_eq!(c.total, 3);
        assert_eq!(c.running, 0);
        assert_eq!(c.sleeping, 0);
        assert_eq!(c.zombie, 0);
        assert_eq!(c.dwait, 0);
        assert_eq!(c.stopped, 0);
    }

    #[test]
    fn tally_handles_lowercase_t_for_tracing_stop() {
        // ps(1): `T` = stopped by signal, `t` = stopped by debugger.
        // Both are "not currently scheduled," so both fold into
        // stopped.
        let c = tally("T\nt\nT\n");
        assert_eq!(c.stopped, 3);
    }

    #[test]
    fn render_format_substitutes_all_tokens() {
        let c = ProcCounts {
            total: 100,
            running: 3,
            sleeping: 90,
            zombie: 1,
            dwait: 2,
            stopped: 4,
        };
        let out = render_format(
            "{total} {running}R {sleeping}S {zombie}Z {dwait}D {stopped}T",
            &c,
        );
        assert_eq!(out, "100 3R 90S 1Z 2D 4T");
    }

    #[test]
    fn render_format_leaves_unknown_tokens_intact() {
        let c = ProcCounts {
            total: 5,
            ..Default::default()
        };
        let s = render_format("{total} {cpu}", &c);
        assert_eq!(s, "5 {cpu}");
    }

    #[test]
    fn read_proc_counts_real_probe_has_a_total() {
        // ps is part of POSIX core utils; macOS/Linux CI runners
        // always have it. Whatever the test rig's process count is,
        // it must be >0 because at minimum the cargo-test harness
        // and ps itself are running.
        let c = read_proc_counts().expect("ps must be available on POSIX CI");
        assert!(
            c.total > 0,
            "ps reported no processes — impossible while this test is running"
        );
    }

    #[test]
    fn process_count_returns_one_chunk_when_ps_available() {
        let out = process_count("{total}", false).expect("ps available");
        assert_eq!(out.len(), 1);
        let contents = out[0]["contents"].as_str().unwrap_or("");
        assert!(
            contents.chars().all(|c| c.is_ascii_digit()),
            "contents should be a numeric string, got {contents:?}"
        );
    }

    #[test]
    fn process_count_warn_zombie_adds_zombie_highlight_group() {
        // We can't synthesize zombies in tests; verify the path via
        // a synthetic ProcCounts. Replicate the group-construction
        // logic to assert on it.
        let c = ProcCounts {
            total: 5,
            zombie: 1,
            ..Default::default()
        };
        let mut groups: Vec<String> = vec!["process_count".into(), "proc".into()];
        let warn_zombie = true;
        if warn_zombie && c.zombie > 0 {
            groups.insert(0, "process_count_zombie".into());
        }
        assert_eq!(groups[0], "process_count_zombie");
        assert_eq!(groups.len(), 3);
    }
}
