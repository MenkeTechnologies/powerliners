// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/vcs/git.py`.
//!
//! Git repository status segment. Upstream has two backends: a
//! `pygit2` C-library path (py:96-160) and a shell-out-to-`git`
//! fallback (py:161-210). The Rust port surfaces:
//!
//! * `_ref_pat` regex and `branch_name_from_config_file` for parsing
//!   `.git/HEAD`
//! * `git_directory(directory)` for resolving the `gitdir: ...`
//!   pointer in worktree `.git` files
//! * `GitRepository` base + `Repository` shell-out backend with
//!   `aggregate_porcelain_status` parsing the
//!   `git status --porcelain` lines into the
//!   `wt_column + index_column + untracked_column` triple
//!
//! The pygit2 backend (py:96-160) and the actual shell exec
//! (`_gitcmd`) are stubbed — adding a libgit2 binding or wiring the
//! shell exec through Rust's Command is out of scope for this pass.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// from powerline.lib.vcs import get_branch_name, get_file_status                              // py:7
// from powerline.lib.shell import readlines        // py:8
// from powerline.lib.path import join              // py:9
// from powerline.lib.encoding import ...           // py:10-11
// from powerline.lib.shell import which            // py:12

use regex::bytes::Regex as ByteRegex;
use std::sync::OnceLock;

/// Port of `_ref_pat` from `powerline/lib/vcs/git.py:15`.
///
/// Matches `ref: refs/heads/<branch>` headers in `.git/HEAD`.
#[allow(non_snake_case)]
pub fn _ref_pat() -> &'static ByteRegex {
    static R: OnceLock<ByteRegex> = OnceLock::new();
    R.get_or_init(|| ByteRegex::new(r"^ref:\s*refs/heads/(.+)$").unwrap())
}

/// Port of `branch_name_from_config_file()` from
/// `powerline/lib/vcs/git.py:18`.
///
/// Reads `.git/HEAD`, returns the symbolic-ref branch name if
/// present, otherwise the first 7 chars of the file (detached-HEAD
/// short SHA). Falls back to `os.path.basename(directory)` on read
/// error.
pub fn branch_name_from_config_file(
    directory: &std::path::Path,
    config_file: &std::path::Path,
) -> String {
    // py:19-22  try open + read
    let raw = match std::fs::read(config_file) {
        Ok(b) => b,
        // py:23  return os.path.basename(directory) on EnvironmentError
        Err(_) => {
            return directory
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
        }
    };
    // py:24-26  _ref_pat.match → symbolic ref
    if let Some(c) = _ref_pat().captures(raw.split(|&b| b == b'\n').next().unwrap_or(&[])) {
        if let Some(m) = c.get(1) {
            return String::from_utf8_lossy(m.as_bytes()).trim().to_string();
        }
    }
    // py:27  return raw[:7]  (detached-HEAD short SHA)
    let head: Vec<u8> = raw.iter().take(7).copied().collect();
    String::from_utf8_lossy(&head).to_string()
}

/// Port of `git_directory()` from `powerline/lib/vcs/git.py:30`.
///
/// Resolves the path to the real `.git` directory: returns the
/// directory itself if `directory/.git` is a directory, or follows
/// the `gitdir: <path>` pointer when it's a file (worktree case).
pub fn git_directory(directory: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    // py:31  path = join(directory, '.git')
    let path = directory.join(".git");
    // py:32  if os.path.isfile(path)
    if path.is_file() {
        // py:33-34  read raw
        let raw = std::fs::read(&path)?;
        // py:35-36  if not raw.startswith(b'gitdir: '): raise
        if !raw.starts_with(b"gitdir: ") {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid gitfile format",
            ));
        }
        // py:37-38  raw = raw[8:]
        let raw = &raw[8..];
        // py:39-40  strip trailing \n
        let raw = if raw.last() == Some(&b'\n') {
            &raw[..raw.len() - 1]
        } else {
            raw
        };
        // py:41-43  decode + verify non-empty
        let s = String::from_utf8_lossy(raw).to_string();
        if s.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no path in gitfile",
            ));
        }
        // py:44  return os.path.abspath(os.path.join(directory, raw))
        let joined = directory.join(&s);
        std::fs::canonicalize(&joined).or(Ok(joined))
    } else {
        // py:46  return path
        Ok(path)
    }
}

