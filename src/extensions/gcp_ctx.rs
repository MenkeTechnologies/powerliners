// vim:fileencoding=utf-8:noet
//! GCP context segment — active gcloud configuration's project +
//! account. Pure filesystem probe: no `gcloud` subprocess.
//!
//! Resolution mirrors `gcloud config configurations describe`:
//! 1. Active configuration name from `$CLOUDSDK_ACTIVE_CONFIG_NAME`,
//!    else the first non-blank line of
//!    `$CLOUDSDK_CONFIG/active_config` (default `~/.config/gcloud/active_config`).
//! 2. Parse the matching `configurations/config_<NAME>` INI file:
//!    - `[core] project = ...`
//!    - `[core] account = ...`
//!
//! Env override hooks honored by the SDK also win here:
//! - `$CLOUDSDK_CORE_PROJECT`  → project
//! - `$CLOUDSDK_CORE_ACCOUNT`  → account
//!
//! Returns `None` when no active config can be located AND no env
//! overrides are set — i.e. the user has no gcloud install or has
//! never run `gcloud init`.
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.gcp.context",
//!   "args": {
//!     "format": "{icon} {project}:{account}",
//!     "hide_account": false
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{icon}`    — GCP glyph
//! - `{project}` — active GCP project id
//! - `{account}` — active gcloud account email
//! - `{config}`  — active configuration name (e.g. `default`, `prod`)

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GcpContext {
    pub config: String,
    pub project: String,
    pub account: String,
}

fn gcloud_root() -> PathBuf {
    if let Some(p) = std::env::var_os("CLOUDSDK_CONFIG") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".config").join("gcloud")
}

/// Pull `project` and `account` out of an INI-style gcloud
/// configuration file. Returns `(project, account)`, either of which
/// may be empty when the file omits it.
pub fn parse_config_file(text: &str) -> (String, String) {
    let mut project = String::new();
    let mut account = String::new();
    let mut in_core = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(name) = rest.strip_suffix(']') {
                in_core = name.trim() == "core";
            }
            continue;
        }
        if !in_core {
            continue;
        }
        if let Some(v) = ini_value(line, "project") {
            project = v;
        } else if let Some(v) = ini_value(line, "account") {
            account = v;
        }
    }
    (project, account)
}

fn ini_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim();
    if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    }
}

/// Pure-data resolver. Tests inject env + on-disk strings directly.
pub fn resolve(
    env_config: Option<&str>,
    env_project: Option<&str>,
    env_account: Option<&str>,
    active_config_file: Option<&str>,
    pick_config_text: impl FnOnce(&str) -> Option<String>,
) -> Option<GcpContext> {
    let config = env_config
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            active_config_file
                .and_then(|t| t.lines().find(|l| !l.trim().is_empty()).map(str::trim))
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        });
    let (file_project, file_account) = match config.as_deref() {
        Some(name) => pick_config_text(name)
            .map(|t| parse_config_file(&t))
            .unwrap_or_default(),
        None => (String::new(), String::new()),
    };
    let project = env_project
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(file_project);
    let account = env_account
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or(file_account);
    let cfg = config.unwrap_or_default();
    if cfg.is_empty() && project.is_empty() && account.is_empty() {
        return None;
    }
    Some(GcpContext {
        config: cfg,
        project,
        account,
    })
}

/// Live probe against the real filesystem + env.
pub fn read_gcp_context() -> Option<GcpContext> {
    let root = gcloud_root();
    let env_config = std::env::var("CLOUDSDK_ACTIVE_CONFIG_NAME").ok();
    let env_project = std::env::var("CLOUDSDK_CORE_PROJECT").ok();
    let env_account = std::env::var("CLOUDSDK_CORE_ACCOUNT").ok();
    let active = fs::read_to_string(root.join("active_config")).ok();
    resolve(
        env_config.as_deref(),
        env_project.as_deref(),
        env_account.as_deref(),
        active.as_deref(),
        |name| fs::read_to_string(root.join("configurations").join(format!("config_{name}"))).ok(),
    )
}

/// Render the segment. `hide_account` drops the account fragment +
/// adjacent separator when set, matching the k8s `hide_default`
/// convention.
pub fn context(format: &str, hide_account: bool) -> Option<Vec<Value>> {
    let state = read_gcp_context()?;
    let contents = if hide_account {
        drop_account(format, &state.project, &state.config)
    } else {
        format
            .replace("{project}", &state.project)
            .replace("{account}", &state.account)
            .replace("{config}", &state.config)
    };
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": ["gcp_context", "gcp"],
        "divider_highlight_group": "background:divider",
    })])
}

fn drop_account(fmt: &str, project: &str, config: &str) -> String {
    let mut s = fmt.to_string();
    for sep in [
        ":{account}",
        "@{account}",
        "/{account}",
        "{account}:",
        "{account}@",
        "{account}/",
    ] {
        if s.contains(sep) {
            s = s.replace(sep, "");
            return s
                .replace("{project}", project)
                .replace("{config}", config);
        }
    }
    s.replace("{account}", "")
        .replace("{project}", project)
        .replace("{config}", config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_file_extracts_project_and_account() {
        let text = "[core]\nproject = my-gcp-proj\naccount = me@example.com\n";
        let (p, a) = parse_config_file(text);
        assert_eq!(p, "my-gcp-proj");
        assert_eq!(a, "me@example.com");
    }

    #[test]
    fn parse_config_file_ignores_other_sections() {
        let text = "[compute]\nproject = wrong\n\n[core]\nproject = right\n";
        let (p, _) = parse_config_file(text);
        assert_eq!(p, "right");
    }

    #[test]
    fn parse_config_file_handles_missing_keys() {
        let text = "[core]\nproject = only-project\n";
        let (p, a) = parse_config_file(text);
        assert_eq!(p, "only-project");
        assert_eq!(a, "");
    }

    #[test]
    fn parse_config_file_skips_comments() {
        let text = "[core]\n# project = ignored\nproject = real\n";
        let (p, _) = parse_config_file(text);
        assert_eq!(p, "real");
    }

    #[test]
    fn resolve_env_overrides_win() {
        let s = resolve(
            Some("prod"),
            Some("env-proj"),
            Some("env@e.com"),
            Some("prod\n"),
            |_| Some("[core]\nproject = file-proj\naccount = file@e.com\n".to_string()),
        )
        .unwrap();
        assert_eq!(s.project, "env-proj");
        assert_eq!(s.account, "env@e.com");
        assert_eq!(s.config, "prod");
    }

    #[test]
    fn resolve_falls_back_to_file() {
        let s = resolve(
            None,
            None,
            None,
            Some("default\n"),
            |name| {
                assert_eq!(name, "default");
                Some("[core]\nproject = file-proj\naccount = file@e.com\n".to_string())
            },
        )
        .unwrap();
        assert_eq!(s.config, "default");
        assert_eq!(s.project, "file-proj");
        assert_eq!(s.account, "file@e.com");
    }

    #[test]
    fn resolve_returns_none_with_no_signal() {
        // No env, no active_config — gcloud was never set up.
        assert!(resolve(None, None, None, None, |_| None).is_none());
    }

    #[test]
    fn resolve_returns_some_with_only_env_project() {
        let s = resolve(None, Some("p"), None, None, |_| None).unwrap();
        assert_eq!(s.project, "p");
        assert!(s.config.is_empty());
        assert!(s.account.is_empty());
    }

    #[test]
    fn drop_account_strips_colon() {
        let s = drop_account("{project}:{account}", "p", "");
        assert_eq!(s, "p");
    }

    #[test]
    fn drop_account_strips_at() {
        let s = drop_account("{account}@{project}", "p", "");
        assert_eq!(s, "p");
    }
}
