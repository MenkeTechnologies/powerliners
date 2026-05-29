// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/env.py`.
//!
//! Environment-driven segments: environment-variable readout,
//! virtualenv detection, cwd breadcrumb, and current-user.
//!
//! Surfaces the pure transformations + the `_get_user` helper.
//! The live `psutil`/`pwd` dispatch is replaced by `libc::geteuid` +
//! `$USER` fallback since the Rust port doesn't need the psutil
//! pre/post-2.0.0 split.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from powerline.lib.unicode import out_u          // py:6
// from powerline.theme import requires_segment_info                                       // py:7
// from powerline.segments import Segment, with_docstring                                  // py:8

use serde_json::{json, Map, Value};

/// Port of `environment()` from
/// `powerline/segments/common/env.py:11`.
///
/// Returns the value of `variable` from the supplied environ map,
/// or `None` when absent.
pub fn environment(environ: &Map<String, Value>, variable: &str) -> Option<String> {
    // py:17  return segment_info['environ'].get(variable, None)
    environ
        .get(variable)
        .and_then(|v| v.as_str().map(String::from))
}

/// Port of `virtualenv()` from
/// `powerline/segments/common/env.py:21`.
///
/// Walks `VIRTUAL_ENV` then `CONDA_DEFAULT_ENV` in reverse path
/// order, returning the first segment that's neither empty nor in
/// `ignored_names`.
pub fn virtualenv(
    environ: &Map<String, Value>,
    ignore_venv: bool,
    ignore_conda: bool,
    ignored_names: &[&str],
) -> Option<String> {
    // py:31-34  VIRTUAL_ENV
    if !ignore_venv {
        let raw = environ
            .get("VIRTUAL_ENV")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        for candidate in raw.split('/').rev() {
            if !candidate.is_empty() && !ignored_names.contains(&candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    // py:35-37  CONDA_DEFAULT_ENV
    if !ignore_conda {
        let raw = environ
            .get("CONDA_DEFAULT_ENV")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        for candidate in raw.split('/').rev() {
            if !candidate.is_empty() && !ignored_names.contains(&candidate) {
                return Some(candidate.to_string());
            }
        }
    }
    // py:38  return None
    None
}

/// Port of `CwdSegment.get_shortened_path()` from
/// `powerline/segments/common/env.py:56`.
///
/// `cwd_result` is the result of `segment_info['getcwd']()` (Python
/// raises OSError(errno=2) when the directory is missing; Rust port
/// takes `Result<String, std::io::Error>` and surfaces the
/// `"[not found]"` sentinel when the error is `NotFound`).
pub fn get_shortened_path(
    cwd_result: std::io::Result<String>,
    home: Option<&str>,
    shorten_home: bool,
) -> Result<String, std::io::Error> {
    // py:58-65  try getcwd; except OSError errno=2: return '[not found]'
    let path = match cwd_result {
        Ok(p) => p,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok("[not found]".to_string());
        }
        Err(e) => return Err(e),
    };
    // py:66-71  shorten_home: path → ~ + suffix
    if shorten_home {
        if let Some(h) = home {
            if !h.is_empty() && path.starts_with(h) {
                return Ok(format!("~{}", &path[h.len()..]));
            }
        }
    }
    Ok(path)
}

/// Port of `CwdSegment.__call__()` from
/// `powerline/segments/common/env.py:74`.
///
/// Produces the breadcrumb-segment list. `dir_shorten_len`
/// truncates each non-leaf component to that many chars;
/// `dir_limit_depth` caps the depth and prepends `ellipsis`
/// when the path was truncated.
pub fn cwd_segments(
    cwd: &str,
    dir_shorten_len: Option<usize>,
    dir_limit_depth: Option<usize>,
    use_path_separator: bool,
    ellipsis: Option<&str>,
) -> Vec<Value> {
    let sep = std::path::MAIN_SEPARATOR;
    // py:80  cwd_split = cwd.split(os.sep)
    let cwd_split: Vec<&str> = cwd.split(sep).collect();
    let cwd_split_len = cwd_split.len();
    // py:82  cwd = [i[0:dir_shorten_len] ... for i in cwd_split[:-1]] + [last]
    let mut parts: Vec<String> = if cwd_split.is_empty() {
        Vec::new()
    } else {
        let last_idx = cwd_split_len - 1;
        let mut v: Vec<String> = cwd_split[..last_idx]
            .iter()
            .map(|i| {
                if let Some(n) = dir_shorten_len {
                    if !i.is_empty() && n > 0 {
                        i.chars().take(n).collect()
                    } else {
                        (*i).to_string()
                    }
                } else {
                    (*i).to_string()
                }
            })
            .collect();
        v.push(cwd_split[last_idx].to_string());
        v
    };
    // py:83-86  dir_limit_depth: trim + ellipsis
    if let Some(depth) = dir_limit_depth {
        if cwd_split_len > depth + 1 {
            let drop_count = parts.len() - depth;
            parts.drain(..drop_count);
            if let Some(e) = ellipsis {
                parts.insert(0, e.to_string());
            }
        }
    }
    // py:88-89  if not cwd[0]: cwd[0] = '/'
    if let Some(first) = parts.first_mut() {
        if first.is_empty() {
            *first = sep.to_string();
        }
    }
    // py:90-99  build segment list
    let draw_inner_divider = !use_path_separator;
    let mut ret: Vec<Value> = Vec::new();
    for part in &parts {
        if part.is_empty() {
            continue;
        }
        let contents = if use_path_separator {
            format!("{}{}", part, sep)
        } else {
            part.clone()
        };
        ret.push(json!({
            "contents": contents,
            "divider_highlight_group": "cwd:divider",
            "draw_inner_divider": draw_inner_divider,
        }));
    }
    // py:100  ret[-1]['highlight_groups'] = ['cwd:current_folder', 'cwd']
    if let Some(last) = ret.last_mut() {
        last["highlight_groups"] = json!(["cwd:current_folder", "cwd"]);
    }
    // py:101-105  use_path_separator post-processing
    if use_path_separator {
        if let Some(last) = ret.last_mut() {
            let s = last["contents"].as_str().unwrap_or("").to_string();
            let trimmed: String = s.chars().take(s.chars().count() - 1).collect();
            last["contents"] = json!(trimmed);
        }
        if ret.len() > 1 {
            let first = ret[0]["contents"].as_str().unwrap_or("").to_string();
            if let Some(first_char) = first.chars().next() {
                if first_char == sep {
                    let stripped: String = first.chars().skip(1).collect();
                    ret[0]["contents"] = json!(stripped);
                }
            }
        }
    }
    ret
}

/// Port of `_get_user()` from
/// `powerline/segments/common/env.py:131`.
///
/// Returns the current username via `$USER` env var.
/// Python tries psutil first, then `pwd.getpwuid(geteuid())`,
/// then `getpass.getuser`. The Rust port surfaces just the
/// env-var path since psutil/pwd aren't reachable from Rust.
pub fn _get_user(environ: &Map<String, Value>) -> Option<String> {
    environ
        .get("USER")
        .and_then(|v| v.as_str().map(String::from))
        .or_else(|| std::env::var("USER").ok())
}

/// Sentinel UUID Python source uses to bypass the live-username
/// dispatch during shell tests
/// (`powerline/segments/common/env.py:171`).
pub const POWERLINE_TEST_USER_UUID: &str = "ee5bcdc6-b749-11e7-9456-50465d597777";

/// Port of `class CwdSegment(Segment)` from
/// `powerline/segments/common/env.py:42`.
///
/// Marker struct holding the segment's introspection metadata. The
/// `__call__` body is the standalone `cwd_segments` fn; this struct
/// surfaces the `argspecobjs` and `omitted_args` introspection hooks
/// used by the lint/argparse machinery.
#[derive(Debug, Clone, Copy, Default)]
pub struct CwdSegment;

impl CwdSegment {
    /// Port of `CwdSegment.argspecobjs()` from
    /// `powerline/segments/common/env.py:44-47`.
    ///
    /// Yields `('get_shortened_path', self.get_shortened_path)` after
    /// the base Segment's argspec entries. The Rust port returns a
    /// fixed slice since the base Segment.argspecobjs() returns no
    /// additional entries.
    pub fn argspecobjs(&self) -> Vec<(&'static str, &'static str)> {
        // py:45-47
        vec![("get_shortened_path", "get_shortened_path")]
    }

    /// Port of `CwdSegment.omitted_args()` from
    /// `powerline/segments/common/env.py:49-53`.
    ///
    /// Returns an empty arg list for the `get_shortened_path` method
    /// per py:51; defers to the base implementation (empty by default)
    /// for any other method name per py:53.
    pub fn omitted_args(&self, method: &str) -> Vec<&'static str> {
        // py:50-53
        if method == "get_shortened_path" {
            // py:51  return ()
            Vec::new()
        } else {
            // py:53  return super(...).omitted_args(...)
            Vec::new()
        }
    }
}

/// Port of `_geteuid` module-level binding at
/// `powerline/segments/common/env.py:163`.
///
/// Python: `_geteuid = getattr(os, 'geteuid', lambda: 1)`. Rust uses
/// `libc::geteuid()` which is always available on Unix; the fallback
/// (`lambda: 1`) covers Windows where `os.geteuid` doesn't exist.
///
/// SAFETY: `libc::geteuid()` is a thread-safe POSIX syscall.
pub fn _geteuid() -> u32 {
    // py:163  os.geteuid() if available else 1
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() as u32 }
    }
    #[cfg(not(unix))]
    {
        1
    }
}

