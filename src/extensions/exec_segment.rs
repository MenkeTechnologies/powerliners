// vim:fileencoding=utf-8:noet
//! User-extensible segment dispatch — spawn a script, parse stdout,
//! return a segment list. Backs both the explicit `exec` adapter
//! (`"function": "exec"` in theme JSON, option A) and the dotted-path
//! filesystem fallback (`"function": "myseg.cpu_temp"` resolves to
//! `<config_path>/segments/myseg/cpu_temp.{sh,py,...}`, option B).
//!
//! Closes the architectural gap that the static `ADAPTERS` table left:
//! upstream Python's `__import__` makes any `~/.config/powerline/
//! segments/...` file callable; the Rust binary has no dynamic import
//! so we forward to a subprocess instead. The subprocess startup cost
//! is paid at the theme's `update_interval` cadence (default 2 s),
//! amortised across renders by the daemon's existing memoize layer.
//!
//! Script output protocol (auto-detected on the first byte of stdout):
//!
//! - **Plain text**: `cpu 42%\n` → wrapped as
//!   `[{"contents": "cpu 42%", "highlight_groups": [<from theme>]}]`
//! - **JSON array**: `[{"contents": "...", "highlight_groups": [...]}]`
//!   → used verbatim, gives the script full control over multi-chunk
//!   output, gradient levels, divider groups, etc.

use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// File extensions that mark a dotted-path script as runnable.
/// Order matters — checked top to bottom; the first hit wins.
/// Empty string matches a no-extension file (must be executable).
pub const SCRIPT_EXTENSIONS: &[&str] = &["sh", "py", "rb", "pl", "lua", "js", ""];

