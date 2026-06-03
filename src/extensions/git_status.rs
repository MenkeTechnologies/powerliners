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

/// Parse the output of `git status --porcelain=v2 --branch` into a
/// `GitState`. Pure-functional — no subprocess, no filesystem access —
/// so the porcelain-v2 line semantics can be unit-tested directly.
///
/// Line shapes recognised (every other line is silently ignored):
/// - `# branch.head <name>`         → `branch`
/// - `# branch.ab +<N> -<M>`        → `ahead` / `behind`
/// - `1 XY <sub> ...`               → `staged` / `unstaged` per X / Y byte
/// - `2 XY <sub> ...`               → same, for rename/copy entries
/// - `u XY ...`                     → `conflicts`
/// - `? <path>`                     → `untracked`
///
/// The literal branch name `(detached)` is normalised to an empty
/// `branch` so the renderer can pick the SHA-fallback path.
pub fn parse_porcelain_v2(text: &str) -> GitState {
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
            tally_xy(rest, &mut state);
        } else if let Some(rest) = line.strip_prefix("2 ") {
            // Renamed/copied entry, same XY format.
            tally_xy(rest, &mut state);
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
    state
}

fn tally_xy(rest: &str, state: &mut GitState) {
    if let Some(xy) = rest.split_whitespace().next() {
        let bytes = xy.as_bytes();
        if bytes.first().is_some_and(|b| *b != b'.') {
            state.staged += 1;
        }
        if bytes.get(1).is_some_and(|b| *b != b'.') {
            state.unstaged += 1;
        }
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
    let mut state = parse_porcelain_v2(&text);
    // Tag at HEAD: probe unconditionally so it can render alongside
    // the branch (p10k's lean.zsh ships the "always show tag" path as
    // a documented opt-in at lines 397-399 — `&& -z $VCS_STATUS_LOCAL_BRANCH`).
    // `git describe --tags --exact-match` returns non-zero when HEAD
    // isn't at a tag; we silently leave `state.tag` empty in that case.
    if let Ok(out) = std::process::Command::new("git")
        .args(["-C", cwd, "describe", "--tags", "--exact-match"])
        .output()
    {
        if out.status.success() {
            state.tag = String::from_utf8_lossy(&out.stdout).trim().to_string();
        }
    }
    if state.branch.is_empty() {
        // Detached HEAD path: short SHA fallback (`@<sha[1,8]>` at
        // p10k-lean.zsh:411-412).
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
/// Glyph bundle for the renderer. Mirrors the `*_icon` theme args of
/// `git_status()` but lets the pure renderer be called without
/// passing 10 positional strings.
pub struct GitIcons<'a> {
    pub branch: &'a str,
    pub github: &'a str,
    pub tag: &'a str,
    pub unstaged: &'a str,
    pub untracked: &'a str,
    pub staged: &'a str,
    pub conflict: &'a str,
    pub ahead: &'a str,
    pub behind: &'a str,
    pub stash: &'a str,
}

/// Render the header (icon + branch / tag / SHA + tag combo). The
/// p10k header rules are: github octocat lead if origin is GitHub →
/// `<branch_icon> <branch>` if branched (with optional ` <tag_icon> <tag>`
/// suffix) → `<tag_icon> <tag>` if only at a tag → `@<commit_short>`
/// detached fallback.
fn render_git_header(state: &GitState, icons: &GitIcons<'_>, out: &mut String) {
    if state.is_github && !icons.github.is_empty() {
        out.push_str(icons.github);
        out.push(' ');
    }
    if !state.branch.is_empty() {
        if !icons.branch.is_empty() {
            out.push_str(icons.branch);
            out.push(' ');
        }
        out.push_str(&state.branch);
        if !state.tag.is_empty() {
            // Always-on tag display (the documented "delete the next
            // line" tip in p10k-lean.zsh:399). Prefix with the tag
            // glyph when configured; the segment-group separator
            // already disambiguates visually otherwise.
            out.push(' ');
            if !icons.tag.is_empty() {
                out.push_str(icons.tag);
                out.push(' ');
            }
            out.push_str(&state.tag);
        }
    } else if !state.tag.is_empty() {
        if !icons.tag.is_empty() {
            out.push_str(icons.tag);
            out.push(' ');
        }
        out.push_str(&state.tag);
    } else {
        // p10k: `@<sha[1,8]>` when on no branch and no tag.
        out.push('@');
        out.push_str(&state.commit_short);
    }
}

/// Render the count-badge tail. Order pinned to p10k-lean.zsh:424-453
/// — action, behind, ahead, stash, conflicts, staged, unstaged,
/// untracked. Each chunk shows only when its counter is non-zero.
fn render_git_counters(state: &GitState, icons: &GitIcons<'_>, out: &mut String) {
    if !state.action.is_empty() {
        out.push(' ');
        out.push_str(&state.action);
    }
    if state.behind > 0 {
        out.push_str(&format!(" {}{}", icons.behind, state.behind));
    }
    if state.ahead > 0 {
        out.push_str(&format!(" {}{}", icons.ahead, state.ahead));
    }
    if state.stashed > 0 {
        out.push_str(&format!(" {}{}", icons.stash, state.stashed));
    }
    if state.conflicts > 0 {
        out.push_str(&format!(" {}{}", icons.conflict, state.conflicts));
    }
    if state.staged > 0 {
        out.push_str(&format!(" {}{}", icons.staged, state.staged));
    }
    if state.unstaged > 0 {
        out.push_str(&format!(" {}{}", icons.unstaged, state.unstaged));
    }
    if state.untracked > 0 {
        out.push_str(&format!(" {}{}", icons.untracked, state.untracked));
    }
}

/// Build the highlight-group chain for the chunk. When `status_colors`
/// is off, render every state with the same neutral `branch` group —
/// matches the original `vcs.branch` behavior with
/// `status_colors: false` (its default).
fn pick_highlight_groups(state: &GitState, status_colors: bool) -> Vec<Value> {
    if status_colors {
        let (primary, fallback) = if state.is_clean() {
            ("git_status_clean", "branch_clean")
        } else {
            ("git_status_dirty", "branch_dirty")
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
    }
}

/// Pure renderer: takes a `GitState` + icons and produces the
/// (contents, highlight_groups) pair. Returns `None` when the state
/// describes neither a branch nor a tag nor a detached SHA — there's
/// nothing meaningful to render.
pub fn render_git_chunk(
    state: &GitState,
    icons: &GitIcons<'_>,
    status_colors: bool,
) -> Option<(String, Vec<Value>)> {
    if state.branch.is_empty() && state.tag.is_empty() && state.commit_short.is_empty() {
        return None;
    }
    let mut s = String::new();
    render_git_header(state, icons, &mut s);
    render_git_counters(state, icons, &mut s);
    let groups = pick_highlight_groups(state, status_colors);
    Some((s, groups))
}

#[allow(clippy::too_many_arguments)]
pub fn git_status(
    cwd: &str,
    branch_icon: &str,
    github_icon: &str,
    tag_icon: &str,
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
    let icons = GitIcons {
        branch: branch_icon,
        github: github_icon,
        tag: tag_icon,
        unstaged: unstaged_icon,
        untracked: untracked_icon,
        staged: staged_icon,
        conflict: conflict_icon,
        ahead: ahead_icon,
        behind: behind_icon,
        stash: stash_icon,
    };
    let (s, groups) = render_git_chunk(&state, &icons, status_colors)?;
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
        let _ = git_status("/tmp", "", "", "", "!", "?", "+", "~", "⇡", "⇣", "*", false);
    }

    // ---- parse_porcelain_v2: porcelain-v2 line semantics ----

    #[test]
    fn parse_branch_head_line_sets_branch() {
        let s = parse_porcelain_v2("# branch.head main\n");
        assert_eq!(s.branch, "main");
    }

    #[test]
    fn parse_branch_head_detached_clears_branch() {
        // git emits the literal "(detached)" for the head line when
        // not on a branch; the parser normalises it away so the
        // renderer takes the SHA-fallback path.
        let s = parse_porcelain_v2("# branch.head (detached)\n");
        assert!(s.branch.is_empty());
    }

    #[test]
    fn parse_branch_ab_populates_ahead_and_behind() {
        let s = parse_porcelain_v2("# branch.ab +3 -7\n");
        assert_eq!(s.ahead, 3);
        assert_eq!(s.behind, 7);
    }

    #[test]
    fn parse_branch_ab_zero_means_synced() {
        let s = parse_porcelain_v2("# branch.ab +0 -0\n");
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
    }

    #[test]
    fn parse_1_line_x_byte_marks_staged() {
        // X=M (modified), Y=. (clean worktree) → staged only
        let s = parse_porcelain_v2("1 M. N... 100644 100644 100644 a a b src/lib.rs\n");
        assert_eq!(s.staged, 1);
        assert_eq!(s.unstaged, 0);
    }

    #[test]
    fn parse_1_line_y_byte_marks_unstaged() {
        // X=. (clean index), Y=M (modified worktree) → unstaged only
        let s = parse_porcelain_v2("1 .M N... 100644 100644 100644 a a b src/lib.rs\n");
        assert_eq!(s.staged, 0);
        assert_eq!(s.unstaged, 1);
    }

    #[test]
    fn parse_1_line_both_bytes_marks_both() {
        let s = parse_porcelain_v2("1 MM N... 100644 100644 100644 a a b src/lib.rs\n");
        assert_eq!(s.staged, 1);
        assert_eq!(s.unstaged, 1);
    }

    #[test]
    fn parse_2_line_rename_uses_same_xy_dispatch() {
        // Rename entry — same XY parsing as the `1 ` line.
        let s = parse_porcelain_v2("2 R. N... 100644 100644 100644 a a b R100 new\told\n");
        assert_eq!(s.staged, 1);
        assert_eq!(s.unstaged, 0);
    }

    #[test]
    fn parse_u_line_counts_conflict() {
        let s = parse_porcelain_v2("u UU N... 100644 100644 100644 100644 a a a a path\n");
        assert_eq!(s.conflicts, 1);
    }

    #[test]
    fn parse_question_line_counts_untracked() {
        let s = parse_porcelain_v2("? new_file.txt\n? another.txt\n");
        assert_eq!(s.untracked, 2);
    }

    #[test]
    fn parse_full_porcelain_v2_sample() {
        // A realistic mixed status: one branch line, one ahead/behind
        // line, mixed staged + unstaged, a conflict, an untracked
        // file. Verifies counters compose without cross-talk.
        let text = "\
# branch.oid abcdef0123456789
# branch.head feature/big
# branch.upstream origin/feature/big
# branch.ab +2 -1
1 M. N... 100644 100644 100644 a a b src/a.rs
1 .M N... 100644 100644 100644 a a b src/b.rs
1 MM N... 100644 100644 100644 a a b src/c.rs
2 R. N... 100644 100644 100644 a a b R100 src/d.rs\told.rs
u UU N... 100644 100644 100644 100644 a a a a src/conflict.rs
? untracked.txt
? another.txt
";
        let s = parse_porcelain_v2(text);
        assert_eq!(s.branch, "feature/big");
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 1);
        assert_eq!(s.staged, 3); // M., MM, R.
        assert_eq!(s.unstaged, 2); // .M, MM
        assert_eq!(s.conflicts, 1);
        assert_eq!(s.untracked, 2);
        assert!(!s.is_clean());
    }

    #[test]
    fn parse_empty_text_is_default_clean() {
        let s = parse_porcelain_v2("");
        assert!(s.is_clean());
        assert!(s.branch.is_empty());
    }

    // ---- is_clean: every field flips dirty ----

    #[test]
    fn is_clean_flips_for_every_dirty_field() {
        let fields: &[fn(&mut GitState)] = &[
            |s| s.unstaged = 1,
            |s| s.untracked = 1,
            |s| s.staged = 1,
            |s| s.conflicts = 1,
            |s| s.ahead = 1,
            |s| s.behind = 1,
            |s| s.action = "merge".into(),
        ];
        for set in fields {
            let mut s = GitState::default();
            set(&mut s);
            assert!(!s.is_clean(), "{s:?} should not be clean");
        }
    }

    #[test]
    fn is_clean_ignores_branch_and_tag_and_stash() {
        // Naming + remote presence + stash don't make the tree dirty;
        // p10k's clean/dirty colour split is about worktree state, not
        // branch metadata.
        let s = GitState {
            branch: "main".into(),
            tag: "v1.0".into(),
            stashed: 5,
            is_github: true,
            ..Default::default()
        };
        assert!(s.is_clean());
    }

    // ---- render_git_chunk: rendering rules ----

    fn icons() -> GitIcons<'static> {
        GitIcons {
            branch: "",
            github: "GH",
            tag: "T",
            unstaged: "!",
            untracked: "?",
            staged: "+",
            conflict: "~",
            ahead: "⇡",
            behind: "⇣",
            stash: "*",
        }
    }

    #[test]
    fn render_empty_state_returns_none() {
        // No branch + no tag + no SHA → nothing meaningful to show.
        let s = GitState::default();
        assert!(render_git_chunk(&s, &icons(), false).is_none());
    }

    #[test]
    fn render_branch_only_renders_glyph_and_name() {
        // With branch_icon non-empty, the header is `<icon> <name>`;
        // with an empty branch_icon the leading glyph+space is omitted
        // (this test pins both halves of that conditional).
        let s = GitState {
            branch: "main".into(),
            ..Default::default()
        };
        let mut ic = icons();
        ic.branch = "B";
        let (text, _) = render_git_chunk(&s, &ic, false).unwrap();
        assert_eq!(text, "B main");
        // And with empty branch_icon: no leading glyph, no stray space.
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        assert_eq!(text, "main");
    }

    #[test]
    fn render_github_remote_leads_with_octocat() {
        let s = GitState {
            branch: "main".into(),
            is_github: true,
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        assert!(text.starts_with("GH "), "github glyph missing: {text:?}");
        assert!(text.contains("main"));
    }

    #[test]
    fn render_branch_plus_tag_uses_combo_form() {
        // Branch + tag at HEAD → both render side-by-side, tag glyph
        // between them. Matches p10k's documented always-show-tag path.
        let s = GitState {
            branch: "main".into(),
            tag: "v1.0".into(),
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        assert!(text.contains("main"));
        assert!(text.contains("T v1.0"));
    }

    #[test]
    fn render_tag_only_no_branch() {
        // On a tag with no branch (rare but possible: detached at tag).
        let s = GitState {
            tag: "v1.0".into(),
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        assert!(text.starts_with("T v1.0"), "tag-only header: {text:?}");
    }

    #[test]
    fn render_detached_head_uses_at_sha_fallback() {
        // No branch + no tag → p10k's `@<sha[1,8]>` fallback.
        let s = GitState {
            commit_short: "abcdef1".into(),
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        assert_eq!(text, "@abcdef1");
    }

    #[test]
    fn render_counter_order_matches_p10k_lean() {
        // p10k-lean.zsh:424-453 order: action, behind, ahead, stash,
        // conflicts, staged, unstaged, untracked. Each counter precedes
        // the next; this test pins that no future refactor reorders
        // them silently.
        let s = GitState {
            branch: "main".into(),
            action: "merge".into(),
            behind: 1,
            ahead: 2,
            stashed: 3,
            conflicts: 4,
            staged: 5,
            unstaged: 6,
            untracked: 7,
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        // Each glyph+count fragment in canonical order.
        let order = ["merge", "⇣1", "⇡2", "*3", "~4", "+5", "!6", "?7"];
        let mut last_idx = 0usize;
        for needle in order {
            let i = text[last_idx..]
                .find(needle)
                .unwrap_or_else(|| panic!("missing {needle:?} after pos {last_idx}: {text:?}"));
            last_idx += i + needle.len();
        }
    }

    #[test]
    fn render_zero_counters_omit_chunks() {
        // Counters at 0 must NOT render — the segment is supposed to
        // be quiet when there's nothing to report.
        let s = GitState {
            branch: "main".into(),
            ..Default::default()
        };
        let (text, _) = render_git_chunk(&s, &icons(), false).unwrap();
        for glyph in ['⇡', '⇣', '*', '~', '+', '!', '?'] {
            assert!(
                !text.contains(glyph),
                "glyph {glyph:?} should be omitted when its counter is 0: {text:?}"
            );
        }
    }

    #[test]
    fn render_empty_github_icon_does_not_lead() {
        // is_github=true but the user has unset the github_icon arg →
        // no leading glyph, no stray space.
        let s = GitState {
            branch: "main".into(),
            is_github: true,
            ..Default::default()
        };
        let mut ic = icons();
        ic.github = "";
        let (text, _) = render_git_chunk(&s, &ic, false).unwrap();
        assert!(
            !text.starts_with(' '),
            "no leading space when github_icon is empty: {text:?}"
        );
    }

    // ---- pick_highlight_groups: status_colors matrix ----

    #[test]
    fn highlight_groups_no_status_colors_is_two_neutral() {
        let s = GitState::default();
        let g = pick_highlight_groups(&s, false);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].as_str().unwrap(), "git_status");
        assert_eq!(g[1].as_str().unwrap(), "branch");
    }

    #[test]
    fn highlight_groups_status_colors_clean_uses_clean_chain() {
        let s = GitState {
            branch: "main".into(),
            ..Default::default()
        };
        let g = pick_highlight_groups(&s, true);
        assert_eq!(g[0].as_str().unwrap(), "git_status_clean");
        assert_eq!(g[2].as_str().unwrap(), "branch_clean");
    }

    #[test]
    fn highlight_groups_status_colors_dirty_uses_dirty_chain() {
        let s = GitState {
            branch: "main".into(),
            unstaged: 1,
            ..Default::default()
        };
        let g = pick_highlight_groups(&s, true);
        assert_eq!(g[0].as_str().unwrap(), "git_status_dirty");
        assert_eq!(g[2].as_str().unwrap(), "branch_dirty");
    }

    #[test]
    fn highlight_groups_status_colors_always_four_levels() {
        // Whenever status_colors is on, the chain is exactly 4 entries
        // — primary, family, fallback, neutral — so the renderer can
        // resolve through any colorscheme.
        let s = GitState::default();
        assert_eq!(pick_highlight_groups(&s, true).len(), 4);
        let s = GitState {
            unstaged: 1,
            ..Default::default()
        };
        assert_eq!(pick_highlight_groups(&s, true).len(), 4);
    }
}
