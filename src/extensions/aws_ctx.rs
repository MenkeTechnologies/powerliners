// vim:fileencoding=utf-8:noet
//! AWS context segment — active profile + region. Pure filesystem
//! probe: no subprocess, no `aws` CLI dependency. Mirrors how aws-sdk
//! and `aws --profile` resolve config:
//!
//! 1. Profile name: `$AWS_PROFILE` env, else `default`.
//! 2. Region: `$AWS_REGION` env, else `$AWS_DEFAULT_REGION` env, else
//!    the `region = <r>` line under the matching section in
//!    `$AWS_CONFIG_FILE` (default `~/.aws/config`). Section is
//!    `[default]` for the default profile, `[profile NAME]` otherwise
//!    (the canonical AWS quirk).
//!
//! Returns `None` when:
//! - no env vars are set AND
//! - the config file is unreadable OR doesn't mention the profile.
//!
//! Theme JSON:
//! ```json
//! {
//!   "function": "powerliners.aws.context",
//!   "args": {
//!     "format": "{icon} {profile}@{region}",
//!     "hide_default_profile": false
//!   }
//! }
//! ```
//!
//! Format tokens:
//! - `{icon}`    — AWS glyph (icon tier dependent)
//! - `{profile}` — active profile name (e.g. `prod`, `default`)
//! - `{region}`  — active region (e.g. `us-east-1`), empty if unknown

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AwsContext {
    pub profile: String,
    /// Empty when no region can be resolved — caller decides how to
    /// render that (drop the segment, show `?`, etc.).
    pub region: String,
}

fn config_path() -> PathBuf {
    if let Some(p) = std::env::var_os("AWS_CONFIG_FILE") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").unwrap_or_default();
    PathBuf::from(home).join(".aws").join("config")
}

/// Walk an INI-style AWS config file and pull `region = ...` out of
/// the section that matches `profile`. The default profile lives under
/// `[default]`; named profiles live under `[profile NAME]`.
pub fn region_from_config(text: &str, profile: &str) -> Option<String> {
    let target = if profile == "default" {
        "default".to_string()
    } else {
        format!("profile {profile}")
    };
    let mut in_section = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(name) = rest.strip_suffix(']') {
                in_section = name.trim() == target;
            }
            continue;
        }
        if !in_section {
            continue;
        }
        if let Some(rest) = line.strip_prefix("region") {
            let rest = rest.trim_start();
            if let Some(rest) = rest.strip_prefix('=') {
                let r = rest.trim().to_string();
                if !r.is_empty() {
                    return Some(r);
                }
            }
        }
    }
    None
}

/// Resolve the active AWS context. `config_text` is `Some(...)` when
/// a config file is readable. Pure function — separated so tests can
/// drive arbitrary env + config combos.
pub fn resolve(
    env_profile: Option<&str>,
    env_region: Option<&str>,
    env_default_region: Option<&str>,
    config_text: Option<&str>,
) -> Option<AwsContext> {
    let profile = env_profile
        .filter(|s| !s.is_empty())
        .unwrap_or("default")
        .to_string();
    let region = env_region
        .or(env_default_region)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| config_text.and_then(|t| region_from_config(t, &profile)))
        .unwrap_or_default();
    // If profile is the literal "default" AND region is empty AND no
    // env signal pointed at AWS at all, treat this as "no AWS context"
    // — don't render a useless `default@` for users who never touch
    // AWS.
    if profile == "default"
        && region.is_empty()
        && env_profile.is_none()
        && env_region.is_none()
        && env_default_region.is_none()
        && config_text.is_none()
    {
        return None;
    }
    Some(AwsContext { profile, region })
}

/// Live probe — reads the process env and the on-disk config file.
pub fn read_aws_context() -> Option<AwsContext> {
    let env_profile = std::env::var("AWS_PROFILE").ok();
    let env_region = std::env::var("AWS_REGION").ok();
    let env_default_region = std::env::var("AWS_DEFAULT_REGION").ok();
    let cfg = fs::read_to_string(config_path()).ok();
    resolve(
        env_profile.as_deref(),
        env_region.as_deref(),
        env_default_region.as_deref(),
        cfg.as_deref(),
    )
}

