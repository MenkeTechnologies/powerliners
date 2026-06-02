// vim:fileencoding=utf-8:noet
//! GitHub CI status segment — shows aggregate state of check-runs for
//! the current branch's HEAD commit. Designed for the "push, watch
//! prompt for green" loop: one `gh api` call, cached on disk by SHA so
//! repeated prompt redraws don't hammer the network.
//!
//! Resolution chain (no env contract — all probed from `cwd`):
//! 1. `git -C <cwd> config --get remote.origin.url`        → owner/repo
//! 2. `git -C <cwd> rev-parse HEAD`                        → SHA
//! 3. cache hit at `$XDG_CACHE_HOME/powerliners/github_ci/<owner>_<repo>_<sha>.json`
//!    (mtime within `ttl_secs`)? → reuse
//! 4. else `gh api repos/<o>/<r>/commits/<sha>/check-runs` → cache + render
//!
//! Returns `None` when:
//! - cwd isn't a GitHub work tree (no `github.com` in origin URL),
//! - `gh` isn't on PATH AND no fresh cache entry exists,
//! - the API call fails (rate limit, auth, offline) AND no cache entry.
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.github.ci_status",
//!   "args": {
//!     "format": "{icon} {state} {passed}/{total}",
//!     "ttl_secs": 30,
//!     "cli": "gh"
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{icon}`    — state-keyed glyph (success/failure/pending/none)
//! - `{state}`   — `ok` / `fail` / `run` / `none`
//! - `{passed}`  — count of successful check-runs
//! - `{failed}`  — count of failed/cancelled check-runs
//! - `{running}` — count of in-flight check-runs
//! - `{total}`   — total check-runs reported

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CiState {
    pub passed: u32,
    pub failed: u32,
    pub running: u32,
    pub total: u32,
}

impl CiState {
    /// Aggregate state. Failure beats running beats success — a single
    /// red check overrides any number of green ones, matching how the
    /// GitHub UI badges a commit.
    pub fn label(&self) -> &'static str {
        if self.total == 0 {
            "none"
        } else if self.failed > 0 {
            "fail"
        } else if self.running > 0 {
            "run"
        } else {
            "ok"
        }
    }
}

/// Parse `owner/repo` out of a remote URL. Handles `https://github.com/o/r(.git)?`
/// and `git@github.com:o/r(.git)?` shapes. Returns `None` for non-GitHub URLs.
pub fn parse_owner_repo(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let tail = if let Some(t) = url.strip_prefix("git@github.com:") {
        t
    } else if let Some(t) = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
    {
        t
    } else {
        return None;
    };
    let tail = tail.strip_suffix(".git").unwrap_or(tail);
    let mut parts = tail.splitn(2, '/');
    let owner = parts.next()?.to_string();
    let repo = parts.next()?.to_string();
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    // Trim any trailing path segment (e.g. `o/r/pulls/3` should never
    // reach here, but a trailing slash from a sloppy origin URL would).
    let repo = repo.split('/').next().unwrap_or(&repo).to_string();
    Some((owner, repo))
}

fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("powerliners").join("github_ci")
}

fn cache_path(owner: &str, repo: &str, sha: &str) -> PathBuf {
    cache_dir().join(format!("{owner}_{repo}_{sha}.json"))
}

fn read_fresh_cache(path: &std::path::Path, ttl_secs: u64) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let age = SystemTime::now().duration_since(mtime).ok()?.as_secs();
    if age > ttl_secs {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn write_cache(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, body);
}

