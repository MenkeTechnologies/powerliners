// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/vcs/bzr.py`.
//!
//! Bazaar repository status segment. Upstream uses the Python `bzrlib`
//! library for the actual `bzr status` invocation; the Rust port
//! surfaces the data-shape (`nick_pat` regex, branch-name reader,
//! the dirty/untracked aggregation for `bzr status -S` output) and
//! stubs the actual bzrlib calls since adding a Rust bzr client is
//! out of scope.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:5
// from io import StringIO                          // py:7
// from bzrlib import (workingtree, status, library_state, trace, ui)                          // py:9
// from powerline.lib.vcs import get_branch_name, get_file_status                              // py:11
// from powerline.lib.path import join              // py:12
// from powerline.lib.encoding import get_preferred_file_contents_encoding                     // py:13

use regex::bytes::Regex as ByteRegex;
use std::sync::OnceLock;

/// Port of `nick_pat` from `powerline/lib/vcs/bzr.py:23`.
///
/// Python: `re.compile(br'nickname\s*=\s*(.+)')`.
pub fn nick_pat() -> &'static ByteRegex {
    static R: OnceLock<ByteRegex> = OnceLock::new();
    R.get_or_init(|| ByteRegex::new(r"^nickname\s*=\s*(.+)$").unwrap())
}

/// Port of `class CoerceIO(StringIO)` from
/// `powerline/lib/vcs/bzr.py:16`.
///
/// In Python this is a StringIO subclass that decodes bytes on
/// write(). The Rust port surfaces only the byte-decode step since
/// the StringIO buffer behaviour is delegated to whatever caller
/// owns the byte buffer.
pub struct CoerceIO {
    pub buffer: String,
}

impl Default for CoerceIO {
    fn default() -> Self {
        Self::new()
    }
}

impl CoerceIO {
    /// Constructs an empty CoerceIO.
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Port of `CoerceIO.write()` from
    /// `powerline/lib/vcs/bzr.py:17`.
    ///
    /// Decodes bytes via UTF-8 with replacement, then appends to the
    /// buffer (Python's super().write()).
    pub fn write(&mut self, arg: &[u8]) -> usize {
        // py:18-19  bytes → decode via preferred encoding, replace errors
        let s = String::from_utf8_lossy(arg);
        let n = s.len();
        // py:20  return super().write(arg)
        self.buffer.push_str(&s);
        n
    }
}

