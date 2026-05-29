// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/powerline-release.py`.
//!
//! Maintenance script for cutting a new powerline release. Ports the
//! pure-functional helpers (`parse_version`, `setup_py_develop_filter`,
//! `setup_py_master_filter`, the `OVERLAY_*` constants) plus signatures
//! for the git/upload/ebuild stages so the script tree is structurally
//! complete. Actual git checkout / subprocess orchestration is deferred
//! since it's a maintenance tool, not part of the runtime path.

// #!/usr/bin/env python                              // sh:1
// import argparse, codecs, os, re                    // sh:5-8
// from subprocess import check_output, check_call, CalledProcessError  // sh:10
// from getpass import getpass                         // sh:11
// from github import Github                            // sh:13

/// Port of `OVERLAY_NAME` constant at
/// `vendor/powerline/scripts/powerline-release.py:16`.
pub const OVERLAY_NAME: &str = "raiagent";

/// Port of `OVERLAY` constant at
/// `vendor/powerline/scripts/powerline-release.py:17`.
pub const OVERLAY: &str = "leycec/raiagent";

/// Port of `OVERLAY_BRANCH_FORMAT` template at
/// `vendor/powerline/scripts/powerline-release.py:18`.
///
/// Returns the branch name for a given version string. Python uses
/// `.format(version)` on `'powerline-release-{0}'`.
pub fn overlay_branch_format(version: &str) -> String {
    // sh:18  'powerline-release-{0}'.format(version)
    format!("powerline-release-{}", version)
}

/// Port of `parse_version()` from
/// `vendor/powerline/scripts/powerline-release.py:21-48`.
///
/// Two modes:
/// - Plain dotted version: parses `"1.2.3"` as `vec![1, 2, 3]`.
/// - Plus-prefixed bump: `"+++"` increments the last component of the
///   existing latest tag's version; `"++"` bumps the second-to-last;
///   `"+"` bumps the first. The existing tag list is supplied via
///   `latest_version` so the fn stays pure (Python uses
///   `check_output(['git', 'tag', '-l', '[0-9]*.*'])`).
pub fn parse_version(s: &str, latest_version: Option<&[u32]>) -> Result<Vec<u32>, String> {
    // sh:22  if s == ('+' * len(s)):
    let all_plus = !s.is_empty() && s.chars().all(|c| c == '+');
    if all_plus {
        // sh:24-29  last_version = next(iter(sorted(... reverse=True)))
        let last = latest_version.ok_or_else(|| "No existing versions found".to_string())?;
        // sh:31  version = []
        let mut version: Vec<u32> = Vec::new();
        // sh:32-36  for i in range(len(s) - 1):
        //          try: version.append(last_version[i])
        //          except IndexError: version.append(0)
        for i in 0..s.len().saturating_sub(1) {
            version.push(*last.get(i).unwrap_or(&0));
        }
        // sh:38-41  try: version.append(last_version[len(s) - 1] + 1)
        //          except IndexError: version.append(1)
        let bump_idx = s.len() - 1;
        let next = last.get(bump_idx).map(|v| v + 1).unwrap_or(1);
        version.push(next);
        // sh:43-44  if len(version) == 1: version.append(0)
        if version.len() == 1 {
            version.push(0);
        }
        // sh:46  return tuple(version)
        Ok(version)
    } else {
        // sh:48  return tuple(map(int, s.split('.')))
        s.split('.')
            .map(|p| p.parse::<u32>().map_err(|e| e.to_string()))
            .collect()
    }
}

/// Port of `setup_py_develop_filter()` from
/// `vendor/powerline/scripts/powerline-release.py:62-65`.
///
/// Rewrites the `\tbase_version = '...'` line in `setup.py` to the
/// new version string. Other lines pass through unchanged.
pub fn setup_py_develop_filter(line: &str, version_string: &str) -> String {
    // sh:63  if line.startswith('\tbase_version = '):
    if line.starts_with("\tbase_version = ") {
        // sh:64  line = '\tbase_version = \'' + version_string + '\'\n'
        format!("\tbase_version = '{}'\n", version_string)
    } else {
        line.to_string()
    }
}

/// Port of `setup_py_master_filter()` from
/// `vendor/powerline/scripts/powerline-release.py:68-73`.
///
/// Rewrites two lines in `setup.py` for the master branch:
/// - `\tversion='...'` gets the new version string.
/// - Lines containing `Development Status` get pinned to
///   `Production/Stable`.
pub fn setup_py_master_filter(line: &str, version_string: &str) -> String {
    // sh:69  if line.startswith('\tversion='):
    if line.starts_with("\tversion=") {
        // sh:70  line = '\tversion=\'' + version_string + '\',\n'
        format!("\tversion='{}',\n", version_string)
    // sh:71  elif 'Development Status' in line:
    } else if line.contains("Development Status") {
        // sh:72  line = '\t\t\'Development Status :: 5 - Production/Stable\',\n'
        "\t\t'Development Status :: 5 - Production/Stable',\n".to_string()
    } else {
        line.to_string()
    }
}