/// Render the segment. `hide_default_profile` drops the `{profile}`
/// fragment (and a single adjacent separator) when profile is
/// `default` — same convention as the k8s `hide_default` flag.
pub fn context(format: &str, hide_default_profile: bool) -> Option<Vec<Value>> {
    let state = read_aws_context()?;
    let contents = if hide_default_profile && state.profile == "default" {
        drop_token(format, "{profile}", &state.region)
    } else {
        format
            .replace("{profile}", &state.profile)
            .replace("{region}", &state.region)
    };
    Some(vec![json!({
        "contents": contents,
        "highlight_groups": ["aws_context", "aws"],
        "divider_highlight_group": "background:divider",
    })])
}

/// Substitute `{region}` into `format` while stripping `{profile}`
/// and a single adjacent separator (`:` `@` `/`). Lets a template like
/// `{profile}@{region}` collapse cleanly to `{region}` when the profile
/// is hidden.
fn drop_token(fmt: &str, drop: &str, region: &str) -> String {
    let mut s = fmt.to_string();
    for sep in [
        format!("{drop}@"),
        format!("{drop}:"),
        format!("{drop}/"),
        format!("@{drop}"),
        format!(":{drop}"),
        format!("/{drop}"),
    ] {
        if s.contains(&sep) {
            s = s.replace(&sep, "");
            return s.replace("{region}", region);
        }
    }
    s.replace(drop, "").replace("{region}", region)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_from_config_default_section() {
        let text = "[default]\nregion = us-east-1\noutput = json\n";
        assert_eq!(
            region_from_config(text, "default"),
            Some("us-east-1".into())
        );
    }

    #[test]
    fn region_from_config_named_profile() {
        let text = "[default]\nregion = us-east-1\n\n[profile prod]\nregion = eu-west-2\n";
        assert_eq!(region_from_config(text, "prod"), Some("eu-west-2".into()));
    }

    #[test]
    fn region_from_config_missing_profile_returns_none() {
        let text = "[default]\nregion = us-east-1\n";
        assert!(region_from_config(text, "staging").is_none());
    }

    #[test]
    fn region_from_config_ignores_comments() {
        let text = "[default]\n# region = wrong\nregion = us-west-1\n";
        assert_eq!(
            region_from_config(text, "default"),
            Some("us-west-1".into())
        );
    }

    #[test]
    fn region_from_config_handles_empty_value() {
        let text = "[default]\nregion =\n";
        assert!(region_from_config(text, "default").is_none());
    }

    #[test]
    fn resolve_env_profile_and_region() {
        let s = resolve(Some("prod"), Some("us-east-2"), None, None).unwrap();
        assert_eq!(s.profile, "prod");
        assert_eq!(s.region, "us-east-2");
    }

    #[test]
    fn resolve_default_env_region_fallback() {
        let s = resolve(Some("staging"), None, Some("us-west-1"), None).unwrap();
        assert_eq!(s.profile, "staging");
        assert_eq!(s.region, "us-west-1");
    }

    #[test]
    fn resolve_config_region_when_env_missing() {
        let cfg = "[profile prod]\nregion = ap-south-1\n";
        let s = resolve(Some("prod"), None, None, Some(cfg)).unwrap();
        assert_eq!(s.region, "ap-south-1");
    }

    #[test]
    fn resolve_returns_none_when_no_signal() {
        // No env, no config — looks like a machine that never touched
        // AWS. The segment must drop out completely.
        assert!(resolve(None, None, None, None).is_none());
    }

    #[test]
    fn resolve_default_with_config_returns_some() {
        // Config file exists but doesn't mention default — still a
        // signal that AWS is configured somewhere on the machine.
        let cfg = "[profile prod]\nregion = eu-west-1\n";
        let s = resolve(None, None, None, Some(cfg)).unwrap();
        assert_eq!(s.profile, "default");
        assert_eq!(s.region, "");
    }

    #[test]
    fn drop_token_strips_at_separator() {
        let s = drop_token("{profile}@{region}", "{profile}", "us-east-1");
        assert_eq!(s, "us-east-1");
    }

    #[test]
    fn drop_token_strips_colon_separator() {
        let s = drop_token("{profile}:{region}", "{profile}", "eu-west-2");
        assert_eq!(s, "eu-west-2");
    }

    #[test]
    fn drop_token_handles_no_separator() {
        let s = drop_token("{profile} {region}", "{profile}", "us-west-1");
        assert_eq!(s, " us-west-1");
    }
}