/// Resolve a dotted path (e.g., `"myseg.cpu_temp"`) to a script file
/// under one of the given segment search directories.
///
/// Each `search_dir` is treated as a config root; the lookup happens
/// under `<search_dir>/segments/<dotted-path-with-dots-as-slashes>`
/// plus each extension in [`SCRIPT_EXTENSIONS`]. Returns the first
/// hit, or `None` when nothing matches.
///
/// Example: `resolve_dotted_path("myseg.cpu_temp",
///                              &[PathBuf::from("/etc/powerline")])`
/// checks `/etc/powerline/segments/myseg/cpu_temp.sh`,
/// `.py`, `.rb`, `.pl`, `.lua`, `.js`, then `cpu_temp` (no extension).
pub fn resolve_dotted_path(dotted: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let rel = dotted.replace('.', "/");
    for dir in search_dirs {
        let base = dir.join("segments").join(&rel);
        for ext in SCRIPT_EXTENSIONS {
            let path = if ext.is_empty() {
                base.clone()
            } else {
                let mut p = base.clone();
                let _ = p.set_extension(ext);
                p
            };
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

/// Apply a printf-style template to a value. Supports the two
/// directives theme authors actually use:
///
/// - `%s` → replaced with `value`
/// - `%%` → literal `%`
///
/// Any other `%X` sequence passes through verbatim.
fn apply_format(fmt: &str, value: &str) -> String {
    let mut out = String::with_capacity(fmt.len() + value.len());
    let bytes = fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b's' => {
                    out.push_str(value);
                    i += 2;
                    continue;
                }
                b'%' => {
                    out.push('%');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Parse a script's stdout into a segment list, dispatching on the
/// first non-whitespace byte:
///
/// - `[` → attempt JSON array parse; on success, return as-is.
/// - anything else (or invalid JSON) → wrap as a single-chunk
///   `[{"contents": <stdout>}]`, optionally formatted via `format` and
///   tagged with `highlight_groups`.
pub fn parse_script_output(
    stdout: &str,
    format: Option<&str>,
    highlight_groups: Option<&[String]>,
) -> Vec<Value> {
    let trimmed = stdout.trim();
    if trimmed.starts_with('[') {
        if let Ok(arr) = serde_json::from_str::<Vec<Value>>(trimmed) {
            return arr;
        }
    }
    let contents = match format {
        Some(fmt) => apply_format(fmt, trimmed),
        None => trimmed.to_string(),
    };
    let mut obj = serde_json::Map::new();
    obj.insert("contents".to_string(), Value::String(contents));
    if let Some(groups) = highlight_groups {
        if !groups.is_empty() {
            obj.insert(
                "highlight_groups".to_string(),
                Value::Array(groups.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
    }
    vec![Value::Object(obj)]
}

/// Spawn `command` with the supplied positional `args`, optional env
/// overrides, and optional cwd. Capture stdout, parse via
/// [`parse_script_output`], return the segment list.
///
/// Returns `None` when:
/// - The command can't be spawned (not on `$PATH`, not executable, etc.)
/// - The command exits with non-zero status
/// - The stdout isn't valid UTF-8
///
/// The daemon treats `None` as "segment skipped" — same as any other
/// adapter that returns `None`.
pub fn exec_segment(
    command: &str,
    args: &[String],
    format: Option<&str>,
    env: Option<&HashMap<String, String>>,
    cwd: Option<&str>,
    highlight_groups: Option<&[String]>,
) -> Option<Vec<Value>> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    if let Some(e) = env {
        for (k, v) in e {
            cmd.env(k, v);
        }
    }
    if let Some(c) = cwd {
        cmd.current_dir(c);
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    Some(parse_script_output(&stdout, format, highlight_groups))
}

/// Convenience wrapper for the dotted-path dispatch: resolve the path
/// first, then `exec_segment` against it with no extra args.
///
/// Returns `None` when the dotted path doesn't resolve to a script
/// under any of the `search_dirs`, or when the resolved script fails
/// to execute / parse. Matches the same `None`-skipped semantics as
/// [`exec_segment`].
pub fn exec_by_dotted_path(
    dotted: &str,
    search_dirs: &[PathBuf],
    args: &[String],
    format: Option<&str>,
    highlight_groups: Option<&[String]>,
) -> Option<Vec<Value>> {
    let script = resolve_dotted_path(dotted, search_dirs)?;
    let script_str = script.to_str()?;
    exec_segment(script_str, args, format, None, None, highlight_groups)
}

/// Hint for theme authors: the `exec` adapter's reserved dotted-path
/// name as registered in the daemon's `ADAPTERS` table. Theme JSON
/// references `"function": "exec"` which `gen_segment_getter`
/// expands to the fully-qualified id via the daemon's default-module
/// fallback chain.
pub const EXEC_DOTTED_PATH: &str = "powerliners.exec.exec";

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn parse_plain_text_wraps_as_contents() {
        let segs = parse_script_output("cpu 42%", None, None);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0]["contents"], "cpu 42%");
        assert!(
            segs[0].get("highlight_groups").is_none(),
            "no theme-side groups → key absent"
        );
    }

    #[test]
    fn parse_plain_text_with_format_applies_template() {
        let segs = parse_script_output("42", Some("cpu: %s%%"), None);
        assert_eq!(segs[0]["contents"], "cpu: 42%");
    }

    #[test]
    fn parse_plain_text_with_highlight_groups_attaches_them() {
        let groups = vec!["cpu_load".to_string(), "background".to_string()];
        let segs = parse_script_output("70%", None, Some(&groups));
        assert_eq!(segs[0]["highlight_groups"][0], "cpu_load");
        assert_eq!(segs[0]["highlight_groups"][1], "background");
    }

    #[test]
    fn parse_json_array_passes_through_verbatim() {
        let json = r#"[{"contents": "A", "highlight_groups": ["g1"]}, {"contents": "B"}]"#;
        let segs = parse_script_output(json, None, None);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0]["contents"], "A");
        assert_eq!(segs[0]["highlight_groups"][0], "g1");
        assert_eq!(segs[1]["contents"], "B");
    }

    #[test]
    fn parse_malformed_json_falls_back_to_plain_text() {
        // Starts with `[` but isn't valid JSON → wrap as plain text.
        let segs = parse_script_output("[not json", None, None);
        assert_eq!(segs[0]["contents"], "[not json");
    }

    #[test]
    fn parse_trims_trailing_newline() {
        let segs = parse_script_output("hello\n", None, None);
        assert_eq!(segs[0]["contents"], "hello");
    }

    #[test]
    fn resolve_dotted_path_finds_sh_extension() {
        let tmp = TempDir::new().expect("tempdir");
        let target = tmp.path().join("segments/myseg/cpu_temp.sh");
        fs::create_dir_all(target.parent().unwrap()).expect("mkdir");
        fs::write(&target, "#!/bin/sh\necho 42").expect("write");
        let found = resolve_dotted_path("myseg.cpu_temp", &[tmp.path().to_path_buf()]);
        assert_eq!(found, Some(target));
    }

    #[test]
    fn resolve_dotted_path_prefers_sh_over_py_when_both_exist() {
        // SCRIPT_EXTENSIONS lists sh before py, so sh wins.
        let tmp = TempDir::new().expect("tempdir");
        let dir = tmp.path().join("segments/myseg");
        fs::create_dir_all(&dir).expect("mkdir");
        let sh = dir.join("cpu_temp.sh");
        let py = dir.join("cpu_temp.py");
        fs::write(&sh, "#!/bin/sh").expect("write sh");
        fs::write(&py, "#!/usr/bin/env python3").expect("write py");
        let found = resolve_dotted_path("myseg.cpu_temp", &[tmp.path().to_path_buf()]);
        assert_eq!(found, Some(sh));
    }

    #[test]
    fn resolve_dotted_path_falls_back_to_no_extension() {
        let tmp = TempDir::new().expect("tempdir");
        let dir = tmp.path().join("segments/myseg");
        fs::create_dir_all(&dir).expect("mkdir");
        let target = dir.join("cpu_temp");
        fs::write(&target, "#!/bin/sh").expect("write");
        let found = resolve_dotted_path("myseg.cpu_temp", &[tmp.path().to_path_buf()]);
        assert_eq!(found, Some(target));
    }

    #[test]
    fn resolve_dotted_path_returns_none_when_missing() {
        let tmp = TempDir::new().expect("tempdir");
        let found = resolve_dotted_path("never.exists", &[tmp.path().to_path_buf()]);
        assert!(found.is_none());
    }

    #[test]
    fn resolve_dotted_path_walks_multiple_search_dirs() {
        let tmp1 = TempDir::new().expect("tempdir1");
        let tmp2 = TempDir::new().expect("tempdir2");
        // Place script only under tmp2.
        let target = tmp2.path().join("segments/myseg/x.sh");
        fs::create_dir_all(target.parent().unwrap()).expect("mkdir");
        fs::write(&target, "#!/bin/sh").expect("write");
        let dirs = vec![tmp1.path().to_path_buf(), tmp2.path().to_path_buf()];
        let found = resolve_dotted_path("myseg.x", &dirs);
        assert_eq!(found, Some(target));
    }

    #[test]
    fn exec_segment_captures_echo_stdout() {
        let segs = exec_segment("echo", &["hello".to_string()], None, None, None, None);
        let segs = segs.expect("echo should succeed");
        assert_eq!(segs[0]["contents"], "hello");
    }

    #[test]
    fn exec_segment_returns_none_for_missing_command() {
        let segs = exec_segment(
            "powerliners_definitely_not_a_real_binary_xyz",
            &[],
            None,
            None,
            None,
            None,
        );
        assert!(segs.is_none());
    }

    #[test]
    fn exec_segment_returns_none_for_nonzero_exit() {
        // `false` exits 1 — daemon should skip the segment.
        let segs = exec_segment("false", &[], None, None, None, None);
        assert!(segs.is_none());
    }

    #[test]
    fn exec_segment_with_format_applies_template_to_stdout() {
        let segs = exec_segment(
            "echo",
            &["42".to_string()],
            Some("cpu: %s%%"),
            None,
            None,
            None,
        );
        assert_eq!(segs.unwrap()[0]["contents"], "cpu: 42%");
    }

    #[test]
    fn exec_segment_with_highlight_groups_attaches_them() {
        let groups = vec!["cpu_load".to_string()];
        let segs = exec_segment("echo", &["x".to_string()], None, None, None, Some(&groups));
        let segs = segs.unwrap();
        assert_eq!(segs[0]["highlight_groups"][0], "cpu_load");
    }

    #[test]
    fn exec_segment_parses_json_array_output() {
        // Use printf to emit JSON without trailing whitespace.
        let json = r#"[{"contents":"A"},{"contents":"B"}]"#;
        let segs = exec_segment(
            "printf",
            &["%s".to_string(), json.to_string()],
            None,
            None,
            None,
            None,
        );
        let segs = segs.expect("printf JSON should parse");
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0]["contents"], "A");
        assert_eq!(segs[1]["contents"], "B");
    }

    #[test]
    fn exec_by_dotted_path_runs_resolved_script() {
        let tmp = TempDir::new().expect("tempdir");
        let script = tmp.path().join("segments/myseg/hello.sh");
        fs::create_dir_all(script.parent().unwrap()).expect("mkdir");
        fs::write(&script, "#!/bin/sh\necho via-script").expect("write");
        // Make it executable so /bin/sh /path/to/script is unambiguous;
        // resolve_dotted_path doesn't check the executable bit but
        // Command::new spawns via the path so it must be runnable.
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).expect("chmod");
        let segs = exec_by_dotted_path("myseg.hello", &[tmp.path().to_path_buf()], &[], None, None);
        let segs = segs.expect("script should run");
        assert_eq!(segs[0]["contents"], "via-script");
    }
}