/// Port of `user()` segment from
/// `powerline/segments/common/env.py:160`.
///
/// Returns the current-user segment, choosing `superuser` highlight
/// when `euid == 0`. Honors the
/// `_POWERLINE_RUNNING_SHELL_TESTS` UUID override (py:167-171) by
/// returning `"user"` verbatim.
pub fn user(
    environ: &Map<String, Value>,
    hide_user: Option<&str>,
    hide_domain: bool,
    euid: u32,
) -> Option<Vec<Value>> {
    // py:167-171  _POWERLINE_RUNNING_SHELL_TESTS UUID short-circuit
    if let Some(test_uuid) = environ
        .get("_POWERLINE_RUNNING_SHELL_TESTS")
        .and_then(|v| v.as_str())
    {
        if test_uuid == POWERLINE_TEST_USER_UUID {
            return Some(vec![json!({
                "contents": "user",
            })]);
        }
    }
    // py:172-178  username lookup + hide_user / hide_domain
    let mut username = _get_user(environ)?;
    if let Some(hu) = hide_user {
        if username == hu {
            return None;
        }
    }
    if hide_domain {
        if let Some(idx) = username.find('@') {
            username.truncate(idx);
        }
    }
    // py:179-184  superuser highlight for euid == 0
    let groups: Vec<Value> = if euid == 0 {
        vec![
            Value::String("superuser".into()),
            Value::String("user".into()),
        ]
    } else {
        vec![Value::String("user".into())]
    };
    Some(vec![json!({
        "contents": username,
        "highlight_groups": groups,
    })])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(pairs: &[(&str, &str)]) -> Map<String, Value> {
        let mut m = Map::new();
        for (k, v) in pairs {
            m.insert(k.to_string(), Value::String((*v).into()));
        }
        m
    }

    #[test]
    fn environment_returns_value_when_set() {
        // py:17  environ.get(variable, None)
        let env = env_with(&[("FOO", "bar")]);
        assert_eq!(environment(&env, "FOO"), Some("bar".to_string()));
    }

    #[test]
    fn environment_returns_none_when_unset() {
        let env = Map::new();
        assert!(environment(&env, "FOO").is_none());
    }

    #[test]
    fn virtualenv_returns_last_path_component_from_virtual_env() {
        // py:31-34  reversed(VIRTUAL_ENV.split('/'))
        let env = env_with(&[("VIRTUAL_ENV", "/path/to/myenv")]);
        assert_eq!(
            virtualenv(&env, false, false, &["venv", ".venv"]),
            Some("myenv".to_string())
        );
    }

    #[test]
    fn virtualenv_skips_ignored_names_and_walks_to_parent() {
        // py:33-34  if candidate not in ignored_names: return
        let env = env_with(&[("VIRTUAL_ENV", "/path/to/myproj/venv")]);
        assert_eq!(
            virtualenv(&env, false, false, &["venv", ".venv"]),
            Some("myproj".to_string())
        );
    }

    #[test]
    fn virtualenv_falls_back_to_conda_when_no_virtualenv() {
        let env = env_with(&[("CONDA_DEFAULT_ENV", "/conda/envs/datasci")]);
        assert_eq!(
            virtualenv(&env, false, false, &["venv", ".venv"]),
            Some("datasci".to_string())
        );
    }

    #[test]
    fn virtualenv_ignore_venv_skips_virtual_env() {
        // py:30  if not ignore_venv
        let env = env_with(&[
            ("VIRTUAL_ENV", "/path/myenv"),
            ("CONDA_DEFAULT_ENV", "/conda/datasci"),
        ]);
        assert_eq!(
            virtualenv(&env, true, false, &[]),
            Some("datasci".to_string())
        );
    }

    #[test]
    fn virtualenv_no_env_returns_none() {
        let env = Map::new();
        assert!(virtualenv(&env, false, false, &[]).is_none());
    }

    #[test]
    fn get_shortened_path_not_found_returns_sentinel() {
        // py:60-63  OSError errno=2 → '[not found]'
        let r = get_shortened_path(
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
            Some("/home/user"),
            true,
        );
        assert_eq!(r.unwrap(), "[not found]");
    }

    #[test]
    fn get_shortened_path_shortens_home_prefix() {
        // py:66-70  if path.startswith(home): '~' + path[len(home):]
        let r = get_shortened_path(
            Ok("/home/user/projects".to_string()),
            Some("/home/user"),
            true,
        );
        assert_eq!(r.unwrap(), "~/projects");
    }

    #[test]
    fn get_shortened_path_no_home_returns_full_path() {
        let r = get_shortened_path(Ok("/home/user/projects".to_string()), None, true);
        assert_eq!(r.unwrap(), "/home/user/projects");
    }

    #[test]
    fn get_shortened_path_shorten_home_false_passes_through() {
        let r = get_shortened_path(
            Ok("/home/user/projects".to_string()),
            Some("/home/user"),
            false,
        );
        assert_eq!(r.unwrap(), "/home/user/projects");
    }

    #[test]
    fn cwd_segments_root_path() {
        let r = cwd_segments("/", None, None, false, Some("..."));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "/");
        assert_eq!(
            r[0]["highlight_groups"],
            json!(["cwd:current_folder", "cwd"])
        );
    }

    #[test]
    fn cwd_segments_multi_component() {
        // py:80-99  /a/b/c → ["a", "b", "c"] with leading empty
        let r = cwd_segments("/a/b/c", None, None, false, Some("..."));
        // "" + "a" + "b" + "c" → first empty replaced by "/", but
        // empty parts get skipped at py:91-92. So we expect:
        // "/", "a", "b", "c".
        assert_eq!(r.len(), 4);
        assert_eq!(r[0]["contents"], "/");
        assert_eq!(r[3]["contents"], "c");
        // Only last gets highlight_groups
        assert!(r[0].get("highlight_groups").is_none());
        assert_eq!(
            r[3]["highlight_groups"],
            json!(["cwd:current_folder", "cwd"])
        );
    }

    #[test]
    fn cwd_segments_dir_shorten_len_truncates_non_leaf() {
        // py:82  i[0:dir_shorten_len] for non-leaf
        let r = cwd_segments("/very/long/path/to/proj", Some(2), None, false, Some("..."));
        // Non-leaf parts truncated to 2 chars; leaf preserved.
        // Expected contents: "/", "ve", "lo", "pa", "to", "proj"
        let texts: Vec<String> = r
            .iter()
            .map(|s| s["contents"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(texts, vec!["/", "ve", "lo", "pa", "to", "proj"]);
    }

    #[test]
    fn cwd_segments_dir_limit_depth_adds_ellipsis() {
        // py:83-86  dir_limit_depth
        let r = cwd_segments("/very/long/path/to/proj", None, Some(2), false, Some("..."));
        let texts: Vec<String> = r
            .iter()
            .map(|s| s["contents"].as_str().unwrap().to_string())
            .collect();
        // depth=2 → keep last 2; prepend "..."
        // Original parts (post-leading-slash fix): "/", "very", "long", "path", "to", "proj"
        // Trimmed to last 2: "to", "proj"; ellipsis prepended → "...", "to", "proj"
        assert_eq!(texts, vec!["...", "to", "proj"]);
    }

    #[test]
    fn cwd_segments_use_path_separator_appends_to_non_last_and_strips_last() {
        // py:101-105  use_path_separator post-processing
        let r = cwd_segments("/a/b", None, None, true, Some("..."));
        // Without use_path_separator: ["/", "a", "b"]
        // With: each gets "/" appended → "//", "a/", "b/"
        // Post-processing: last "/" stripped → "b"; first leading "/"
        // stripped (when len > 1) → "/"
        let last_text = r.last().unwrap()["contents"].as_str().unwrap();
        assert_eq!(last_text, "b");
    }

    #[test]
    fn cwd_segments_divider_group_set_on_all() {
        let r = cwd_segments("/a/b", None, None, false, Some("..."));
        for s in &r {
            assert_eq!(s["divider_highlight_group"], "cwd:divider");
        }
    }

    #[test]
    fn cwd_segments_draw_inner_divider_inverse_of_use_path_separator() {
        // py:90  draw_inner_divider = not use_path_separator
        let r = cwd_segments("/a", None, None, false, Some("..."));
        assert_eq!(r[0]["draw_inner_divider"], true);
        let r2 = cwd_segments("/a", None, None, true, Some("..."));
        assert_eq!(r2[0]["draw_inner_divider"], false);
    }

    #[test]
    fn cwd_segments_empty_string_yields_root_segment() {
        // py:88-89  if not cwd[0]: cwd[0] = '/'  — empty input becomes "/"
        let r = cwd_segments("", None, None, false, Some("..."));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "/");
    }

    #[test]
    fn user_test_uuid_returns_user_verbatim() {
        // py:167-171  shell-test UUID override
        let env = env_with(&[("_POWERLINE_RUNNING_SHELL_TESTS", POWERLINE_TEST_USER_UUID)]);
        let r = user(&env, None, false, 1000).unwrap();
        assert_eq!(r[0]["contents"], "user");
    }

    #[test]
    fn user_returns_username_with_user_highlight_for_non_root() {
        let env = env_with(&[("USER", "alice")]);
        let r = user(&env, None, false, 1000).unwrap();
        assert_eq!(r[0]["contents"], "alice");
        assert_eq!(r[0]["highlight_groups"], json!(["user"]));
    }

    #[test]
    fn user_root_gets_superuser_highlight() {
        // py:179-184  euid == 0 → ['superuser', 'user']
        let env = env_with(&[("USER", "root")]);
        let r = user(&env, None, false, 0).unwrap();
        assert_eq!(r[0]["highlight_groups"], json!(["superuser", "user"]));
    }

    #[test]
    fn user_hidden_returns_none() {
        // py:175-176  if username == hide_user: return None
        let env = env_with(&[("USER", "alice")]);
        assert!(user(&env, Some("alice"), false, 1000).is_none());
    }

    #[test]
    fn user_hide_domain_strips_at_suffix() {
        // py:177-178  hide_domain: username = username[:index('@')]
        let env = env_with(&[("USER", "alice@example.com")]);
        let r = user(&env, None, true, 1000).unwrap();
        assert_eq!(r[0]["contents"], "alice");
    }

    #[test]
    fn user_hide_domain_no_at_passes_through() {
        let env = env_with(&[("USER", "alice")]);
        let r = user(&env, None, true, 1000).unwrap();
        assert_eq!(r[0]["contents"], "alice");
    }

    #[test]
    fn get_user_reads_environ_user_variable() {
        let env = env_with(&[("USER", "bob")]);
        assert_eq!(_get_user(&env), Some("bob".to_string()));
    }

    #[test]
    fn powerline_test_user_uuid_matches_upstream() {
        // py:171  exact constant from upstream
        assert_eq!(
            POWERLINE_TEST_USER_UUID,
            "ee5bcdc6-b749-11e7-9456-50465d597777"
        );
    }

    #[test]
    fn cwd_segment_argspecobjs_yields_get_shortened_path() {
        // py:45-47
        let s = CwdSegment;
        let entries = s.argspecobjs();
        assert!(entries
            .iter()
            .any(|(name, _)| *name == "get_shortened_path"));
    }

    #[test]
    fn cwd_segment_omitted_args_for_get_shortened_path_is_empty() {
        // py:50-51
        let s = CwdSegment;
        assert!(s.omitted_args("get_shortened_path").is_empty());
    }

    #[test]
    fn cwd_segment_omitted_args_for_unknown_method_is_empty() {
        // py:52-53  fall-through to super().omitted_args (empty default)
        let s = CwdSegment;
        assert!(s.omitted_args("anything_else").is_empty());
    }

    #[test]
    fn geteuid_returns_non_negative() {
        // py:163  os.geteuid() — always returns a uid >= 0 on Unix
        let uid = _geteuid();
        // Just verify the syscall succeeded (the result type is u32).
        let _ = uid;
    }
}
