// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/vcs/mercurial.py`.
//!
//! Mercurial repository status segment. Upstream uses the Python
//! `hglib` library for the actual `hg status` invocation; the Rust
//! port surfaces the data-shape (status code mapping + bitmask
//! aggregation) and the branch-name file reader. The `do_status`
//! shell-out is stubbed since adding a Rust hg client crate is out of
//! scope for this pass.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import hglib                                     // py:6
// from powerline.lib.vcs import get_branch_name, get_file_status                              // py:8
// from powerline.lib.path import join              // py:9
// from powerline.lib.encoding import get_preferred_file_contents_encoding                     // py:10

use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `branch_name_from_config_file()` from
/// `powerline/lib/vcs/mercurial.py:13`.
///
/// Reads `.hg/branch`, decodes via the preferred file-contents
/// encoding (UTF-8 with replace), strips whitespace. Returns
/// `"default"` on any read error.
pub fn branch_name_from_config_file(
    _directory: &std::path::Path,
    config_file: &std::path::Path,
) -> String {
    // py:13  def branch_name_from_config_file(directory, config_file):
    // py:14  try:
    // py:15  with open(config_file, 'rb') as f:
    // py:16  raw = f.read()
    // py:17  return raw.decode(get_preferred_file_contents_encoding(), 'replace').strip()
    // py:18  except Exception:
    // py:19  return 'default'
    match std::fs::read(config_file) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).trim().to_string(),
        Err(_) => "default".to_string(),
    }
}

/// Per-file status code returned by `Repository::status(path)`.
///
/// Mirrors the first element of the Python tuple at
/// `powerline/lib/vcs/mercurial.py:26-28`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    /// Modified
    M,
    /// Added
    A,
    /// Removed
    R,
    /// Deleted from filesystem but still tracked
    D,
    /// Unknown
    U,
    /// Ignored
    I,
    /// Clean (no flag)
    Clean,
}

impl FileStatus {
    /// Returns the single-character display code Python emits via
    /// `self.statuses[status][0]`.
    pub fn as_char(&self) -> &'static str {
        match self {
            FileStatus::M => "M",
            FileStatus::A => "A",
            FileStatus::R => "R",
            FileStatus::D => "D",
            FileStatus::U => "U",
            FileStatus::I => "I",
            FileStatus::Clean => "",
        }
    }
}

/// Port of `Repository.statuses` from
/// `powerline/lib/vcs/mercurial.py:26-28`.
///
/// Maps the `hg status` single-byte code to (file_status, repo_bit)
/// where repo_bit is the bitfield contribution for aggregate
/// repository status.
pub fn statuses() -> &'static HashMap<u8, (FileStatus, u8)> {
    static M: OnceLock<HashMap<u8, (FileStatus, u8)>> = OnceLock::new();
    M.get_or_init(|| {
        let mut t = HashMap::new();
        // py:26-27  M/A/R/!  → 1-bit (dirty)
        t.insert(b'M', (FileStatus::M, 1));
        t.insert(b'A', (FileStatus::A, 1));
        t.insert(b'R', (FileStatus::R, 1));
        t.insert(b'!', (FileStatus::D, 1));
        // py:27  ?  → 2-bit (untracked)
        t.insert(b'?', (FileStatus::U, 2));
        // py:28  I  → 0-bit (ignored)
        t.insert(b'I', (FileStatus::I, 0));
        // py:28  C  → 0-bit (clean)
        t.insert(b'C', (FileStatus::Clean, 0));
        t
    })
}

/// Port of `Repository.repo_statuses_str` from
/// `powerline/lib/vcs/mercurial.py:29`.
///
/// Indexed by the 2-bit aggregate (0..=3):
///   0 → clean, 1 → dirty, 2 → untracked, 3 → both.
pub const REPO_STATUSES_STR: [Option<&str>; 4] = [None, Some("D "), Some(" U"), Some("DU")];

