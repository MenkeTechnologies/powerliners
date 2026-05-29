// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/shell.py`.
//!
//! Shell-specific segments: jobnum, last_status (exit code),
//! last_pipe_status (pipe array), mode, continuation (parser state),
//! and ShellCwdSegment (cwd with `--renderer-arg shortened_path` override).

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import requires_segment_info                                       // py:4
// from powerline.segments import with_docstring                                           // py:5
// from powerline.segments.common.env import CwdSegment                                    // py:6
// from powerline.lib.unicode import out_u                                                 // py:7

use serde_json::{json, Map, Value};

/// Per-shell-segment info shape.
#[derive(Debug, Clone, Default)]
pub struct ShellSegmentInfo {
    pub jobnum: Option<i32>,
    pub last_exit_code: Option<i32>,
    pub last_pipe_status: Vec<i32>,
    pub mode: Option<String>,
    pub default_mode: Option<String>,
    pub parser_state: Option<String>,
    pub shortened_path: Option<String>,
}

/// Port of `jobnum()` from `powerline/segments/shell.py:11`.
///
/// Return the number of jobs.
///
/// :param show_zero: If false (default) shows nothing if there are no
///     jobs. Otherwise shows zero for no jobs.
pub fn jobnum(_pl: &(), segment_info: &ShellSegmentInfo, show_zero: bool) -> Option<String> {
    // py:10  @requires_segment_info
    // py:11  def jobnum(pl, segment_info, show_zero=False):
    // py:12-17  docstring
    // py:18  jobnum = segment_info['args'].jobnum
    let jobnum = segment_info.jobnum?;
    // py:19  if jobnum is None or (not show_zero and jobnum == 0):
    // py:20  return None
    if !show_zero && jobnum == 0 {
        return None;
    }
    // py:21  else:
    // py:22  return str(jobnum)
    Some(jobnum.to_string())
}

/// Port of module-level binding `exit_codes` from
/// `powerline/segments/shell.py:24-28`.
///
/// Python: `dict((k, v) for v, k in reversed(sorted(signal.__dict__.items()))
///                if v.startswith('SIG') and not v.startswith('SIG_'))`
///
/// Maps signal number → name string (e.g. 9 → "SIGKILL"). Used by
/// `last_status` / `last_pipe_status` to translate `exit_code - 128`
/// into the signal name for processes killed by signals.
#[allow(non_upper_case_globals)]
pub fn exit_codes(n: i32) -> Option<&'static str> {
    match n {
        1 => Some("SIGHUP"),
        2 => Some("SIGINT"),
        3 => Some("SIGQUIT"),
        4 => Some("SIGILL"),
        6 => Some("SIGABRT"),
        8 => Some("SIGFPE"),
        9 => Some("SIGKILL"),
        11 => Some("SIGSEGV"),
        13 => Some("SIGPIPE"),
        14 => Some("SIGALRM"),
        15 => Some("SIGTERM"),
        // The signal number map varies by platform; only the cross-Unix
        // common subset is enumerated here. The full Python `signal`
        // module enumerates everything visible in libc — porting the
        // full set is deferred to when a consumer asks.
        _ => None,
    }
}

/// Port of `last_status()` from `powerline/segments/shell.py:31`.
///
/// Return last exit code.
///
/// :param signal_names: If true (default), translate signal numbers
///     to human-readable names.
///
/// Highlight groups used: `exit_fail`.
pub fn last_status(
    _pl: &(),
    segment_info: &ShellSegmentInfo,
    signal_names: bool,
) -> Option<Vec<Value>> {
    // py:31  @requires_segment_info
    // py:32  def last_status(pl, segment_info, signal_names=True):
    // py:33-39  docstring
    // py:40  if not segment_info['args'].last_exit_code:
    // py:41  return None
    let last_exit_code = segment_info.last_exit_code?;
    if last_exit_code == 0 {
        return None;
    }
    // py:43  try:
    // py:44  if signal_names and segment_info['args'].last_exit_code - 128 in exit_codes:
    // py:45  return [{'contents': exit_codes[...], 'highlight_groups': ['exit_fail']}]
    // py:46  except TypeError:
    // py:47  pass
    if signal_names {
        if let Some(name) = exit_codes(last_exit_code - 128) {
            return Some(vec![json!({
                "contents": name,
                "highlight_groups": ["exit_fail"],
            })]);
        }
    }
    // py:48  return [{'contents': str(segment_info['args'].last_exit_code), 'highlight_groups': ['exit_fail']}]
    Some(vec![json!({
        "contents": last_exit_code.to_string(),
        "highlight_groups": ["exit_fail"],
    })])
}