/// Port of the `stages` tuple at
/// `vendor/powerline/scripts/powerline-release.py:206-212`.
///
/// Each entry is `(stage_name, stage_callable)`. Rust port returns
/// stage names only since the callables (`merge`, `push`, `upload`,
/// `create_ebuilds`, `update_overlay`) are subprocess-heavy
/// maintenance helpers — their dispatch shape is exposed as
/// `STAGE_NAMES` so the script's CLI arg surface can be wired up
/// without porting the bodies.
pub const STAGE_NAMES: &[&str] = &[
    // sh:207  ('merge', merge)
    "merge",
    // sh:208  ('push', push)
    "push",
    // sh:209  ('upload', upload)
    "upload",
    // sh:210  ('create_ebuilds', create_ebuilds)
    "create_ebuilds",
    // sh:211  ('update_overlay', update_overlay)
    "update_overlay",
];

/// Port of `create_release()` signature at
/// `vendor/powerline/scripts/powerline-release.py:215-221`.
///
/// Drives the release stages in order, skipping any whose names
/// aren't in `run_stages` (or running all when `run_stages` is None).
/// Stage bodies are deferred; this signature exists so callers can
/// wire the CLI through to it.
pub fn create_release(
    version: &[u32],
    _user: &str,
    _password: Option<&str>,
    run_stages: Option<&[&str]>,
) -> String {
    // sh:216  version_string = '.'.join(...)
    let version_string = version
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(".");
    // sh:217-218  if not password: password = getpass(...)
    // (deferred — release stages not yet wired)
    // sh:219-221  for stname, stfunc in stages: if run_stages is None or stname in run_stages: stfunc(...)
    for stname in STAGE_NAMES.iter() {
        let in_stages = match run_stages {
            None => true,
            Some(s) => s.contains(stname),
        };
        if in_stages {
            // Stage body deferred.
        }
    }
    version_string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_constants_pin_to_upstream() {
        // sh:16-17
        assert_eq!(OVERLAY_NAME, "raiagent");
        assert_eq!(OVERLAY, "leycec/raiagent");
    }

    #[test]
    fn overlay_branch_format_interpolates_version() {
        // sh:18
        assert_eq!(
            overlay_branch_format("2.7"),
            "powerline-release-2.7".to_string()
        );
    }

    #[test]
    fn parse_version_plain_dotted() {
        // sh:48  tuple(map(int, s.split('.')))
        assert_eq!(parse_version("1.2.3", None).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_version("2.0", None).unwrap(), vec![2, 0]);
    }

    #[test]
    fn parse_version_rejects_invalid_int() {
        assert!(parse_version("1.x.3", None).is_err());
    }

    #[test]
    fn parse_version_plus_bumps_major() {
        // sh:22-46  "+" → bump first component
        // "+" is len 1 so we iterate range(0) → nothing prepended,
        // then bump component[0]. Then len(version)==1 → append 0.
        let r = parse_version("+", Some(&[1, 2, 3])).unwrap();
        assert_eq!(r, vec![2, 0]);
    }

    #[test]
    fn parse_version_plusplus_bumps_minor() {
        // "++" len 2 → prepend component[0], then bump component[1]
        let r = parse_version("++", Some(&[1, 2, 3])).unwrap();
        assert_eq!(r, vec![1, 3]);
    }

    #[test]
    fn parse_version_plusplusplus_bumps_patch() {
        // "+++" len 3 → prepend [0,1], bump [2]
        let r = parse_version("+++", Some(&[1, 2, 3])).unwrap();
        assert_eq!(r, vec![1, 2, 4]);
    }

    #[test]
    fn parse_version_plus_no_latest_errors() {
        // sh:28-29  raise ValueError('No existing versions found')
        let r = parse_version("+", None);
        assert!(r.is_err());
    }

    #[test]
    fn setup_py_develop_filter_rewrites_base_version_line() {
        // sh:63-64
        let r = setup_py_develop_filter("\tbase_version = 'old'\n", "1.2.3");
        assert_eq!(r, "\tbase_version = '1.2.3'\n");
    }

    #[test]
    fn setup_py_develop_filter_passes_through_other_lines() {
        let r = setup_py_develop_filter("# comment\n", "1.2.3");
        assert_eq!(r, "# comment\n");
    }

    #[test]
    fn setup_py_master_filter_rewrites_version_line() {
        // sh:69-70
        let r = setup_py_master_filter("\tversion='old',\n", "1.2.3");
        assert_eq!(r, "\tversion='1.2.3',\n");
    }

    #[test]
    fn setup_py_master_filter_pins_development_status_line() {
        // sh:71-72
        let r = setup_py_master_filter("\t\t'Development Status :: 4 - Beta',\n", "1.2.3");
        assert_eq!(r, "\t\t'Development Status :: 5 - Production/Stable',\n");
    }

    #[test]
    fn setup_py_master_filter_passes_through_other_lines() {
        let r = setup_py_master_filter("\tname='powerline',\n", "1.2.3");
        assert_eq!(r, "\tname='powerline',\n");
    }

    #[test]
    fn stage_names_match_upstream_order() {
        // sh:206-212
        assert_eq!(
            STAGE_NAMES,
            &[
                "merge",
                "push",
                "upload",
                "create_ebuilds",
                "update_overlay"
            ]
        );
    }

    #[test]
    fn create_release_returns_dotted_version_string() {
        // sh:216  '.'.join(...)
        assert_eq!(create_release(&[1, 2, 3], "user", None, None), "1.2.3");
    }

    #[test]
    fn create_release_with_run_stages_subset() {
        // sh:219-220  if run_stages is None or stname in run_stages: stfunc(...)
        // We can't easily observe stage invocation since bodies are
        // deferred, but the call must succeed without panic.
        let r = create_release(&[2, 0], "u", None, Some(&["merge", "push"]));
        assert_eq!(r, "2.0");
    }
}