/// Port of `class GitRepository(object)` from
/// `powerline/lib/vcs/git.py:49`.
#[derive(Debug)]
pub struct GitRepository {
    /// Python: `self.directory` — absolute path to repo root.
    pub directory: std::path::PathBuf,
    /// Python: `self.create_watcher` — see mercurial.rs note.
    pub create_watcher: (),
}

impl GitRepository {
    /// Port of `GitRepository.__init__()` from
    /// `powerline/lib/vcs/git.py:52`.
    pub fn new(directory: impl AsRef<std::path::Path>, create_watcher: ()) -> Self {
        // py:53-54  self.directory = os.path.abspath(...)
        let abs = std::fs::canonicalize(directory.as_ref())
            .unwrap_or_else(|_| directory.as_ref().to_path_buf());
        Self {
            directory: abs,
            create_watcher,
        }
    }

    /// Port of `GitRepository.branch()` from
    /// `powerline/lib/vcs/git.py:83`.
    pub fn branch(&self) -> String {
        // py:84  directory = git_directory(self.directory)
        let dir = git_directory(&self.directory).unwrap_or_else(|_| self.directory.join(".git"));
        // py:85  head = join(directory, 'HEAD')
        let head = dir.join("HEAD");
        // py:86-91  get_branch_name(...)
        branch_name_from_config_file(&dir, &head)
    }
}

/// Port of `class Repository(GitRepository)` shell-out backend
/// from `powerline/lib/vcs/git.py:161`.
///
/// The pygit2 backend at py:96-160 is omitted (no libgit2 binding
/// here); this struct mirrors the fallback that shells out to `git`.
#[derive(Debug)]
pub struct Repository {
    pub base: GitRepository,
}

impl Repository {
    /// Port of `Repository.__init__()` from
    /// `powerline/lib/vcs/git.py:163`.
    ///
    /// Python raises `OSError` when `git` isn't on `$PATH`. Rust port
    /// surfaces this as `Err(io::Error::NotFound)`.
    pub fn new(
        directory: impl AsRef<std::path::Path>,
        create_watcher: (),
    ) -> std::io::Result<Self> {
        // py:164-165  if not which('git'): raise
        if which_exists("git").is_none() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "git executable is not available",
            ));
        }
        Ok(Self {
            base: GitRepository::new(directory, create_watcher),
        })
    }

    /// Port of `Repository.ignore_event()` (staticmethod) from
    /// `powerline/lib/vcs/git.py:170`.
    ///
    /// `.git/index.lock` updates happen frequently and don't indicate
    /// a working-tree change; the watcher should ignore them.
    pub fn ignore_event(path: &str, name: &str) -> bool {
        // py:174-175  path.endswith('.git') and name == 'index.lock'
        path.ends_with(".git") && name == "index.lock"
    }

    /// Aggregates `git status --porcelain` lines into the
    /// `wt_column + index_column + untracked_column` triple per
    /// `powerline/lib/vcs/git.py:194-208` (shell-out backend) and the
    /// equivalent pygit2 branch at py:141-159.
    pub fn aggregate_porcelain_status(lines: &[&str]) -> Option<String> {
        // py:194-196  wt_column = index_column = untracked_column = ' '
        let mut wt_column: char = ' ';
        let mut index_column: char = ' ';
        let mut untracked_column: char = ' ';
        for line in lines {
            let bytes = line.as_bytes();
            // py:198-200  line[0] == '?' → untracked
            if !bytes.is_empty() && bytes[0] == b'?' {
                untracked_column = 'U';
                continue;
            }
            // py:201  line[0] == '!' → ignored, skip
            if !bytes.is_empty() && bytes[0] == b'!' {
                continue;
            }
            // py:203  line[0] != ' ' → index column dirty
            if !bytes.is_empty() && bytes[0] != b' ' {
                index_column = 'I';
            }
            // py:204-205  line[1] != ' ' → working tree dirty
            if bytes.len() > 1 && bytes[1] != b' ' {
                wt_column = 'D';
            }
        }
        // py:206-208  return r if r != '   ' else None
        let r: String = format!("{}{}{}", wt_column, index_column, untracked_column);
        if r == "   " {
            None
        } else {
            Some(r)
        }
    }

    /// Port of `Repository.do_status()` (shell-out backend) from
    /// `powerline/lib/vcs/git.py:179`.
    ///
    /// **Status:** stub for the actual shell-out path. Always returns
    /// None. The aggregation logic that consumes the output is
    /// available via `aggregate_porcelain_status()` for testing.
    pub fn do_status(&self, _directory: &std::path::Path, _path: Option<&str>) -> Option<String> {
        // py:179-193 stub for the shell exec
        None
    }

    /// Port of `Repository.stash()` (shell-out backend) from
    /// `powerline/lib/vcs/git.py:175`.
    pub fn stash(&self) -> usize {
        // py:176  sum(1 for _ in self._gitcmd(...))
        0
    }
}

