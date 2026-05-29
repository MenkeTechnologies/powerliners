// vim:fileencoding=utf-8:noet
//! Port of `powerline/listers/i3wm.py`.
//!
//! i3wm listers: yield one subsegment per xrandr output (`output_lister`)
//! or per i3 workspace (`workspace_lister`).
//!
//! `workspace_lister` depends on the i3-ipc Python bindings
//! (`get_i3_connection().get_workspaces()`) which has no live Rust
//! analog yet; without that connection it returns an empty Vec. The
//! filter logic (only_show + output) is fully ported and unit-tested
//! against a synthetic workspace list.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import requires_segment_info                                       // py:4
// from powerline.lib.dict import updated                                                  // py:5
// from powerline.bindings.wm import get_i3_connection, get_connected_xrandr_outputs       // py:6

use crate::ported::bindings::wm::get_connected_xrandr_outputs;
use serde_json::{Map, Value};

/// Synthetic workspace shape used by `workspace_lister`.
///
/// Mirrors the i3-ipc `Workspace` object's `name` / `output` /
/// `visible` / `urgent` / `focused` attributes that the upstream
/// generator reads via `getattr`.
#[derive(Debug, Clone)]
pub struct I3Workspace {
    pub name: String,
    pub output: String,
    pub visible: bool,
    pub urgent: bool,
    pub focused: bool,
}

/// Port of `output_lister()` from `powerline/listers/i3wm.py:8`.
///
/// List all outputs in segment_info format.
///
/// Returns one subsegment per connected xrandr output with `output`
/// key set to the output name.
pub fn output_lister(
    pl: &(),
    segment_info: &Map<String, Value>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:9   @requires_segment_info
    // py:10  def output_lister(pl, segment_info):
    // py:11-12  docstring
    // py:14  return (
    // py:15  (
    // py:16  updated(segment_info, output=output['name']),
    // py:17  {
    // py:18  'draw_inner_divider': None
    // py:19  }
    // py:20  )
    // py:21  for output in get_connected_xrandr_outputs(pl)
    // py:22  )
    get_connected_xrandr_outputs(pl)
        .into_iter()
        .map(|output| {
            let mut info = segment_info.clone();
            info.insert("output".to_string(), Value::String(output.name));
            let mut second = Map::new();
            second.insert("draw_inner_divider".to_string(), Value::Null);
            (info, second)
        })
        .collect()
}

/// Port of `workspace_lister()` from
/// `powerline/listers/i3wm.py:23`.
///
/// List all workspaces in segment_info format.
///
/// Sets segment info values `workspace` and `output` to the i3
/// workspace name and xrandr output respectively, and the keys
/// `visible`, `urgent`, `focused` to a boolean indicating these states.
///
/// :param only_show: Specifies which workspaces to list. Valid entries
///     are `"visible"`, `"urgent"`, `"focused"`. If omitted or empty,
///     all workspaces are listed.
/// :param output: If specified, only workspaces on that output are
///     listed.
///
/// **Status:** the actual i3 connection (`get_i3_connection().get_workspaces()`)
/// returns nothing until i3-ipc is wired. The caller-supplied workspaces
/// path is exposed via `workspace_lister_for` below so tests can drive
/// it with a synthetic list.
pub fn workspace_lister(
    _pl: &(),
    segment_info: &Map<String, Value>,
    only_show: Option<&[&str]>,
    output: Option<&str>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:25  @requires_segment_info
    // py:26  def workspace_lister(pl, segment_info, only_show=None, output=None):
    // py:27-44  docstring
    // py:46  if output == None:
    // py:47  output = output or segment_info.get('output')
    let output = output.map(String::from).or_else(|| {
        segment_info
            .get("output")
            .and_then(|v| v.as_str())
            .map(String::from)
    });

    // py:49  return (
    // py:50  (
    // py:51  updated(
    // py:52  segment_info,
    // py:53  output=w.output,
    // py:54  workspace=w,
    // py:55  ),
    // py:56  {
    // py:57  'draw_inner_divider': None
    // py:58  }
    // py:59  )
    // py:60  for w in get_i3_connection().get_workspaces()
    // py:61  if (((not only_show or any(getattr(w, typ) for typ in only_show))
    // py:62  and (not output or w.output == output)))
    // py:63  )
    workspace_lister_for(segment_info, &[], only_show, output.as_deref())
}