/// Port of `class Repository(object)` from
/// `powerline/lib/vcs/mercurial.py:22`.
pub struct Repository {
    /// Python: `self.directory` — absolute path to repo root.
    pub directory: std::path::PathBuf,
    /// Python: `self.create_watcher` — typed as a generic factory in
    /// Rust since the watcher trait lives in the unported watcher
    /// module.
    pub create_watcher: (),
}

impl Repository {
    /// Port of `Repository.__init__()` from
    /// `powerline/lib/vcs/mercurial.py:31`.
    pub fn new(directory: impl AsRef<std::path::Path>, create_watcher: ()) -> Self {
        // py:32-33  self.directory = os.path.abspath(...)
        let abs = std::fs::canonicalize(directory.as_ref())
            .unwrap_or_else(|_| directory.as_ref().to_path_buf());
        Self {
            directory: abs,
            create_watcher,
        }
    }

    /// Port of `Repository.status()` from
    /// `powerline/lib/vcs/mercurial.py:39`.
    ///
    /// Repository status (path=None) returns one of `Some("D ")`,
    /// `Some(" U")`, `Some("DU")`, `None`. The actual hg invocation
    /// is stubbed; returns `None` (clean) without watcher / dirstate
    /// integration.
    pub fn status(&self, _path: Option<&str>) -> Option<String> {
        // py:41  def status(self, path=None):
        // py:42-53  docstring
        // py:54  if path:
        // py:55  return get_file_status(
        // py:56  directory=self.directory,
        // py:57  dirstate_file=join(self.directory, '.hg', 'dirstate'),
        // py:58  file_path=path,
        // py:59  ignore_file_name='.hgignore',
        // py:60  get_func=self.do_status,
        // py:61  create_watcher=self.create_watcher,
        // py:62  )
        // py:63  return self.do_status(self.directory, path)
        None
    }

    /// Port of `Repository.do_status()` from
    /// `powerline/lib/vcs/mercurial.py:66`.
    ///
    /// **Status:** stub. The Python implementation shells out via
    /// `hglib.open(directory)`; adding a Rust hg client is out of
    /// scope. Always returns `None` (clean).
    pub fn do_status(&self, _directory: &std::path::Path, _path: Option<&str>) -> Option<String> {
        // py:65  def do_status(self, directory, path):
        // py:66  with self._repo(directory) as repo:
        // py:67  if path:
        // py:68  path = os.path.join(directory, path)
        // py:69  statuses = repo.status(include=path, all=True)
        // py:70  for status, paths in statuses:
        // py:71  if paths:
        // py:72  return self.statuses[status][0]
        // py:73  return None
        // py:74  else:
        // py:75  resulting_status = 0
        // py:76  for status, paths in repo.status(all=True):
        // py:77  if paths:
        // py:78  resulting_status |= self.statuses[status][1]
        // py:79  return self.repo_statuses_str[resulting_status]
        None
    }

    /// Port of `Repository.branch()` from
    /// `powerline/lib/vcs/mercurial.py:85`.
    ///
    /// Reads `.hg/branch` directly. The Python `get_branch_name`
    /// helper also wires up a watcher; we delegate to the file-read
    /// path since the watcher module isn't ported.
    pub fn branch(&self) -> String {
        // py:81  def branch(self):
        // py:82  config_file = join(self.directory, '.hg', 'branch')
        // py:83  return get_branch_name(
        // py:84  directory=self.directory,
        // py:85  config_file=config_file,
        // py:86  get_func=branch_name_from_config_file,
        // py:87  create_watcher=self.create_watcher,
        // py:88  )
        let config_file = self.directory.join(".hg").join("branch");
        branch_name_from_config_file(&self.directory, &config_file)
    }

