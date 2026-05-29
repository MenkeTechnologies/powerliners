// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/vcs.py`.
//!
//! VCS branch + stash segments. Surfaces the pure transformation
//! logic: branch-name lookup via the `guess()` repo dispatcher,
//! status-color classification, and stash-count formatting.
//!
//! The actual `guess` / `tree_status` resolution + `pl.exception`
//! logging are stubbed since they need the lib::vcs registry +
//! logger plumbing.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lib.vcs import guess, tree_status                                          // py:4
// from powerline.segments import Segment, with_docstring                                    // py:5
// from powerline.theme import requires_segment_info, requires_filesystem_watcher            // py:6

use serde_json::{json, Value};

/// Trait the segment uses to query repository state. The Python
/// source resolves a concrete subclass via `guess(path, ...)` from
/// `lib/vcs`; the Rust port factors out just the surface the
/// segments touch (`branch()`, `stash()`, `tree_status()`).
pub trait VcsRepository {
    /// Returns the current branch name (or short SHA in detached
    /// HEAD).
    fn branch(&self) -> String;
    /// Returns the stash count, or None when the backend doesn't
    /// support stash. Mirrors the Python `getattr(repo, 'stash',
    /// None)` probe at py:75.
    fn stash(&self) -> Option<u64> {
        None
    }
    /// Returns the repository's tree status (`"D "`, `" U"`, `"DU"`,
    /// or None). Equivalent to Python's `tree_status(repo, pl)`
    /// helper at py:23.
    fn tree_status(&self) -> Option<String> {
        None
    }
}

/// Port of `class BranchSegment(Segment)` from
/// `powerline/segments/common/vcs.py:9`.
///
/// Renders the current branch as a segment. When `status_colors=true`,
/// also picks the `branch_clean` / `branch_dirty` highlight based on
/// `tree_status`.
pub struct BranchSegment;

impl Default for BranchSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchSegment {
    /// Python class attribute: `divider_highlight_group = None`
    /// (py:10).
    pub const DIVIDER_HIGHLIGHT_GROUP: Option<&'static str> = None;

    pub fn new() -> Self {
        Self
    }

    /// Port of `BranchSegment.get_directory()` (staticmethod) from
    /// `powerline/segments/common/vcs.py:13`.
    ///
    /// Python: `segment_info['getcwd']()` — the caller provides a
    /// zero-arg getcwd closure on segment_info. Rust port takes the
    /// closure directly.
    pub fn get_directory<F>(getcwd: F) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        // py:15  return segment_info['getcwd']()
        getcwd()
    }

    /// Port of `BranchSegment.__call__()` from
    /// `powerline/segments/common/vcs.py:17`.
    ///
    /// Renders the branch segment for the given repository.
    /// `repo` is the resolved VCS repository (Python resolves via
    /// `guess(path=name, create_watcher=...)` at py:21); the Rust
    /// port takes the concrete repository through the trait.
    pub fn call<R: VcsRepository>(
        repo: Option<&R>,
        status_colors: bool,
        ignore_statuses: &[String],
    ) -> Option<Vec<Value>> {
        // py:18  def __call__(self, pl, segment_info, create_watcher, status_colors=False, ignore_statuses=()):
        // py:19  name = self.get_directory(segment_info)
        // py:20  if name:
        // py:21  repo = guess(path=name, create_watcher=create_watcher)
        // py:22  if repo is not None:
        let repo = repo?;
        // py:23  branch = repo.branch()
        let branch = repo.branch();
        // py:24  scol = ['branch']
        let mut scol: Vec<String> = vec!["branch".to_string()];
        // py:25  if status_colors:
        if status_colors {
            // py:26  try:
            // py:27  status = tree_status(repo, pl)
            let status = repo.tree_status();
            // py:28  except Exception as e:
            // py:29  pl.exception('Failed to compute tree status: {0}', str(e))
            // py:30  status = '?'
            // py:31  else:
            // py:32  status = status and status.strip()
            // py:33  if status in ignore_statuses:
            // py:34  status = None
            let effective = status.and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() || ignore_statuses.iter().any(|i| i == &trimmed) {
                    None
                } else {
                    Some(trimmed)
                }
            });
            // py:35  scol.insert(0, 'branch_dirty' if status else 'branch_clean')
            let group = if effective.is_some() {
                "branch_dirty"
            } else {
                "branch_clean"
            };
            scol.insert(0, group.to_string());
        }
        // py:36  return [{
        // py:37  'contents': branch,
        // py:38  'highlight_groups': scol,
        // py:39  'divider_highlight_group': self.divider_highlight_group,
        // py:40  }]
        Some(vec![json!({
            "contents": branch,
            "highlight_groups": scol,
            "divider_highlight_group": Self::DIVIDER_HIGHLIGHT_GROUP,
        })])
    }
}