/// Port of `last_pipe_status()` from `powerline/segments/shell.py:49`.
///
/// Return last pipe status.
///
/// :param signal_names: If true (default), translate signal numbers
///     to human-readable names.
///
/// Highlight groups used: `exit_fail`, `exit_success`.
pub fn last_pipe_status(
    _pl: &(),
    segment_info: &ShellSegmentInfo,
    signal_names: bool,
) -> Option<Vec<Value>> {
    // py:51  @requires_segment_info
    // py:52  def last_pipe_status(pl, segment_info, signal_names=True):
    // py:53-59  docstring
    // py:60  last_pipe_status = (
    // py:61  segment_info['args'].last_pipe_status
    // py:62  or (segment_info['args'].last_exit_code,)
    // py:63  )
    let statuses: Vec<i32> = if !segment_info.last_pipe_status.is_empty() {
        segment_info.last_pipe_status.clone()
    } else {
        match segment_info.last_exit_code {
            Some(code) => vec![code],
            None => return None,
        }
    };
    // py:64  if any(last_pipe_status):
    if !statuses.iter().any(|&s| s != 0) {
        // py:79  return None
        return None;
    }
    // py:65  try:
    // py:66  return [{
    // py:67  'contents': exit_codes[status - 128] if signal_names and \
    // py:68  status - 128 in exit_codes else str(status),
    // py:69  'highlight_groups': ['exit_fail' if status else 'exit_success'],
    // py:70  'draw_inner_divider': True
    // py:71  } for status in last_pipe_status]
    // py:72  except TypeError:
    // py:73  return [{
    // py:74  'contents': str(status),
    // py:75  'highlight_groups': ['exit_fail' if status else 'exit_success'],
    // py:76  'draw_inner_divider': True
    // py:77  } for status in last_pipe_status]
    let segments: Vec<Value> = statuses
        .iter()
        .map(|&status| {
            let contents = if signal_names {
                exit_codes(status - 128)
                    .map(String::from)
                    .unwrap_or_else(|| status.to_string())
            } else {
                status.to_string()
            };
            let highlight = if status != 0 {
                "exit_fail"
            } else {
                "exit_success"
            };
            json!({
                "contents": contents,
                "highlight_groups": [highlight],
                "draw_inner_divider": true,
            })
        })
        .collect();
    Some(segments)
}

/// Port of `mode()` from `powerline/segments/shell.py:80`.
///
/// Return the current mode.
///
/// :param override: dict for overriding mode strings.
/// :param default: If current mode is equal to this string then this
///     segment will not get displayed. If not specified the value is
///     taken from `$POWERLINE_DEFAULT_MODE` variable (set by zsh
///     bindings for any mode that does not start from `vi`).
pub fn mode(
    _pl: &(),
    segment_info: &ShellSegmentInfo,
    override_table: &Map<String, Value>,
    default: Option<&str>,
) -> Option<String> {
    // py:81  @requires_segment_info
    // py:82  def mode(pl, segment_info, override={'vicmd': 'COMMND', 'viins': 'INSERT'}, default=None):
    // py:83-92  docstring
    // py:93  mode = segment_info.get('mode', None)
    // py:94  if not mode:
    // py:95  pl.debug('No mode specified')
    // py:96  return None
    let mode = segment_info.mode.as_ref()?;
    if mode.is_empty() {
        return None;
    }
    // py:97  default = default or segment_info.get('default_mode', None)
    let default = default
        .map(String::from)
        .or_else(|| segment_info.default_mode.clone());
    // py:98  if mode == default:
    // py:99  return None
    if Some(mode.clone()) == default {
        return None;
    }
    // py:100  try:
    // py:101  return override[mode]
    if let Some(override_val) = override_table.get(mode).and_then(|v| v.as_str()) {
        return Some(override_val.to_string());
    }
    // py:102  except KeyError:
    // py:103-108  comment about zsh line editor / unknown modes
    // py:109  return mode.upper()
    Some(mode.to_uppercase())
}