/// Parse a `gh api check-runs` response body into `CiState`. The
/// payload shape is `{"total_count": N, "check_runs": [{status, conclusion}, ...]}`.
/// `status` ∈ {queued, in_progress, completed}; `conclusion` ∈
/// {success, failure, neutral, cancelled, skipped, timed_out,
/// action_required, stale, null}. Failure-class conclusions roll into
/// `failed`; non-completed status rolls into `running`.
pub fn parse_check_runs(body: &str) -> CiState {
    let mut state = CiState::default();
    let v: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return state,
    };
    let runs = match v.get("check_runs").and_then(|x| x.as_array()) {
        Some(r) => r,
        None => return state,
    };
    for run in runs {
        state.total += 1;
        let status = run.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status != "completed" {
            state.running += 1;
            continue;
        }
        let conclusion = run.get("conclusion").and_then(|s| s.as_str()).unwrap_or("");
        match conclusion {
            "success" | "neutral" | "skipped" => state.passed += 1,
            "failure" | "cancelled" | "timed_out" | "action_required" | "stale" => {
                state.failed += 1
            }
            _ => {}
        }
    }
    state
}

/// Resolve current HEAD SHA via `git rev-parse HEAD`. Returns `None`
/// outside a git work tree.
pub fn read_head_sha(cwd: &str) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", cwd, "rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Resolve owner/repo from `remote.origin.url`. Returns `None` when
/// the remote is missing or non-GitHub.
pub fn read_owner_repo(cwd: &str) -> Option<(String, String)> {
    let out = Command::new("git")
        .args(["-C", cwd, "config", "--get", "remote.origin.url"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8(out.stdout).ok()?;
    parse_owner_repo(&url)
}

fn run_gh(cli: &str, owner: &str, repo: &str, sha: &str) -> Option<String> {
    let path = format!("repos/{owner}/{repo}/commits/{sha}/check-runs");
    let out = Command::new(cli).args(["api", &path]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// Public probe — combines git inspection, cache lookup, and gh call.
/// `ttl_secs` controls cache freshness; `cli` is the gh binary
/// (overridable for tests).
pub fn read_ci_state(cwd: &str, cli: &str, ttl_secs: u64) -> Option<CiState> {
    let (owner, repo) = read_owner_repo(cwd)?;
    let sha = read_head_sha(cwd)?;
    let path = cache_path(&owner, &repo, &sha);
    if let Some(body) = read_fresh_cache(&path, ttl_secs) {
        return Some(parse_check_runs(&body));
    }
    let body = run_gh(cli, &owner, &repo, &sha)?;
    write_cache(&path, &body);
    Some(parse_check_runs(&body))
}

fn pick_icon(state: &CiState, ok_icon: &str, fail_icon: &str, run_icon: &str) -> String {
    match state.label() {
        "ok" => ok_icon.to_string(),
        "fail" => fail_icon.to_string(),
        "run" => run_icon.to_string(),
        _ => String::new(),
    }
}

/// Render the segment. Returns `None` when `read_ci_state` does or
/// when `total == 0` (no CI configured for this SHA — don't clutter).
pub fn ci_status(
    cwd: &str,
    cli: &str,
    format: &str,
    ttl_secs: u64,
    ok_icon: &str,
    fail_icon: &str,
    run_icon: &str,
) -> Option<Vec<Value>> {
    let state = read_ci_state(cwd, cli, ttl_secs)?;
    if state.total == 0 {
        return None;
    }
    let icon = pick_icon(&state, ok_icon, fail_icon, run_icon);
    let contents = format
        .replace("{icon}", &icon)
        .replace("{state}", state.label())
        .replace("{passed}", &state.passed.to_string())
        .replace("{failed}", &state.failed.to_string())
        .replace("{running}", &state.running.to_string())
        .replace("{total}", &state.total.to_string());
    let primary = match state.label() {
        "ok" => "github_ci_success",
        "fail" => "github_ci_failure",
        "run" => "github_ci_pending",
        _ => "github_ci",
    };
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": [primary, "github_ci"],
        "divider_highlight_group": "background:divider",
    })])
}