/// Port of `class StashSegment(Segment)` from
/// `powerline/segments/common/vcs.py:64`.
///
/// Renders the stash count as a segment. Returns None when the
/// backend doesn't support stash or when the stash count is zero.
pub struct StashSegment;

impl Default for StashSegment {
    fn default() -> Self {
        Self::new()
    }
}

impl StashSegment {
    /// Python class attribute: `divider_highlight_group = None`
    /// (py:65).
    pub const DIVIDER_HIGHLIGHT_GROUP: Option<&'static str> = None;

    pub fn new() -> Self {
        Self
    }

    /// Port of `StashSegment.get_directory()` (staticmethod) from
    /// `powerline/segments/common/vcs.py:68`.
    pub fn get_directory<F>(getcwd: F) -> Option<String>
    where
        F: FnOnce() -> Option<String>,
    {
        getcwd()
    }

    /// Port of `StashSegment.__call__()` from
    /// `powerline/segments/common/vcs.py:72`.
    ///
    /// Renders the stash segment. Returns None when no repository,
    /// no stash support, or zero stashes.
    pub fn call<R: VcsRepository>(repo: Option<&R>) -> Option<Vec<Value>> {
        // py:70  def __call__(self, pl, segment_info, create_watcher):
        // py:71  name = self.get_directory(segment_info)
        // py:72  if name:
        // py:73  repo = guess(path=name, create_watcher=create_watcher)
        // py:74  if repo is not None:
        let repo = repo?;
        // py:75  stash = getattr(repo, 'stash', None)
        // py:76  if stash:
        // py:77  stashes = stash()
        let stashes = repo.stash()?;
        // py:78  if stashes:
        if stashes == 0 {
            return None;
        }
        // py:79  return [{
        // py:80  'contents': str(stashes),
        // py:81  'highlight_groups': ['stash'],
        // py:82  'divider_highlight_group': self.divider_highlight_group
        // py:83  }]
        Some(vec![json!({
            "contents": stashes.to_string(),
            "highlight_groups": ["stash"],
            "divider_highlight_group": Self::DIVIDER_HIGHLIGHT_GROUP,
        })])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeRepo {
        branch_name: String,
        stash_count: Option<u64>,
        status: Option<String>,
    }

    impl VcsRepository for FakeRepo {
        fn branch(&self) -> String {
            self.branch_name.clone()
        }
        fn stash(&self) -> Option<u64> {
            self.stash_count
        }
        fn tree_status(&self) -> Option<String> {
            self.status.clone()
        }
    }

    fn repo(branch: &str) -> FakeRepo {
        FakeRepo {
            branch_name: branch.to_string(),
            stash_count: None,
            status: None,
        }
    }

    #[test]
    fn branch_segment_divider_highlight_group_is_none() {
        // py:10  divider_highlight_group = None
        assert_eq!(BranchSegment::DIVIDER_HIGHLIGHT_GROUP, None);
    }

    #[test]
    fn branch_segment_get_directory_calls_getcwd_closure() {
        let r = BranchSegment::get_directory(|| Some("/repo".to_string()));
        assert_eq!(r, Some("/repo".to_string()));
    }

    #[test]
    fn branch_segment_get_directory_none_when_getcwd_returns_none() {
        let r = BranchSegment::get_directory(|| None);
        assert!(r.is_none());
    }

    #[test]
    fn branch_segment_no_repo_returns_none() {
        // py:21  if repo is not None: ...
        let r: Option<&FakeRepo> = None;
        let segments = BranchSegment::call(r, false, &[]);
        assert!(segments.is_none());
    }

    #[test]
    fn branch_segment_no_status_colors_returns_branch_highlight() {
        let r = repo("main");
        let segments = BranchSegment::call(Some(&r), false, &[]).unwrap();
        assert_eq!(segments[0]["contents"], "main");
        assert_eq!(segments[0]["highlight_groups"], json!(["branch"]));
    }

    #[test]
    fn branch_segment_with_status_colors_clean_picks_branch_clean() {
        // py:35  scol.insert(0, 'branch_dirty' if status else 'branch_clean')
        let r = FakeRepo {
            branch_name: "main".to_string(),
            stash_count: None,
            status: None,
        };
        let segments = BranchSegment::call(Some(&r), true, &[]).unwrap();
        // First group becomes branch_clean.
        assert_eq!(segments[0]["highlight_groups"][0], "branch_clean");
        assert_eq!(segments[0]["highlight_groups"][1], "branch");
    }

    #[test]
    fn branch_segment_with_status_colors_dirty_picks_branch_dirty() {
        let r = FakeRepo {
            branch_name: "main".to_string(),
            stash_count: None,
            status: Some("D ".to_string()),
        };
        let segments = BranchSegment::call(Some(&r), true, &[]).unwrap();
        assert_eq!(segments[0]["highlight_groups"][0], "branch_dirty");
    }

    #[test]
    fn branch_segment_ignore_statuses_strips_to_clean() {
        // py:31-34  status = status.strip(); if status in ignore_statuses: None
        // The comparison happens AFTER strip, so ignore_statuses entries
        // must match the trimmed form. Here " U" trims to "U".
        let r = FakeRepo {
            branch_name: "main".to_string(),
            stash_count: None,
            status: Some(" U".to_string()),
        };
        let segments = BranchSegment::call(Some(&r), true, &["U".to_string()]).unwrap();
        assert_eq!(segments[0]["highlight_groups"][0], "branch_clean");
    }

    #[test]
    fn branch_segment_empty_status_is_clean() {
        // py:31  status = status and status.strip() — empty → falsy → clean
        let r = FakeRepo {
            branch_name: "main".to_string(),
            stash_count: None,
            status: Some("   ".to_string()),
        };
        let segments = BranchSegment::call(Some(&r), true, &[]).unwrap();
        assert_eq!(segments[0]["highlight_groups"][0], "branch_clean");
    }

    #[test]
    fn branch_segment_emits_divider_highlight_group_as_null() {
        let r = repo("main");
        let segments = BranchSegment::call(Some(&r), false, &[]).unwrap();
        assert_eq!(segments[0]["divider_highlight_group"], Value::Null);
    }

    #[test]
    fn stash_segment_divider_highlight_group_is_none() {
        // py:65  divider_highlight_group = None
        assert_eq!(StashSegment::DIVIDER_HIGHLIGHT_GROUP, None);
    }

    #[test]
    fn stash_segment_no_repo_returns_none() {
        let r: Option<&FakeRepo> = None;
        assert!(StashSegment::call(r).is_none());
    }

    #[test]
    fn stash_segment_no_stash_support_returns_none() {
        // py:77  stash = getattr(repo, 'stash', None); if stash: ...
        let r = FakeRepo {
            branch_name: String::new(),
            stash_count: None,
            status: None,
        };
        assert!(StashSegment::call(Some(&r)).is_none());
    }

    #[test]
    fn stash_segment_zero_stashes_returns_none() {
        // py:79  if stashes: return [...] — 0 is falsy
        let r = FakeRepo {
            branch_name: String::new(),
            stash_count: Some(0),
            status: None,
        };
        assert!(StashSegment::call(Some(&r)).is_none());
    }

    #[test]
    fn stash_segment_positive_count_returns_segment() {
        let r = FakeRepo {
            branch_name: String::new(),
            stash_count: Some(3),
            status: None,
        };
        let segments = StashSegment::call(Some(&r)).unwrap();
        assert_eq!(segments[0]["contents"], "3");
        assert_eq!(segments[0]["highlight_groups"], json!(["stash"]));
    }

    #[test]
    fn stash_segment_get_directory_calls_closure() {
        let r = StashSegment::get_directory(|| Some("/repo".to_string()));
        assert_eq!(r, Some("/repo".to_string()));
    }
}