/// Port of `which()` from `powerline/lib/shell.py:which`.
///
/// Returns `Some(path)` if the executable is on `$PATH`, else `None`.
fn which_exists(name: &str) -> Option<std::path::PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let full = dir.join(name);
        if full.is_file() {
            return Some(full);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-git-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn ref_pat_matches_symbolic_head() {
        let m = _ref_pat().captures(b"ref: refs/heads/main").unwrap();
        assert_eq!(&m[1], b"main");
    }

    #[test]
    fn ref_pat_matches_with_extra_whitespace() {
        let m = _ref_pat().captures(b"ref:   refs/heads/feature/x").unwrap();
        assert_eq!(&m[1], b"feature/x");
    }

    #[test]
    fn ref_pat_does_not_match_sha() {
        // Detached-HEAD content is a hex SHA, not a ref line.
        assert!(_ref_pat().captures(b"abc1234567890abcdef").is_none());
    }

    #[test]
    fn branch_name_from_symbolic_head() {
        let d = tmp_dir();
        let f = d.join("HEAD");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"ref: refs/heads/develop\n").unwrap();
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "develop");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_from_detached_head_returns_short_sha() {
        let d = tmp_dir();
        let f = d.join("HEAD");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"abcdef1234567890\n").unwrap();
        // py:27  return raw[:7]
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "abcdef1");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_missing_file_returns_basename() {
        let d = tmp_dir();
        let basename = d.file_name().unwrap().to_string_lossy().to_string();
        let f = d.join("does-not-exist");
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, basename);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_directory_returns_dot_git_when_it_is_a_directory() {
        let d = tmp_dir();
        let gitd = d.join(".git");
        std::fs::create_dir_all(&gitd).unwrap();
        let result = git_directory(&d).unwrap();
        assert_eq!(result, gitd);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_directory_follows_gitfile_pointer() {
        // worktree case: `.git` is a file containing `gitdir: <path>`.
        let d = tmp_dir();
        let target = d.join("realgit");
        std::fs::create_dir_all(&target).unwrap();
        let gitfile = d.join(".git");
        let mut h = std::fs::File::create(&gitfile).unwrap();
        h.write_all(b"gitdir: realgit\n").unwrap();
        let resolved = git_directory(&d).unwrap();
        // canonicalize may add /private/ on macOS — just verify the
        // tail name matches and the path is absolute.
        assert!(resolved.is_absolute());
        assert!(resolved.file_name().unwrap() == "realgit");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_directory_errors_on_invalid_gitfile() {
        let d = tmp_dir();
        let gitfile = d.join(".git");
        let mut h = std::fs::File::create(&gitfile).unwrap();
        h.write_all(b"not a gitdir pointer\n").unwrap();
        let r = git_directory(&d);
        assert!(r.is_err());
        let e = r.unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_directory_errors_on_empty_gitfile_pointer() {
        let d = tmp_dir();
        let gitfile = d.join(".git");
        let mut h = std::fs::File::create(&gitfile).unwrap();
        h.write_all(b"gitdir: \n").unwrap();
        let r = git_directory(&d);
        assert!(r.is_err());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_repository_new_canonicalizes() {
        let d = tmp_dir();
        let repo = GitRepository::new(&d, ());
        assert!(repo.directory.is_absolute());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn git_repository_branch_reads_head() {
        let d = tmp_dir();
        let gitd = d.join(".git");
        std::fs::create_dir_all(&gitd).unwrap();
        let head = gitd.join("HEAD");
        let mut h = std::fs::File::create(&head).unwrap();
        h.write_all(b"ref: refs/heads/trunk\n").unwrap();
        let repo = GitRepository::new(&d, ());
        assert_eq!(repo.branch(), "trunk");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_new_errors_when_git_not_on_path() {
        // Force PATH to a directory that definitely has no git binary.
        let saved = std::env::var_os("PATH");
        // SAFETY: tests run single-threaded for env mutation; brief mutation
        // followed by restore.
        unsafe {
            std::env::set_var("PATH", "/nonexistent-empty-dir-for-test");
        }
        let d = tmp_dir();
        let result = Repository::new(&d, ());
        if let Some(p) = saved {
            unsafe {
                std::env::set_var("PATH", p);
            }
        } else {
            unsafe {
                std::env::remove_var("PATH");
            }
        }
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn ignore_event_index_lock_is_ignored() {
        // py:174  path.endswith('.git') and name == 'index.lock'
        assert!(Repository::ignore_event("/repo/.git", "index.lock"));
    }

    #[test]
    fn ignore_event_other_files_not_ignored() {
        assert!(!Repository::ignore_event("/repo/.git", "HEAD"));
        assert!(!Repository::ignore_event("/repo/src", "index.lock"));
    }

    #[test]
    fn aggregate_porcelain_status_empty_returns_none() {
        let r = Repository::aggregate_porcelain_status(&[]);
        assert_eq!(r, None);
    }

    #[test]
    fn aggregate_porcelain_status_modified_workingtree_returns_d() {
        // py:204-205  line[1] != ' ' → wt dirty. Result is
        // {wt}{index}{untracked}; wt is the first column.
        // " M file.txt" → line[0]=' ' (index clean), line[1]='M' (wt dirty)
        // → wt='D' index=' ' untracked=' ' → "D  "
        let r = Repository::aggregate_porcelain_status(&[" M file.txt"]);
        assert_eq!(r, Some("D  ".to_string()));
    }

    #[test]
    fn aggregate_porcelain_status_modified_index_returns_i() {
        // "M  file.txt" → line[0]='M' (index dirty) line[1]=' ' (wt clean)
        // → wt=' ' index='I' untracked=' ' → " I "
        let r = Repository::aggregate_porcelain_status(&["M  file.txt"]);
        assert_eq!(r, Some(" I ".to_string()));
    }

    #[test]
    fn aggregate_porcelain_status_untracked_returns_u() {
        // py:198  line[0] == '?'
        let r = Repository::aggregate_porcelain_status(&["?? newfile.txt"]);
        // wt=' ' index=' ' untracked='U' → "  U"
        assert_eq!(r, Some("  U".to_string()));
    }

    #[test]
    fn aggregate_porcelain_status_ignored_line_does_not_change_state() {
        // py:201  line[0] == '!' → skip
        let r = Repository::aggregate_porcelain_status(&["!! ignored.txt"]);
        // Should yield no flags → None
        assert_eq!(r, None);
    }

    #[test]
    fn aggregate_porcelain_status_combined_index_wt_untracked() {
        let lines = ["MM both-dirty.txt", "?? untracked.txt"];
        let r = Repository::aggregate_porcelain_status(&lines);
        // line[0]='M' → index='I', line[1]='M' → wt='D'; ?? → untracked='U'
        // → "DIU"
        assert_eq!(r, Some("DIU".to_string()));
    }

    #[test]
    fn aggregate_porcelain_status_all_spaces_returns_none() {
        // No lines = " " + " " + " " = "   " → None
        let r = Repository::aggregate_porcelain_status(&[]);
        assert_eq!(r, None);
    }

    #[test]
    fn do_status_stub_returns_none() {
        // Skip if no git on path; test only the stub return.
        if which_exists("git").is_none() {
            return;
        }
        let d = tmp_dir();
        let repo = Repository::new(&d, ()).unwrap();
        assert_eq!(repo.do_status(&d, None), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn stash_stub_returns_zero() {
        if which_exists("git").is_none() {
            return;
        }
        let d = tmp_dir();
        let repo = Repository::new(&d, ()).unwrap();
        assert_eq!(repo.stash(), 0);
        std::fs::remove_dir_all(&d).ok();
    }
}