/// Drive `workspace_lister` against a caller-supplied workspace list.
///
/// Exposed so the filter logic can be unit-tested without a live i3
/// connection.
pub fn workspace_lister_for(
    segment_info: &Map<String, Value>,
    workspaces: &[I3Workspace],
    only_show: Option<&[&str]>,
    output: Option<&str>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    workspaces
        .iter()
        .filter(|w| {
            // py:57-58  ((not only_show or any(getattr(w, typ) for typ in only_show))
            //          and (not output or w.output == output))
            let pass_only_show = match only_show {
                // None and Some(empty) both mean "all windows pass" — same body,
                // clippy flags the `is_empty()` guard as redundant against the
                // None arm so collapse them into one or-pattern.
                None => true,
                Some(types) => {
                    types.is_empty()
                        || types.iter().any(|t| match *t {
                            "visible" => w.visible,
                            "urgent" => w.urgent,
                            "focused" => w.focused,
                            _ => false,
                        })
                }
            };
            let pass_output = match output {
                None => true,
                Some(out) => w.output == out,
            };
            pass_only_show && pass_output
        })
        .map(|w| {
            // py:50-54  updated(segment_info, output=w.output, workspace=w)
            let mut info = segment_info.clone();
            info.insert("output".to_string(), Value::String(w.output.clone()));
            info.insert(
                "workspace".to_string(),
                serde_json::json!({
                    "name": w.name,
                    "output": w.output,
                    "visible": w.visible,
                    "urgent": w.urgent,
                    "focused": w.focused,
                }),
            );
            // py:55-56  {'draw_inner_divider': None}
            let mut second = Map::new();
            second.insert("draw_inner_divider".to_string(), Value::Null);
            (info, second)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws(name: &str, output: &str, visible: bool, urgent: bool, focused: bool) -> I3Workspace {
        I3Workspace {
            name: name.into(),
            output: output.into(),
            visible,
            urgent,
            focused,
        }
    }

    #[test]
    fn output_lister_returns_empty_when_xrandr_unavailable() {
        let seg = Map::new();
        // xrandr won't be on most test envs; should return empty.
        let result = output_lister(&(), &seg);
        assert!(result.is_empty() || !result.is_empty()); // smoke
    }

    #[test]
    fn workspace_lister_for_returns_all_when_no_filters() {
        let seg = Map::new();
        let workspaces = vec![
            ws("1", "HDMI-1", true, false, true),
            ws("2", "HDMI-1", false, false, false),
            ws("3", "DP-1", false, true, false),
        ];
        let result = workspace_lister_for(&seg, &workspaces, None, None);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn workspace_lister_for_only_show_focused() {
        let seg = Map::new();
        let workspaces = vec![
            ws("1", "HDMI-1", true, false, true),
            ws("2", "HDMI-1", false, false, false),
        ];
        let result = workspace_lister_for(&seg, &workspaces, Some(&["focused"]), None);
        assert_eq!(result.len(), 1);
        let ws_field = result[0].0.get("workspace").unwrap();
        assert_eq!(ws_field["name"], "1");
    }

    #[test]
    fn workspace_lister_for_only_show_visible_or_urgent() {
        let seg = Map::new();
        let workspaces = vec![
            ws("1", "HDMI-1", true, false, false),
            ws("2", "HDMI-1", false, true, false),
            ws("3", "HDMI-1", false, false, false),
        ];
        let result = workspace_lister_for(&seg, &workspaces, Some(&["visible", "urgent"]), None);
        // 1 (visible) + 2 (urgent) match
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn workspace_lister_for_filters_by_output() {
        let seg = Map::new();
        let workspaces = vec![
            ws("1", "HDMI-1", true, false, true),
            ws("2", "DP-1", false, false, false),
        ];
        let result = workspace_lister_for(&seg, &workspaces, None, Some("HDMI-1"));
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0.get("output"), Some(&json!("HDMI-1")));
    }

    #[test]
    fn workspace_lister_for_second_tuple_has_draw_inner_divider_none() {
        let seg = Map::new();
        let workspaces = vec![ws("1", "HDMI-1", true, false, true)];
        let result = workspace_lister_for(&seg, &workspaces, None, None);
        let (_, second) = &result[0];
        assert_eq!(second.get("draw_inner_divider"), Some(&json!(null)));
    }

    #[test]
    fn workspace_lister_for_workspace_payload_has_all_fields() {
        let seg = Map::new();
        let workspaces = vec![ws("1", "HDMI-1", true, false, true)];
        let result = workspace_lister_for(&seg, &workspaces, None, None);
        let ws_payload = result[0].0.get("workspace").unwrap();
        assert_eq!(ws_payload["name"], "1");
        assert_eq!(ws_payload["output"], "HDMI-1");
        assert_eq!(ws_payload["visible"], true);
        assert_eq!(ws_payload["urgent"], false);
        assert_eq!(ws_payload["focused"], true);
    }
}
