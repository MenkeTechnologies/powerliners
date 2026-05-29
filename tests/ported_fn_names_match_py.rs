// vim:fileencoding=utf-8:noet
//! Allowlist enforcement: every `fn` defined under `src/ported/` must have a
//! name that exists in upstream powerline Python.
//!
//! Mirrors the zshrs `tests/ported_fn_names_match_c.rs` enforcement pattern.
//!
//! Sources of truth:
//!   - `docs/powerline_py_functions.txt` — names extracted from
//!     `vendor/powerline/powerline/**/*.py`
//!   - `tests/data/fake_fn_allowlist.txt` — maintainer-approved exemptions
//!
//! Exemptions (do NOT need to appear in either file):
//!   - trait-impl methods commonly required by Rust (`new`, `drop`, `fmt`,
//!     `clone`, `default`, `from`, `into`, `as_ref`, `deref`, `eq`, `hash`,
//!     `partial_cmp`, `cmp`, `next`, `poll`, `serialize`, `deserialize`)
//!   - functions inside `#[cfg(test)]` blocks or `mod tests { ... }` modules
//!
//! This test must remain load-bearing — weakening it is treated as
//! audit-tool tampering per the user's global rules.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const TRAIT_IMPL_EXEMPTIONS: &[&str] = &[
    "new",
    "drop",
    "fmt",
    "clone",
    "default",
    "from",
    "into",
    "as_ref",
    "deref",
    "eq",
    "hash",
    "partial_cmp",
    "cmp",
    "next",
    "poll",
    "serialize",
    "deserialize",
    "borrow",
    "borrow_mut",
    "as_mut",
    "try_from",
    "try_into",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_allowlist(path: &Path) -> HashSet<String> {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    raw.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// Extract top-level `fn NAME(` and `pub fn NAME(` names from a Rust source
/// file, excluding fns inside `#[cfg(test)]` / `mod tests` blocks.
///
/// This is a deliberately simple regex-style walker — not a full Rust parser —
/// chosen for zero-dependency, fast execution. It matches what the strict-port
/// audit cares about: function definitions that ship in the non-test build.
fn extract_fn_names(src: &str) -> Vec<(String, usize)> {
    let mut names = Vec::new();
    let mut brace_depth: i32 = 0;
    // Stack of (start_brace_depth, is_test_block) for each `mod tests` /
    // `#[cfg(test)]` scope we're currently inside.
    let mut test_scopes: Vec<i32> = Vec::new();
    let mut pending_test_attr = false;

    for (lineno, line) in src.lines().enumerate() {
        let trimmed = line.trim_start();

        // Detect cfg(test) attribute on the next item.
        if trimmed.starts_with("#[cfg(test)]") || trimmed.starts_with("#[cfg(any(test") {
            pending_test_attr = true;
            continue;
        }

        // Open `mod tests {` or `mod <X> {` immediately after a #[cfg(test)] attr.
        if pending_test_attr && trimmed.starts_with("mod ") && line.contains('{') {
            test_scopes.push(brace_depth);
            brace_depth += count_braces(line);
            pending_test_attr = false;
            continue;
        }
        // Also catch bare `mod tests {` without a #[cfg(test)] attr — common pattern.
        if trimmed.starts_with("mod tests") && line.contains('{') {
            test_scopes.push(brace_depth);
            brace_depth += count_braces(line);
            pending_test_attr = false;
            continue;
        }

        // If we're inside a test scope, skip fn extraction.
        if test_scopes.last().is_some() {
            brace_depth += count_braces(line);
            // Pop test scopes whose start_brace_depth is now >= brace_depth.
            while let Some(&start) = test_scopes.last() {
                if brace_depth <= start {
                    test_scopes.pop();
                } else {
                    break;
                }
            }
            continue;
        }

        // Extract fn NAME from `fn NAME(` or `pub fn NAME(` or `pub(crate) fn NAME(`.
        if let Some(name) = parse_fn_name(trimmed) {
            names.push((name, lineno + 1));
        }

        brace_depth += count_braces(line);
        pending_test_attr = false;
    }
    names
}

fn count_braces(line: &str) -> i32 {
    let mut in_string = false;
    let mut in_char = false;
    let mut prev_backslash = false;
    let mut delta: i32 = 0;
    for c in line.chars() {
        if prev_backslash {
            prev_backslash = false;
            continue;
        }
        if c == '\\' {
            prev_backslash = true;
            continue;
        }
        if in_string {
            if c == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if c == '\'' {
                in_char = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '\'' => in_char = true,
            '{' => delta += 1,
            '}' => delta -= 1,
            _ => {}
        }
    }
    delta
}

/// Parse `fn NAME(` from a line, returning NAME if present.
/// Skips `// ...` comments, `fn` inside a string, and lines that just
/// reference `fn` in a doc-comment.
fn parse_fn_name(trimmed: &str) -> Option<String> {
    // Skip doc-comments and line comments.
    if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("//") {
        return None;
    }
    // Find `fn ` token.
    let prefixes = ["pub fn ", "pub(crate) fn ", "pub(super) fn ", "fn "];
    let mut rest: Option<&str> = None;
    for p in prefixes {
        if let Some(after) = trimmed.strip_prefix(p) {
            rest = Some(after);
            break;
        }
        // Also handle `async fn` / `unsafe fn` / `const fn` / `extern "C" fn`.
        for kw in ["async ", "unsafe ", "const ", "extern \"C\" "] {
            let with_kw = format!("{p}{kw}");
            if let Some(after) = trimmed.strip_prefix(&with_kw) {
                rest = Some(after);
                break;
            }
        }
        if rest.is_some() {
            break;
        }
    }
    let rest = rest?;
    // Name ends at the first `(`, `<`, ` `, or `:`.
    let end = rest.find(['(', '<', ' ', ':']).unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

#[test]
fn every_ported_fn_name_exists_in_python_allowlist() {
    let root = repo_root();
    let py_allow = load_allowlist(&root.join("docs/powerline_py_functions.txt"));
    let fake_allow = load_allowlist(&root.join("tests/data/fake_fn_allowlist.txt"));
    let trait_allow: HashSet<&str> = TRAIT_IMPL_EXEMPTIONS.iter().copied().collect();

    let ported_root = root.join("src/ported");
    let mut violations: Vec<String> = Vec::new();
    let mut total_fns = 0usize;

    for entry in WalkDir::new(&ported_root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (name, lineno) in extract_fn_names(&src) {
            total_fns += 1;
            if py_allow.contains(&name) {
                continue;
            }
            if fake_allow.contains(&name) {
                continue;
            }
            if trait_allow.contains(name.as_str()) {
                continue;
            }
            let rel = path.strip_prefix(&root).unwrap_or(path);
            violations.push(format!(
                "  {}:{}  fn {}  — name not in docs/powerline_py_functions.txt and not in tests/data/fake_fn_allowlist.txt",
                rel.display(), lineno, name
            ));
        }
    }

    if !violations.is_empty() {
        panic!(
            "\n{} invented fn name(s) found in src/ported/ (out of {} total fns):\n{}\n\
             \nFix options:\n\
             1. Rename to a fn that exists in vendor/powerline/powerline/ (verify with `grep -xF '<name>' docs/powerline_py_functions.txt`).\n\
             2. If the name MUST be Rust-only, get maintainer approval per docs/PORT.md Rule 0, then add to tests/data/fake_fn_allowlist.txt with the approval date in the commit message.\n",
            violations.len(), total_fns,
            violations.join("\n")
        );
    }

    // Sanity: at least the two exemplar ports must show up.
    assert!(
        total_fns >= 2,
        "test discovered only {} fns; expected at least 2 (get_version, humanize_bytes)",
        total_fns
    );
}

#[test]
fn allowlist_files_are_well_formed() {
    let root = repo_root();
    let py_allow = load_allowlist(&root.join("docs/powerline_py_functions.txt"));
    let fake_allow = load_allowlist(&root.join("tests/data/fake_fn_allowlist.txt"));

    assert!(py_allow.len() >= 500,
        "powerline_py_functions.txt has only {} entries; expected 500+ (upstream has 624 unique fn names)",
        py_allow.len());

    // No name should appear in both lists — fake_allow is the exemption list,
    // and entries that are already legal don't belong there.
    let overlap: Vec<&String> = fake_allow
        .iter()
        .filter(|n| py_allow.contains(*n))
        .collect();
    assert!(
        overlap.is_empty(),
        "names in BOTH py-allowlist and fake-allowlist (drop from fake): {:?}",
        overlap
    );
}