/// Port of `continuation()` from `powerline/segments/shell.py:112`.
///
/// Display parser state.
///
/// :param omit_cmdsubst: Do not display cmdsubst parser state if it is
///     the last one.
/// :param right_align: Align to the right.
/// :param renames: Rename states: `{old_name: new_name}`. If new_name
///     is None then given state is not displayed.
pub fn continuation(
    _pl: &(),
    segment_info: &ShellSegmentInfo,
    omit_cmdsubst: bool,
    right_align: bool,
    renames: &Map<String, Value>,
) -> Vec<Value> {
    // py:112  @requires_segment_info
    // py:113  def continuation(pl, segment_info, omit_cmdsubst=True, right_align=False, renames={}):
    // py:114-125  docstring
    // py:126  if not segment_info.get('parser_state'):
    // py:127  return [{
    // py:128  'contents': '',
    // py:129  'width': 'auto',
    // py:130  'highlight_groups': ['continuation:current', 'continuation'],
    // py:131  }]
    let parser_state = match &segment_info.parser_state {
        Some(s) if !s.is_empty() => s,
        _ => {
            return vec![json!({
                "contents": "",
                "width": "auto",
                "highlight_groups": ["continuation:current", "continuation"],
            })];
        }
    };
    // py:132  ret = []
    let mut ret: Vec<Value> = Vec::new();

    // py:134  for state in segment_info['parser_state'].split():
    // py:135  state = renames.get(state, state)
    // py:136  if state:
    // py:137  ret.append({
    // py:138  'contents': state,
    // py:139  'highlight_groups': ['continuation'],
    // py:140  'draw_inner_divider': True,
    // py:141  })
    for state in parser_state.split_whitespace() {
        let renamed = renames.get(state).and_then(|v| v.as_str()).unwrap_or(state);
        if renamed.is_empty() {
            continue;
        }
        ret.push(json!({
            "contents": renamed,
            "highlight_groups": ["continuation"],
            "draw_inner_divider": true,
        }));
    }

    // py:143  if omit_cmdsubst and ret[-1]['contents'] == 'cmdsubst':
    // py:144  ret.pop(-1)
    if omit_cmdsubst {
        if let Some(last) = ret.last() {
            if last.get("contents").and_then(|v| v.as_str()) == Some("cmdsubst") {
                ret.pop();
            }
        }
    }

    // py:146  if not ret:
    // py:147  ret.append({
    // py:148  'contents': ''
    // py:149  })
    if ret.is_empty() {
        ret.push(json!({"contents": ""}));
    }

    // py:151  if right_align:
    // py:152  ret[0].update(width='auto', align='r')
    // py:153  ret[-1]['highlight_groups'] = ['continuation:current', 'continuation']
    // py:154  else:
    // py:155  ret[-1].update(width='auto', align='l', highlight_groups=['continuation:current', 'continuation'])
    if right_align {
        if let Some(Value::Object(map)) = ret.first_mut() {
            map.insert("width".into(), Value::String("auto".into()));
            map.insert("align".into(), Value::String("r".into()));
        }
        if let Some(Value::Object(map)) = ret.last_mut() {
            map.insert(
                "highlight_groups".into(),
                json!(["continuation:current", "continuation"]),
            );
        }
    } else if let Some(Value::Object(map)) = ret.last_mut() {
        map.insert("width".into(), Value::String("auto".into()));
        map.insert("align".into(), Value::String("l".into()));
        map.insert(
            "highlight_groups".into(),
            json!(["continuation:current", "continuation"]),
        );
    }

    // py:157  return ret
    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobnum_returns_none_when_zero_and_no_show_zero() {
        let info = ShellSegmentInfo {
            jobnum: Some(0),
            ..Default::default()
        };
        assert!(jobnum(&(), &info, false).is_none());
    }

    #[test]
    fn jobnum_returns_zero_when_show_zero() {
        let info = ShellSegmentInfo {
            jobnum: Some(0),
            ..Default::default()
        };
        assert_eq!(jobnum(&(), &info, true), Some("0".into()));
    }

    #[test]
    fn jobnum_returns_count_string() {
        let info = ShellSegmentInfo {
            jobnum: Some(3),
            ..Default::default()
        };
        assert_eq!(jobnum(&(), &info, false), Some("3".into()));
    }

    #[test]
    fn jobnum_none_jobnum_returns_none() {
        let info = ShellSegmentInfo::default();
        assert!(jobnum(&(), &info, false).is_none());
    }

    #[test]
    fn exit_codes_known_signals() {
        assert_eq!(exit_codes(9), Some("SIGKILL"));
        assert_eq!(exit_codes(15), Some("SIGTERM"));
        assert_eq!(exit_codes(2), Some("SIGINT"));
        assert_eq!(exit_codes(999), None);
    }

    #[test]
    fn last_status_zero_returns_none() {
        let info = ShellSegmentInfo {
            last_exit_code: Some(0),
            ..Default::default()
        };
        assert!(last_status(&(), &info, true).is_none());
    }

    #[test]
    fn last_status_signal_kills() {
        // exit 137 = 128 + 9 (SIGKILL)
        let info = ShellSegmentInfo {
            last_exit_code: Some(137),
            ..Default::default()
        };
        let result = last_status(&(), &info, true).unwrap();
        assert_eq!(result[0]["contents"], "SIGKILL");
        assert_eq!(result[0]["highlight_groups"], json!(["exit_fail"]));
    }

    #[test]
    fn last_status_non_signal_returns_number() {
        let info = ShellSegmentInfo {
            last_exit_code: Some(42),
            ..Default::default()
        };
        let result = last_status(&(), &info, true).unwrap();
        assert_eq!(result[0]["contents"], "42");
    }

    #[test]
    fn last_pipe_status_all_zero_returns_none() {
        let info = ShellSegmentInfo {
            last_pipe_status: vec![0, 0, 0],
            ..Default::default()
        };
        assert!(last_pipe_status(&(), &info, true).is_none());
    }

    #[test]
    fn last_pipe_status_mixed_emits_per_status_segments() {
        let info = ShellSegmentInfo {
            last_pipe_status: vec![0, 1, 137],
            ..Default::default()
        };
        let result = last_pipe_status(&(), &info, true).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0]["highlight_groups"], json!(["exit_success"]));
        assert_eq!(result[1]["highlight_groups"], json!(["exit_fail"]));
        assert_eq!(result[2]["contents"], "SIGKILL");
    }

    #[test]
    fn mode_returns_uppercase_when_no_override() {
        let info = ShellSegmentInfo {
            mode: Some("normal".into()),
            ..Default::default()
        };
        let override_table = Map::new();
        assert_eq!(
            mode(&(), &info, &override_table, None),
            Some("NORMAL".into())
        );
    }

    #[test]
    fn mode_returns_override_when_present() {
        let info = ShellSegmentInfo {
            mode: Some("vicmd".into()),
            ..Default::default()
        };
        let mut override_table = Map::new();
        override_table.insert("vicmd".into(), json!("COMMND"));
        assert_eq!(
            mode(&(), &info, &override_table, None),
            Some("COMMND".into())
        );
    }

    #[test]
    fn mode_returns_none_when_matches_default() {
        let info = ShellSegmentInfo {
            mode: Some("insert".into()),
            ..Default::default()
        };
        let override_table = Map::new();
        assert!(mode(&(), &info, &override_table, Some("insert")).is_none());
    }

    #[test]
    fn continuation_no_parser_state_returns_empty_segment() {
        let info = ShellSegmentInfo::default();
        let renames = Map::new();
        let r = continuation(&(), &info, true, false, &renames);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "");
        assert_eq!(r[0]["width"], "auto");
    }

    #[test]
    fn continuation_splits_parser_state() {
        let info = ShellSegmentInfo {
            parser_state: Some("if while for".into()),
            ..Default::default()
        };
        let renames = Map::new();
        let r = continuation(&(), &info, true, false, &renames);
        // 3 states, last one gets the alignment + current highlight overlay
        assert_eq!(r.len(), 3);
        assert_eq!(r[0]["contents"], "if");
        assert_eq!(
            r[2]["highlight_groups"],
            json!(["continuation:current", "continuation"])
        );
    }

    #[test]
    fn continuation_omits_cmdsubst_last() {
        let info = ShellSegmentInfo {
            parser_state: Some("if cmdsubst".into()),
            ..Default::default()
        };
        let renames = Map::new();
        let r = continuation(&(), &info, true, false, &renames);
        // cmdsubst dropped, only "if" remains
        assert_eq!(r.len(), 1);
        assert_eq!(r[0]["contents"], "if");
    }

    #[test]
    fn continuation_right_align_sets_first_align() {
        let info = ShellSegmentInfo {
            parser_state: Some("if while".into()),
            ..Default::default()
        };
        let renames = Map::new();
        let r = continuation(&(), &info, true, true, &renames);
        assert_eq!(r[0]["align"], "r");
        assert_eq!(r[0]["width"], "auto");
    }
}
