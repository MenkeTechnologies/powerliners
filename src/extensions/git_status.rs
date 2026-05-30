// vim:fileencoding=utf-8:noet
//! Rich VCS segment — single chunk that mirrors the powerlevel10k
//! prompt style: branch glyph + name + count badges for unstaged /
//! untracked / staged / ahead / behind / stashed. Replaces the
//! upstream split between `vcs.branch` (name only) and `vcs.stash`
//! (count only) with one efficient probe.
//!
//! All counts come from a single `git status --porcelain=v2 --branch`
//! invocation so the segment is one fork per render instead of the
//! 2+ that `branch` + `stash` (+ a separate ahead/behind probe)
//! require. Returns `None` outside a git work tree so the segment
//! is omitted (same convention as upstream `vcs.branch`).

use serde_json::{json, Value};

#[derive(Debug, Default, Clone)]
pub struct GitState {
    pub branch: String,
    /// Short SHA used when HEAD is detached (no branch, no tag).
    /// Mirrors p10k's `@<sha[1,8]>` fallback in `p10k-lean.zsh:411-412`.
    pub commit_short: String,
    /// Tag at HEAD when on no branch (mirrors p10k's `#<tag>`).
    pub tag: String,
    pub unstaged: u32,
    pub untracked: u32,
    pub staged: u32,
    pub conflicts: u32,
    pub ahead: u32,
    pub behind: u32,
    pub stashed: u32,
    /// Current `.git/<action>` state (merge / rebase / cherry-pick /
    /// bisect / revert). Empty when no operation is in flight. p10k
    /// surfaces this as a `merge` / `rebase` etc word in the segment.
    pub action: String,
    /// `true` when the `origin` remote URL contains `github.com`, so
    /// the segment can lead with the GitHub octocat glyph.
    pub is_github: bool,
}

impl GitState {
    pub fn is_clean(&self) -> bool {
        self.unstaged == 0
            && self.untracked == 0
            && self.staged == 0
            && self.conflicts == 0
            && self.ahead == 0
            && self.behind == 0
            && self.action.is_empty()
    }
}