/// Port of `branch_name_from_config_file()` from
/// `powerline/lib/vcs/bzr.py:26`.
///
/// Reads `branch.conf`, returns the `nickname = ...` value if found,
/// otherwise falls back to `os.path.basename(directory)`.
pub fn branch_name_from_config_file(
    directory: &std::path::Path,
    config_file: &std::path::Path,
) -> String {
    // py:28-35  try open + iterate lines + nick_pat.match
    if let Ok(bytes) = std::fs::read(config_file) {
        for line in bytes.split(|&b| b == b'\n') {
            if let Some(c) = nick_pat().captures(line) {
                if let Some(m) = c.get(1) {
                    let s = String::from_utf8_lossy(m.as_bytes()).trim().to_string();
                    if !s.is_empty() {
                        return s;
                    }
                }
            }
        }
    }
    // py:37  ans or os.path.basename(directory)
    directory
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Port of `class Repository(object)` from
/// `powerline/lib/vcs/bzr.py:43`.
pub struct Repository {
    /// Python: `self.directory` — absolute path to repo root.
    pub directory: std::path::PathBuf,
    /// Python: `self.create_watcher` — see mercurial.rs note.
    pub create_watcher: (),
}

impl Repository {
    /// Port of `Repository.__init__()` from
    /// `powerline/lib/vcs/bzr.py:44`.
    pub fn new(directory: impl AsRef<std::path::Path>, create_watcher: ()) -> Self {
        // py:45-46  self.directory = os.path.abspath(...)
        let abs = std::fs::canonicalize(directory.as_ref())
            .unwrap_or_else(|_| directory.as_ref().to_path_buf());
        Self {
            directory: abs,
            create_watcher,
        }
    }

    /// Port of `Repository.status()` from
    /// `powerline/lib/vcs/bzr.py:48`.
    ///
    /// **Status:** stub for the bzrlib path. Returns None (clean).
    pub fn status(&self, _path: Option<&str>) -> Option<String> {
        // py:60-67  delegated to do_status which needs bzrlib
        None
    }

    /// Port of `Repository.do_status()` from
    /// `powerline/lib/vcs/bzr.py:70`.
    pub fn do_status(&self, _directory: &std::path::Path, _path: Option<&str>) -> Option<String> {
        // py:71-74  try _status; swallow exception
        None
    }

    /// Port of `Repository._status()` from
    /// `powerline/lib/vcs/bzr.py:75`.
    ///
    /// **Status:** stub. The Python implementation invokes
    /// `bzrlib.status.show_tree_status` and parses the `-S` output;
    /// adding a Rust bzrlib is out of scope.
    pub fn _status(&self, _directory: &std::path::Path, _path: Option<&str>) -> Option<String> {
        // py:75-95 stub
        None
    }

    /// Port of `Repository.branch()` from
    /// `powerline/lib/vcs/bzr.py:97`.
    pub fn branch(&self) -> String {
        // py:98-103  config_file = .bzr/branch/branch.conf
        let config_file = self
            .directory
            .join(".bzr")
            .join("branch")
            .join("branch.conf");
        branch_name_from_config_file(&self.directory, &config_file)
    }

    /// Parses `bzr status -S` raw output and aggregates the
    /// dirty/untracked state into the "DU"/"D "/" U"/None string.
    /// Equivalent to the loop body at
    /// `powerline/lib/vcs/bzr.py:87-93`.
    pub fn aggregate_short_status(raw: &str) -> Option<String> {
        // py:84-85  if not raw.strip(): return
        if raw.trim().is_empty() {
            return None;
        }
        // py:87-91  walk lines for dirty/untracked indicators
        let mut dirtied: char = ' ';
        let mut untracked: char = ' ';
        for line in raw.lines() {
            let bytes = line.as_bytes();
            // py:89  line[1] in 'ACDMRIN'
            if bytes.len() > 1 && b"ACDMRIN".contains(&bytes[1]) {
                dirtied = 'D';
            }
            // py:90-91  line[0] == '?'
            if !bytes.is_empty() && bytes[0] == b'?' {
                untracked = 'U';
            }
        }
        // py:92-93  return ans if ans.strip() else None
        let ans: String = format!("{}{}", dirtied, untracked);
        if ans.trim().is_empty() {
            None
        } else {
            Some(ans)
        }
    }

    /// Parses `bzr status -S` raw output to extract the
    /// per-file two-char status code. Equivalent to py:80-83:
    /// `ans = raw[:2]; if ans == 'I ': ans = None`.
    pub fn extract_file_status(raw: &str) -> Option<String> {
        if raw.trim().is_empty() {
            return None;
        }
        // py:80  ans = raw[:2]
        let ans: String = raw.chars().take(2).collect();
        // py:82-83  if ans == 'I ': ans = None
        if ans == "I " {
            None
        } else {
            Some(ans)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "powerliners-bzr-{}-{}",
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
    fn nick_pat_matches_simple_nickname_line() {
        // py:23  re.compile(br'nickname\s*=\s*(.+)')
        let m = nick_pat().captures(b"nickname = main-branch").unwrap();
        assert_eq!(&m[1], b"main-branch");
    }

    #[test]
    fn nick_pat_matches_with_extra_whitespace() {
        let m = nick_pat()
            .captures(b"nickname    =    my-feature  ")
            .unwrap();
        assert_eq!(&m[1], b"my-feature  ");
    }

    #[test]
    fn nick_pat_does_not_match_unrelated_line() {
        assert!(nick_pat().captures(b"# comment").is_none());
        assert!(nick_pat().captures(b"other = value").is_none());
    }

    #[test]
    fn coerce_io_write_decodes_bytes() {
        let mut io = CoerceIO::new();
        io.write(b"hello ");
        io.write(b"world");
        assert_eq!(io.buffer, "hello world");
    }

    #[test]
    fn coerce_io_write_handles_invalid_utf8() {
        let mut io = CoerceIO::new();
        // py: get_preferred_file_contents_encoding, errors='replace'
        // Invalid UTF-8 byte 0xFF gets replaced.
        io.write(&[b'a', 0xff, b'b']);
        assert!(io.buffer.contains('a'));
        assert!(io.buffer.contains('b'));
    }

    #[test]
    fn branch_name_extracts_nickname_from_config() {
        let d = tmp_dir();
        let f = d.join("branch.conf");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"# header\nnickname = feature-x\nother = ignored\n")
            .unwrap();
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, "feature-x");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_falls_back_to_directory_basename() {
        let d = tmp_dir();
        let basename = d.file_name().unwrap().to_string_lossy().to_string();
        let f = d.join("does-not-exist");
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, basename);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn branch_name_falls_back_when_no_nickname_line() {
        let d = tmp_dir();
        let basename = d.file_name().unwrap().to_string_lossy().to_string();
        let f = d.join("branch.conf");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"# only a comment\n").unwrap();
        let name = branch_name_from_config_file(&d, &f);
        assert_eq!(name, basename);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_new_canonicalizes_directory() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        assert!(repo.directory.is_absolute());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_branch_reads_branch_conf() {
        let d = tmp_dir();
        let branch_dir = d.join(".bzr").join("branch");
        std::fs::create_dir_all(&branch_dir).unwrap();
        let f = branch_dir.join("branch.conf");
        let mut h = std::fs::File::create(&f).unwrap();
        h.write_all(b"nickname = lp:foo\n").unwrap();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.branch(), "lp:foo");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_branch_falls_back_to_basename_when_no_conf() {
        let d = tmp_dir();
        let basename = d.file_name().unwrap().to_string_lossy().to_string();
        let repo = Repository::new(&d, ());
        // canonicalized directory may differ in basename; compare against
        // the canonical repo.directory's file_name
        let expected = repo
            .directory
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let actual = repo.branch();
        // Either matches the canonical basename or the pre-canon basename
        // depending on whether canonicalize prepended /private/ on macOS.
        assert!(actual == expected || actual == basename);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn repository_status_stub_returns_none() {
        let d = tmp_dir();
        let repo = Repository::new(&d, ());
        assert_eq!(repo.status(None), None);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn aggregate_short_status_empty_returns_none() {
        // py:84-85  if not raw.strip(): return
        assert_eq!(Repository::aggregate_short_status(""), None);
        assert_eq!(Repository::aggregate_short_status("  \n  "), None);
    }

    #[test]
    fn aggregate_short_status_modified_returns_d() {
        // py:89  line[1] in 'ACDMRIN'
        let raw = " M  file.txt\n";
        assert_eq!(
            Repository::aggregate_short_status(raw),
            Some("D ".to_string())
        );
    }

    #[test]
    fn aggregate_short_status_untracked_returns_u() {
        // py:90-91  line[0] == '?'
        let raw = "?   newfile.txt\n";
        assert_eq!(
            Repository::aggregate_short_status(raw),
            Some(" U".to_string())
        );
    }

    #[test]
    fn aggregate_short_status_both_returns_du() {
        let raw = " M  a.txt\n?   b.txt\n";
        assert_eq!(
            Repository::aggregate_short_status(raw),
            Some("DU".to_string())
        );
    }

    #[test]
    fn aggregate_short_status_only_clean_chars_returns_none() {
        // Lines with no dirty/untracked indicators → " " + " " = "  " → None
        let raw = "  \n  \n";
        assert_eq!(Repository::aggregate_short_status(raw), None);
    }

    #[test]
    fn extract_file_status_takes_first_two_chars() {
        // py:80  ans = raw[:2]
        assert_eq!(
            Repository::extract_file_status(" M file.txt\n"),
            Some(" M".to_string())
        );
        assert_eq!(
            Repository::extract_file_status("?  file.txt\n"),
            Some("? ".to_string())
        );
    }

    #[test]
    fn extract_file_status_ignored_returns_none() {
        // py:82-83  if ans == 'I ': ans = None
        assert_eq!(Repository::extract_file_status("I  file.txt\n"), None);
    }

    #[test]
    fn extract_file_status_empty_returns_none() {
        assert_eq!(Repository::extract_file_status(""), None);
        assert_eq!(Repository::extract_file_status("   "), None);
    }
}
