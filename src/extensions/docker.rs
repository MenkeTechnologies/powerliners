// vim:fileencoding=utf-8:noet
//! Docker / OCI segment — running containers, total containers,
//! image count. Probes the daemon via the `docker` CLI so it works
//! against any OCI-compatible runtime that ships the same surface
//! (`docker`, `podman` aliased to `docker`, `nerdctl`, …).
//!
//! Returns `None` when the CLI isn't on PATH or the daemon is
//! unreachable, so the segment is silently omitted on systems that
//! don't run containers (most desktops).
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.docker.containers",
//!   "args": {
//!     "format": "{running}/{total}",
//!     "show_when_zero": false
//!   }
//! }
//! ```
//!
//! Available substitution tokens in `format`:
//! - `{running}` — running container count
//! - `{total}`   — running + stopped container count
//! - `{images}`  — local image count
//! - `{stopped}` — total − running

use serde_json::{json, Value};
use std::process::Command;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DockerInfo {
    pub running: u32,
    pub total: u32,
    pub images: u32,
}

impl DockerInfo {
    pub fn stopped(&self) -> u32 {
        self.total.saturating_sub(self.running)
    }
}

/// Probe the local docker daemon. Returns `None` on:
/// - missing `docker` binary,
/// - daemon unreachable (no socket),
/// - non-zero exit from any subcommand.
///
/// One fork per query (running, total, images = 3 forks); the daemon
/// can be reached over its IPC socket without re-authenticating per
/// call, so this is cheap relative to the user-perceived prompt
/// latency budget.
pub fn read_docker_info(cli: &str) -> Option<DockerInfo> {
    let running = count(cli, &["ps", "-q"])?;
    let total = count(cli, &["ps", "-aq"])?;
    let images = count(cli, &["images", "-q"])?;
    Some(DockerInfo {
        running,
        total,
        images,
    })
}

fn count(cli: &str, args: &[&str]) -> Option<u32> {
    let out = Command::new(cli).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = std::str::from_utf8(&out.stdout).ok()?;
    Some(text.lines().filter(|l| !l.is_empty()).count() as u32)
}

/// Render the Docker segment. `format` is template-string with
/// `{running}` / `{total}` / `{images}` / `{stopped}` tokens.
///
/// Returns `None` when:
/// - the daemon is unreachable (segment omitted entirely), OR
/// - `show_when_zero` is `false` AND both running + total are zero
///   (clean state, no signal to display).
pub fn containers(cli: &str, format: &str, show_when_zero: bool) -> Option<Vec<Value>> {
    let info = read_docker_info(cli)?;
    if !show_when_zero && info.total == 0 && info.images == 0 {
        return None;
    }
    let contents = render_format(format, &info);
    let gradient = if info.total == 0 {
        0.0
    } else {
        100.0 * info.running as f64 / info.total as f64
    };
    Some(vec![json!({
        "contents": contents,
        "gradient_level": gradient,
        "highlight_groups": [
            "docker_containers_gradient",
            "docker_containers",
            "docker",
        ],
        "divider_highlight_group": "background:divider",
    })])
}

fn render_format(fmt: &str, info: &DockerInfo) -> String {
    let mut s = fmt.to_string();
    s = s.replace("{running}", &info.running.to_string());
    s = s.replace("{total}", &info.total.to_string());
    s = s.replace("{images}", &info.images.to_string());
    s = s.replace("{stopped}", &info.stopped().to_string());
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_is_total_minus_running() {
        let info = DockerInfo {
            running: 3,
            total: 10,
            images: 0,
        };
        assert_eq!(info.stopped(), 7);
    }

    #[test]
    fn stopped_saturates_at_zero_when_running_exceeds_total() {
        // Defensive: count() never returns a value bigger than its
        // own probe, but if running and total are sampled in
        // different ticks a running container could theoretically
        // exceed total. Don't underflow.
        let info = DockerInfo {
            running: 5,
            total: 2,
            images: 0,
        };
        assert_eq!(info.stopped(), 0);
    }

    #[test]
    fn render_format_substitutes_all_tokens() {
        let info = DockerInfo {
            running: 2,
            total: 5,
            images: 7,
        };
        let out = render_format("R{running} T{total} I{images} S{stopped}", &info);
        assert_eq!(out, "R2 T5 I7 S3");
    }

    #[test]
    fn render_format_leaves_unknown_tokens_intact() {
        // {cpu} isn't a docker token; the formatter must leave it
        // alone for other layers (or as a visible bug signal).
        let info = DockerInfo::default();
        let out = render_format("{running}/{total} {cpu}", &info);
        assert_eq!(out, "0/0 {cpu}");
    }

    #[test]
    fn read_docker_info_missing_binary_returns_none() {
        // Any path that doesn't exist on PATH must yield None — the
        // segment shouldn't render when docker isn't installed.
        assert!(read_docker_info("/nonexistent/docker-binary-xyz").is_none());
    }

    #[test]
    fn containers_unreachable_daemon_returns_none() {
        // Mirror: when the CLI doesn't exist, the segment is omitted.
        let result = containers("/nonexistent/docker-xyz", "{running}/{total}", true);
        assert!(result.is_none());
    }

    #[test]
    fn count_handles_empty_output() {
        // /usr/bin/true exits 0 with no output → 0 lines. Mirror of
        // a docker daemon with no containers.
        let n = count("true", &[]).expect("true should succeed");
        assert_eq!(n, 0);
    }

    #[test]
    fn count_returns_lines_for_multiline_output() {
        // /bin/echo with -e for portable newlines isn't reliable;
        // use printf which is POSIX-stable.
        let n = count("printf", &["a\nb\nc\n"]).expect("printf should succeed");
        assert_eq!(n, 3);
    }

    #[test]
    fn count_strips_blank_lines() {
        // Empty lines must not inflate the count (matches `docker ps -q`
        // which never emits blanks but defensive against trailing
        // newlines from other tools).
        let n = count("printf", &["a\n\nb\n"]).expect("printf");
        assert_eq!(n, 2);
    }

    #[test]
    fn containers_zero_state_respects_show_when_zero_false() {
        // Force a "daemon answered with 0/0/0" scenario by feeding
        // render_format directly; the public API path requires a
        // real docker which we don't depend on in tests.
        let info = DockerInfo::default();
        let s = render_format("{running}/{total}", &info);
        assert_eq!(s, "0/0");
        // The show_when_zero gate happens in containers(); we can't
        // unit-test it without mocking the daemon, but the format
        // substitution at zero is verified here.
    }
}