/// Touch UNIX_EPOCH so the `unused` warning on the re-export is
/// suppressed in tests — referenced only when computing cache age.
#[allow(dead_code)]
fn _epoch_anchor() -> SystemTime {
    UNIX_EPOCH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_owner_repo_https() {
        assert_eq!(
            parse_owner_repo("https://github.com/MenkeTechnologies/powerliners.git"),
            Some(("MenkeTechnologies".into(), "powerliners".into()))
        );
    }

    #[test]
    fn parse_owner_repo_https_no_suffix() {
        assert_eq!(
            parse_owner_repo("https://github.com/o/r"),
            Some(("o".into(), "r".into()))
        );
    }

    #[test]
    fn parse_owner_repo_ssh() {
        assert_eq!(
            parse_owner_repo("git@github.com:o/r.git"),
            Some(("o".into(), "r".into()))
        );
    }

    #[test]
    fn parse_owner_repo_ssh_scheme() {
        assert_eq!(
            parse_owner_repo("ssh://git@github.com/o/r.git"),
            Some(("o".into(), "r".into()))
        );
    }

    #[test]
    fn parse_owner_repo_non_github_returns_none() {
        assert!(parse_owner_repo("https://gitlab.com/o/r.git").is_none());
        assert!(parse_owner_repo("https://bitbucket.org/o/r.git").is_none());
    }

    #[test]
    fn parse_owner_repo_empty_returns_none() {
        assert!(parse_owner_repo("").is_none());
        assert!(parse_owner_repo("https://github.com/").is_none());
        assert!(parse_owner_repo("https://github.com/o").is_none());
    }

    #[test]
    fn label_none_when_no_runs() {
        assert_eq!(CiState::default().label(), "none");
    }

    #[test]
    fn label_failure_overrides_running() {
        let s = CiState {
            passed: 5,
            failed: 1,
            running: 3,
            total: 9,
        };
        assert_eq!(s.label(), "fail");
    }

    #[test]
    fn label_running_when_no_failures() {
        let s = CiState {
            passed: 5,
            failed: 0,
            running: 2,
            total: 7,
        };
        assert_eq!(s.label(), "run");
    }

    #[test]
    fn label_ok_when_all_passed() {
        let s = CiState {
            passed: 4,
            failed: 0,
            running: 0,
            total: 4,
        };
        assert_eq!(s.label(), "ok");
    }

    #[test]
    fn parse_check_runs_empty_payload() {
        let s = parse_check_runs(r#"{"total_count":0,"check_runs":[]}"#);
        assert_eq!(s, CiState::default());
    }

    #[test]
    fn parse_check_runs_all_success() {
        let body = r#"{
            "total_count": 2,
            "check_runs": [
                {"status": "completed", "conclusion": "success"},
                {"status": "completed", "conclusion": "success"}
            ]
        }"#;
        let s = parse_check_runs(body);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 0);
        assert_eq!(s.running, 0);
        assert_eq!(s.total, 2);
        assert_eq!(s.label(), "ok");
    }

    #[test]
    fn parse_check_runs_mixed() {
        let body = r#"{
            "total_count": 4,
            "check_runs": [
                {"status": "completed", "conclusion": "success"},
                {"status": "completed", "conclusion": "failure"},
                {"status": "in_progress", "conclusion": null},
                {"status": "completed", "conclusion": "skipped"}
            ]
        }"#;
        let s = parse_check_runs(body);
        assert_eq!(s.passed, 2);
        assert_eq!(s.failed, 1);
        assert_eq!(s.running, 1);
        assert_eq!(s.total, 4);
        assert_eq!(s.label(), "fail");
    }

    #[test]
    fn parse_check_runs_malformed_returns_default() {
        assert_eq!(parse_check_runs("not json at all"), CiState::default());
        assert_eq!(parse_check_runs("{}"), CiState::default());
    }

    #[test]
    fn read_ci_state_outside_repo_returns_none() {
        // `/tmp` is not a git work tree on CI.
        assert!(read_ci_state("/tmp", "/nonexistent/gh-xyz", 30).is_none());
    }

    #[test]
    fn ci_status_outside_repo_returns_none() {
        assert!(ci_status("/tmp", "/nonexistent/gh", "{state}", 30, "v", "x", "~").is_none());
    }
}