    /// Aggregates an iterator of `hg status` byte codes into the
    /// repo-status bitmask using the `statuses` table. Equivalent to
    /// the loop body at `powerline/lib/vcs/mercurial.py:79-82`.
    pub fn aggregate_repo_status(codes: impl IntoIterator<Item = u8>) -> Option<&'static str> {
        let mut bits: u8 = 0;
        let table = statuses();
        for c in codes {
            if let Some((_, bit)) = table.get(&c) {
                bits |= bit;
            }
        }
        REPO_STATUSES_STR[bits as usize % 4]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-hg-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn statuses_table_matches_upstream() {
        let t = statuses();
        // py:26-27
        assert_eq!(t.get(&b'M'), Some(&(FileStatus::M, 1)));
        assert_eq!(t.get(&b'A'), Some(&(FileStatus::A, 1)));
        assert_eq!(t.get(&b'R'), Some(&(FileStatus::R, 1)));
        assert_eq!(t.get(&b'!'), Some(&(FileStatus::D, 1)));
        // py:27-28
        assert_eq!(t.get(&b'?'), Some(&(FileStatus::U, 2)));
        assert_eq!(t.get(&b'I'), Some(&(FileStatus::I, 0)));
        assert_eq!(t.get(&b'C'), Some(&(FileStatus::Clean, 0)));
    }

    #[test]
    fn repo_statuses_str_matches_upstream() {
        // py:29  repo_statuses_str = (None, 'D ', ' U', 'DU')
        assert_eq!(REPO_STATUSES_STR[0], None);
        assert_eq!(REPO_STATUSES_STR[1], Some("D "));
        assert_eq!(REPO_STATUSES_STR[2], Some(" U"));
        assert_eq!(REPO_STATUSES_STR[3], Some("DU"));
    }

    #[test]
    fn file_status_as_char_matches_upstream() {
        assert_eq!(FileStatus::M.as_char(), "M");
        assert_eq!(FileStatus::A.as_char(), "A");
        assert_eq!(FileStatus::R.as_char(), "R");
        assert_eq!(FileStatus::D.as_char(), "D");
        assert_eq!(FileStatus::U.as_char(), "U");
        assert_eq!(FileStatus::I.as_char(), "I");
        assert_eq!(FileStatus::Clean.as_char(), "");
    }

    #[test]
    fn branch_name_from_existing_file_returns_trimmed_content() {
        let d = tmp_dir();
        let f = d.join("branch");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"feature-xyz\n").unwrap();
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "feature-xyz");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_missing_file_returns_default() {
        let d = tmp_dir();
        let f = d.join("does-not-exist");
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "default");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_empty_file_returns_empty_string_after_strip() {
        let d = tmp_dir();
        let f = d.join("branch");
        std::fs::File::create(&f).unwrap();
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_new_canonicalizes_directory() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        // Canonicalized — should resolve to an absolute path.
        assert!(repo.directory.is_absolute());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_branch_reads_hg_branch_file() {
        let d = tmp_dir();
        let hg = d.join(".hg");
        std::fs::create_dir_all(&hg).unwrap();
        let f = hg.join("branch");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"main\n").unwrap();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.branch(), "main");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_branch_missing_returns_default() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.branch(), "default");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_status_repo_level_stub_returns_none() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.status(None), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_do_status_stub_returns_none() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.do_status(&d, None), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn aggregate_repo_status_clean_returns_none() {
        // No codes → 0 bits → None
        let r = Repository::aggregate_repo_status(std::iter::empty());
        assert_eq!(r, None);
    }

    #[test]
    fn aggregate_repo_status_modified_returns_dirty() {
        // M → 1-bit
        let r = Repository::aggregate_repo_status([b'M']);
        assert_eq!(r, Some("D "));
    }

    #[test]
    fn aggregate_repo_status_unknown_returns_untracked() {
        // ? → 2-bit
        let r = Repository::aggregate_repo_status([b'?']);
        assert_eq!(r, Some(" U"));
    }

    #[test]
    fn aggregate_repo_status_dirty_plus_untracked_returns_both() {
        // M + ? → 3 bits
        let r = Repository::aggregate_repo_status([b'M', b'?']);
        assert_eq!(r, Some("DU"));
    }

    #[test]
    fn aggregate_repo_status_clean_codes_ignored() {
        // I + C → 0 bits → None
        let r = Repository::aggregate_repo_status([b'I', b'C']);
        assert_eq!(r, None);
    }
}