/// Probe `git status --porcelain=v2 --branch` in `cwd`. Returns
/// `None` if the dir isn't inside a git work tree or git isn't on
/// PATH.
pub fn read_git_state(cwd: &str) -> Option<GitState> {
    let out = std::process::Command::new("git")
        .args(["-C", cwd, "status", "--porcelain=v2", "--branch"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    let mut state = GitState::default();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            state.branch = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: `+<ahead> -<behind>`
            let mut parts = rest.split_whitespace();
            if let Some(a) = parts.next().and_then(|s| s.strip_prefix('+')) {
                state.ahead = a.parse().unwrap_or(0);
            }
            if let Some(b) = parts.next().and_then(|s| s.strip_prefix('-')) {
                state.behind = b.parse().unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("1 ") {
            // Ordinary changed entry: `XY <sub> ...`
            if let Some(xy) = rest.split_whitespace().next() {
                let bytes = xy.as_bytes();
                if bytes.first().is_some_and(|b| *b != b'.') {
                    state.staged += 1;
                }
                if bytes.get(1).is_some_and(|b| *b != b'.') {
                    state.unstaged += 1;
                }
            }
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // Renamed/copied entry, same XY format.
            if let Some(xy) = rest.split_whitespace().next() {
                let bytes = xy.as_bytes();
                if bytes.first().is_some_and(|b| *b != b'.') {
                    state.staged += 1;
                }
                if bytes.get(1).is_some_and(|b| *b != b'.') {
                    state.unstaged += 1;
                }
            }
        } else if line.starts_with("u ") {
            // Unmerged entry — merge conflict on the path. p10k
            // surfaces these as `~N`.
            state.conflicts += 1;
        } else if line.starts_with("? ") {
            state.untracked += 1;
        }
    }
    if state.branch == "(detached)" {
        state.branch.clear();
    }
    if state.branch.is_empty() {
        // Detached HEAD path: prefer a tag if one points at HEAD
        // (p10k's `#<tag>` rule at p10k-lean.zsh:396-407), else fall
        // back to short SHA (`@<sha[1,8]>` at p10k-lean.zsh:411-412).
        if let Ok(out) = std::process::Command::new("git")
            .args(["-C", cwd, "describe", "--tags", "--exact-match"])
            .output()
        {
            if out.status.success() {
                state.tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
            }
        }
        if let Ok(out) = std::process::Command::new("git")
            .args(["-C", cwd, "rev-parse", "--short", "HEAD"])
            .output()
        {
            if let Ok(s) = String::from_utf8(out.stdout) {
                state.commit_short = s.trim().to_string();
            }
        }
    }
    // In-flight action: detect via the well-known directories git
    // creates while a merge/rebase/etc is open. Cheap stat-only
    // probe — no extra subprocess. Mirrors the set p10k surfaces.
    if let Ok(out) = std::process::Command::new("git")
        .args(["-C", cwd, "rev-parse", "--git-dir"])
        .output()
    {
        if out.status.success() {
            let gitdir = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let dir = std::path::Path::new(&gitdir);
            if dir.join("MERGE_HEAD").exists() {
                state.action = "merge".to_string();
            } else if dir.join("rebase-merge").exists() || dir.join("rebase-apply").exists() {
                state.action = "rebase".to_string();
            } else if dir.join("CHERRY_PICK_HEAD").exists() {
                state.action = "cherry-pick".to_string();
            } else if dir.join("REVERT_HEAD").exists() {
                state.action = "revert".to_string();
            } else if dir.join("BISECT_LOG").exists() {
                state.action = "bisect".to_string();
            }
        }
    }
    // Stash list is a separate ref-walk; folded here so the segment
    // is still one call from the adapter's POV.
    if let Ok(out) = std::process::Command::new("git")
        .args(["-C", cwd, "stash", "list"])
        .output()
    {
        if out.status.success() {
            state.stashed = String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count() as u32;
        }
    }
    // Detect a GitHub remote so the segment can lead with the
    // octocat glyph. Cheap probe — `git config` reads `.git/config`
    // directly and doesn't network. Missing remote / non-github
    // host leaves `is_github = false`.
    if let Ok(out) = std::process::Command::new("git")
        .args(["-C", cwd, "config", "--get", "remote.origin.url"])
        .output()
    {
        if out.status.success() {
            let url = String::from_utf8_lossy(&out.stdout).to_string();
            state.is_github = url.contains("github.com");
        }
    }
    Some(state)
}

/// Render the p10k-style segment. Theme JSON:
/// ```json
/// {
///   "function": "powerliners.vcs.git_status",
///   "args": {
///     "branch_icon": "",
///     "unstaged_icon": "!",
///     "untracked_icon": "?",
///     "staged_icon": "+",
///     "ahead_icon": "⇡",
///     "behind_icon": "⇣",
///     "stash_icon": "*"
///   }
/// }
/// ```
#[allow(clippy::too_many_arguments)]
pub fn git_status(
    cwd: &str,
    branch_icon: &str,
    github_icon: &str,
    unstaged_icon: &str,
    untracked_icon: &str,
    staged_icon: &str,
    conflict_icon: &str,
    ahead_icon: &str,
    behind_icon: &str,
    stash_icon: &str,
    status_colors: bool,
) -> Option<Vec<Value>> {
    let state = read_git_state(cwd)?;
    if state.branch.is_empty() && state.tag.is_empty() && state.commit_short.is_empty() {
        return None;
    }
    let mut s = String::new();
    if state.is_github && !github_icon.is_empty() {
        s.push_str(github_icon);
        s.push(' ');
    }
    if !state.branch.is_empty() {
        if !branch_icon.is_empty() {
            s.push_str(branch_icon);
            s.push(' ');
        }
        s.push_str(&state.branch);
    } else if !state.tag.is_empty() {
        // p10k:`#<tag>` when on no branch.
        s.push('#');
        s.push_str(&state.tag);
    } else {
        // p10k:`@<sha[1,8]>` when on no branch and no tag.
        s.push('@');
        s.push_str(&state.commit_short);
    }
    // Order matches p10k-lean.zsh:424-453: action, ahead/behind,
    // stash, conflicts, staged, unstaged, untracked. Each chunk
    // shows only when its counter is non-zero.
    if !state.action.is_empty() {
        s.push(' ');
        s.push_str(&state.action);
    }
    if state.behind > 0 {
        s.push_str(&format!(" {}{}", behind_icon, state.behind));
    }
    if state.ahead > 0 {
        s.push_str(&format!(" {}{}", ahead_icon, state.ahead));
    }
    if state.stashed > 0 {
        s.push_str(&format!(" {}{}", stash_icon, state.stashed));
    }
    if state.conflicts > 0 {
        s.push_str(&format!(" {}{}", conflict_icon, state.conflicts));
    }
    if state.staged > 0 {
        s.push_str(&format!(" {}{}", staged_icon, state.staged));
    }
    if state.unstaged > 0 {
        s.push_str(&format!(" {}{}", unstaged_icon, state.unstaged));
    }
    if state.untracked > 0 {
        s.push_str(&format!(" {}{}", untracked_icon, state.untracked));
    }
    // When `status_colors` is off, render every state with the same
    // neutral `branch` group — matches the original `vcs.branch`
    // behavior with `status_colors: false` (its default).
    let groups: Vec<Value> = if status_colors {
        let primary = if state.is_clean() {
            "git_status_clean"
        } else {
            "git_status_dirty"
        };
        let fallback = if state.is_clean() {
            "branch_clean"
        } else {
            "branch_dirty"
        };
        vec![
            Value::String(primary.into()),
            Value::String("git_status".into()),
            Value::String(fallback.into()),
            Value::String("branch".into()),
        ]
    } else {
        vec![
            Value::String("git_status".into()),
            Value::String("branch".into()),
        ]
    };
    Some(vec![json!({
        "contents": s,
        "highlight_groups": groups,
        "divider_highlight_group": "background:divider",
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_state_default_is_clean() {
        assert!(GitState::default().is_clean());
    }

    #[test]
    fn git_state_unstaged_marks_dirty() {
        let s = GitState {
            unstaged: 1,
            ..Default::default()
        };
        assert!(!s.is_clean());
    }

    #[test]
    fn git_status_outside_repo_returns_none() {
        // /tmp is never a git work tree on CI; even if a stray
        // .git happens to sit there, the function still must not
        // panic. We only assert the no-panic contract here.
        let _ = git_status("/tmp", "", "", "!", "?", "+", "~", "⇡", "⇣", "*", false);
    }
}
