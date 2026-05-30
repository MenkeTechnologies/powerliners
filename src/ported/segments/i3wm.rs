// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/i3wm.py`.
//!
//! i3 / sway window-manager segment helpers. Surfaces the pure
//! transformation functions: workspace-group classification,
//! workspace-name stripping, icon dispatch, scratchpad-group
//! classification, mode-name translation. The actual live
//! `i3ipc` connection (`get_i3_connection`) + `requires_segment_info`
//! decorated workspace/scratchpad/active_window functions are stubbed
//! since they need the i3ipc runtime.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// from powerline.theme import requires_segment_info                                       // py:6
// from powerline.bindings.wm import get_i3_connection                                     // py:7

use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

/// Port of `WORKSPACE_REGEX` from
/// `powerline/segments/i3wm.py:9`
/// `re.compile(r'^[0-9]+: ?')`.
#[allow(non_snake_case)]
pub fn WORKSPACE_REGEX() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[0-9]+: ?").unwrap())
}

/// Workspace flag triple used by `workspace_groups`.
///
/// Python passes the live `i3ipc.Workspace` object and reads its
/// `.focused`/`.urgent`/`.visible` attributes; Rust port takes the
/// flags directly.
#[derive(Debug, Clone, Copy)]
pub struct WorkspaceFlags {
    pub focused: bool,
    pub urgent: bool,
    pub visible: bool,
}

/// Port of `workspace_groups()` from
/// `powerline/segments/i3wm.py:11`.
///
/// Returns the list of highlight-group names matching the workspace's
/// state.
pub fn workspace_groups(w: WorkspaceFlags) -> Vec<String> {
    // py:11  def workspace_groups(w):
    // py:12  group = []
    let mut group: Vec<String> = Vec::new();
    // py:13  if w.focused:
    // py:14  group.append('workspace:focused')
    // py:15  group.append('w_focused')
    if w.focused {
        group.push("workspace:focused".to_string());
        group.push("w_focused".to_string());
    }
    // py:16  if w.urgent:
    // py:17  group.append('workspace:urgent')
    // py:18  group.append('w_urgent')
    if w.urgent {
        group.push("workspace:urgent".to_string());
        group.push("w_urgent".to_string());
    }
    // py:19  if w.visible:
    // py:20  group.append('workspace:visible')
    // py:21  group.append('w_visible')
    if w.visible {
        group.push("workspace:visible".to_string());
        group.push("w_visible".to_string());
    }
    // py:22  group.append('workspace')
    // py:23  return group
    group.push("workspace".to_string());
    group
}

/// Port of `format_name()` from
/// `powerline/segments/i3wm.py:26`.
///
/// When `strip == true`, removes the leading `<digits>: ?` prefix.
pub fn format_name(name: &str, strip: bool) -> String {
    // py:26  def format_name(name, strip=False):
    // py:27  if strip:
    // py:28  return WORKSPACE_REGEX.sub('', name, count=1)
    if strip {
        WORKSPACE_REGEX().replace(name, "").into_owned()
    } else {
        // py:29  return name
        name.to_string()
    }
}

/// Per-workspace container summary used by `is_empty_workspace` and
/// `get_icon`. Mirrors the fields they read from `i3ipc` Container
/// objects.
#[derive(Debug, Clone)]
pub struct WorkspaceContainer {
    /// Window classes of the leaves in this workspace.
    pub window_classes: Vec<String>,
    /// `scratchpad_state` for each leaf (parallel to window_classes).
    pub scratchpad_states: Vec<String>,
}

/// Port of `is_empty_workspace()` from
/// `powerline/segments/i3wm.py:33`.
///
/// Returns true if the workspace is not focused / not visible AND has
/// no leaves.
pub fn is_empty_workspace(w: WorkspaceFlags, container: &WorkspaceContainer) -> bool {
    // py:32  def is_empty_workspace(workspace, containers):
    // py:33  if workspace.focused or workspace.visible:
    // py:34  return False
    if w.focused || w.visible {
        return false;
    }
    // py:35  wins = [win for win in containers[workspace.name].leaves()]
    // py:36  return False if len(wins) > 0 else True
    container.window_classes.is_empty()
}

/// Port of `WS_ICONS` from
/// `powerline/segments/i3wm.py:39`
/// `WS_ICONS = {"multiple": "M"}`.
pub fn ws_icons() -> Map<String, Value> {
    // py:39  WS_ICONS = {"multiple": "M"}
    let mut m = Map::new();
    m.insert("multiple".to_string(), Value::String("M".into()));
    m
}

/// Port of `get_icon()` from
/// `powerline/segments/i3wm.py:41`.
///
/// Returns the icon string for a workspace based on the windows
/// present and the icons table. Honors `show_multiple_icons` for the
/// `multiple` collapsed fallback.
pub fn get_icon(
    container: &WorkspaceContainer,
    separator: &str,
    icons: &Map<String, Value>,
    show_multiple_icons: bool,
) -> String {
    // py:40  def get_icon(workspace, separator, icons, show_multiple_icons, ws_containers):
    // py:41  icons_tmp = WS_ICONS
    // py:42  icons_tmp.update(icons)
    // py:43  icons = icons_tmp
    let mut icons_merged = ws_icons();
    for (k, v) in icons {
        icons_merged.insert(k.clone(), v.clone());
    }
    // py:45  wins = [win for win in ws_containers[workspace.name].leaves() \
    // py:46  if win.parent.scratchpad_state == 'none']
    let wins: Vec<&String> = container
        .window_classes
        .iter()
        .zip(container.scratchpad_states.iter())
        .filter_map(|(wc, ss)| if ss == "none" { Some(wc) } else { None })
        .collect();
    // py:47  if len(wins) == 0:
    // py:48  return ''
    if wins.is_empty() {
        return String::new();
    }
    // py:50  result = ''
    // py:51  cnt = 0
    let mut result = String::new();
    let mut cnt: u32 = 0;
    // py:52  for key in icons:
    for (key, val_v) in &icons_merged {
        let val = val_v.as_str().unwrap_or("");
        // py:53  if not icons[key] or len(icons[key]) < 1:
        // py:54  continue
        if val.is_empty() {
            continue;
        }
        // py:55  if any(key in win.window_class for win in wins if win.window_class):
        if wins.iter().any(|wc| !wc.is_empty() && wc.contains(key)) {
            // py:56  result += (separator if cnt > 0 else '') + icons[key]
            // py:57  cnt += 1
            if cnt > 0 {
                result.push_str(separator);
            }
            result.push_str(val);
            cnt += 1;
        }
    }
    // py:58  if not show_multiple_icons and cnt > 1:
    // py:59  if 'multiple' in icons:
    // py:60  return icons['multiple']
    // py:61  else:
    // py:62  return ''
    if !show_multiple_icons && cnt > 1 {
        if let Some(multi) = icons_merged.get("multiple").and_then(|v| v.as_str()) {
            return multi.to_string();
        }
        return String::new();
    }
    // py:63  return result
    result
}

/// Port of `mode()` segment from
/// `powerline/segments/i3wm.py:243`.
///
/// Alias for [`mode_segment`] preserving the upstream Python name
/// byte-for-byte. The disambiguated `_segment` suffix exists to
/// avoid collisions with other `mode` identifiers across the
/// codebase; this fn surfaces the bare-name shape callers expect.
pub fn mode(current_mode: &str, names: &Map<String, Value>) -> Option<String> {
    mode_segment(current_mode, names)
}

/// Port of the inner `sort_ws()` closure from
/// `powerline/segments/i3wm.py:123-132`.
///
/// Sorts a workspace name list according to:
///   - py:124-128  `natural_key` ordering (digit runs as integers,
///     alpha runs as strings) when `sort_workspaces=true`.
///   - py:130-132  priority entries pinned to the front in the
///     order specified by `priority_workspaces`.
///
/// Returns the resorted name list. Python's `sort_ws` captures
/// `sort_workspaces` and `priority_workspaces` from the outer
/// `workspaces()` scope; the Rust port takes them as explicit
/// arguments since closure capture across module boundaries isn't
/// available.
pub fn sort_ws(
    ws: &[String],
    sort_workspaces: bool,
    priority_workspaces: &[String],
) -> Vec<String> {
    // py:123  def sort_ws(ws):
    // py:124  if sort_workspaces:
    let working: Vec<String> = if sort_workspaces {
        // py:125-128  ws = sorted(ws, key=natural_key)
        let mut v: Vec<String> = ws.to_vec();
        v.sort_by_key(|a| natural_key(a));
        v
    } else {
        ws.to_vec()
    };
    // py:130-132  priority pin + tail
    priority_sort_workspaces(&working, priority_workspaces)
}

/// Port of `mode()` segment from
/// `powerline/segments/i3wm.py:243`.
///
/// Returns the translated mode name or None when mapped to null.
/// `names` defaults to `{"default": null}` per py:243.
pub fn mode_segment(current_mode: &str, names: &Map<String, Value>) -> Option<String> {
    // py:242  @requires_segment_info
    // py:243  def mode(pl, segment_info, names={'default': None}):
    // py:244-251  docstring
    // py:252  mode = segment_info['mode']
    // py:253  if mode in names:
    // py:254  return names[mode]
    if let Some(translation) = names.get(current_mode) {
        match translation {
            Value::Null => None,
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        }
    } else {
        // py:255  return mode
        Some(current_mode.to_string())
    }
}

/// Scratchpad-window flag bundle used by `scratchpad_groups`.
#[derive(Debug, Clone)]
pub struct ScratchpadFlags {
    pub urgent: bool,
    /// Python: `w.nodes[0].focused`.
    pub first_node_focused: bool,
    /// Python: `w.workspace().name`.
    pub workspace_name: String,
}

/// Port of `scratchpad_groups()` from
/// `powerline/segments/i3wm.py:260`.
pub fn scratchpad_groups(w: &ScratchpadFlags) -> Vec<String> {
    // py:258  def scratchpad_groups(w):
    // py:259  group = []
    let mut group: Vec<String> = Vec::new();
    // py:260  if w.urgent:
    // py:261  group.append('scratchpad:urgent')
    if w.urgent {
        group.push("scratchpad:urgent".to_string());
    }
    // py:262  if w.nodes[0].focused:
    // py:263  group.append('scratchpad:focused')
    if w.first_node_focused {
        group.push("scratchpad:focused".to_string());
    }
    // py:264  if w.workspace().name != '__i3_scratch':
    // py:265  group.append('scratchpad:visible')
    if w.workspace_name != "__i3_scratch" {
        group.push("scratchpad:visible".to_string());
    }
    // py:266  group.append('scratchpad')
    // py:267  return group
    group.push("scratchpad".to_string());
    group
}

/// Port of `SCRATCHPAD_ICONS` from
/// `powerline/segments/i3wm.py:272`.
pub fn scratchpad_icons() -> Map<String, Value> {
    // py:272-274  {'fresh': 'O', 'changed': 'X'}
    let mut m = Map::new();
    m.insert("fresh".to_string(), Value::String("O".into()));
    m.insert("changed".to_string(), Value::String("X".into()));
    m
}

/// Helper for the `active_window` segment: given the focused
/// window's title + class + cutoff, returns the title or the class
/// (when title exceeds cutoff). Mirrors the
/// `powerline/segments/i3wm.py:295-302` logic.
pub fn active_window_contents(title: &str, window_class: &str, cutoff: usize) -> String {
    // py:295  @requires_segment_info
    // py:296  def active_window(pl, segment_info, cutoff=100):
    // py:297  '''Returns the title of the currently active window
    // py:298
    // py:299  :param int cutoff:
    // py:300  Maximum title length. If the title is longer, the window_class is used instead.
    // py:301  '''
    // py:302  current_workspace = next((ws for ws in get_i3_connection().get_workspaces() if ws.focused))
    // py:303  return current_workspace.name if title and len(title) > cutoff else title
    if title.chars().count() > cutoff {
        window_class.to_string()
    } else {
        title.to_string()
    }
}

/// Port of the inline natural-sort key from
/// `powerline/segments/i3wm.py:125-127` (defined inside
/// `sort_ws()`).
///
/// Python:
/// ```python
/// def natural_key(ws):
///     str = ws.name
///     return [int(s) if s.isdigit() else s for s in re.split(r'(\d+)', str)]
/// ```
///
/// Splits a workspace name into alternating digit/non-digit chunks
/// so that "ws10" sorts after "ws2". Rust port returns a Vec of
/// owned strings since we can't return a Python-style mixed
/// `[int|str]` list; callers compare lexicographically with
/// numeric ordering preserved by zero-padding digit groups.
pub fn natural_key(name: &str) -> Vec<String> {
    // py:123  def sort_ws(ws):
    // py:124  if sort_workspaces:
    // py:125  def natural_key(ws):
    // py:126  str = ws.name
    // py:127  return [int(s) if s.isdigit() else s for s in re.split(r'(\d+)', str)]
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"(\d+)").unwrap());
    let mut out: Vec<String> = Vec::new();
    let mut last_end = 0;
    for m in re.find_iter(name) {
        if m.start() > last_end {
            out.push(name[last_end..m.start()].to_string());
        }
        out.push(format!("{:0>10}", m.as_str()));
        last_end = m.end();
    }
    if last_end < name.len() {
        out.push(name[last_end..].to_string());
    }
    out
}

/// Port of `sort_ws()` priority-prefix logic from
/// `powerline/segments/i3wm.py:129-132`.
///
/// Reorders `workspaces` so that any names in `priority_names`
/// appear first in the listed order, with remaining workspaces
/// following in their original relative order.
pub fn priority_sort_workspaces(workspaces: &[String], priority_names: &[String]) -> Vec<String> {
    // py:128  ws = sorted(ws, key=natural_key)
    // py:129  result = []
    let mut result: Vec<String> = Vec::new();
    // py:130  for n in priority_workspaces:
    // py:131  result += [w for w in ws if w.name == n]
    for priority in priority_names {
        for w in workspaces {
            if w == priority {
                result.push(w.clone());
            }
        }
    }
    // py:132  return result + [w for w in ws if not w.name in priority_workspaces]
    for w in workspaces {
        if !priority_names.contains(w) {
            result.push(w.clone());
        }
    }
    result
}

/// Port of the `format` keyword default at
/// `powerline/segments/i3wm.py:207-208` (inside `workspace()`).
///
/// Python:
/// ```python
/// if format == None:
///     format = '{stripped_name}' if strip else '{name}'
/// ```
pub fn workspace_default_format(strip: bool) -> &'static str {
    // py:207-208
    if strip {
        "{stripped_name}"
    } else {
        "{name}"
    }
}

/// Port of `scratchpad()` per-window entry builder from
/// `powerline/segments/i3wm.py:286-292`.
///
/// Returns the segment dict for one scratchpad window:
/// `{'contents': icons[state] (defaulting to 'changed'),
///   'highlight_groups': scratchpad_groups(w)}`.
/// Returns None when the window's scratchpad_state is `'none'`
/// per py:292.
pub fn scratchpad_entry(
    scratchpad_state: &str,
    flags: &ScratchpadFlags,
    icons: &Map<String, Value>,
) -> Option<Value> {
    // py:292  if w.scratchpad_state != 'none'
    if scratchpad_state == "none" {
        return None;
    }
    // py:288  icons.get(w.scratchpad_state, icons['changed'])
    let contents = icons
        .get(scratchpad_state)
        .and_then(|v| v.as_str())
        .map(String::from)
        .or_else(|| {
            icons
                .get("changed")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .unwrap_or_default();
    // py:289  scratchpad_groups(w)
    let groups: Vec<Value> = scratchpad_groups(flags)
        .into_iter()
        .map(Value::String)
        .collect();
    Some(json!({
        "contents": contents,
        "highlight_groups": groups,
    }))
}

/// Port of `active_window()` segment from
/// `powerline/segments/i3wm.py:295-302`.
///
/// Wraps `active_window_contents` into the segment-shape result
/// expected by the segment dispatcher. Returns the contents (or
/// window_class fallback) as a plain string per py:302.
pub fn active_window(title: &str, window_class: &str, cutoff: usize) -> String {
    // py:295-302
    active_window_contents(title, window_class, cutoff)
}

/// Builder for one entry of the workspace segment output. Mirrors
/// the `format.format(name=..., stripped_name=..., number=..., icon=...,
/// multi_icon=...)` call inside the `workspaces` segment (py:140-150).
pub fn build_workspace_entry(
    format: &str,
    name: &str,
    number: i64,
    container: &WorkspaceContainer,
    icons: &Map<String, Value>,
    strip: usize,
) -> Value {
    // py:143  res += [{
    // py:144  'contents': format.format(name = w.name[min(len(w.name), strip):],
    // py:145  stripped_name = format_name(w.name, strip=True),
    // py:146  number = w.num,
    // py:147  icon = get_icon(w, '', icons, False, ws_containers),
    // py:148  multi_icon = get_icon(w, ' ', icons, True, ws_containers)),
    // py:149  'highlight_groups': workspace_groups(w)
    // py:150  } for w in sort_ws(conn.get_workspaces()) \
    let stripped_idx = std::cmp::min(name.chars().count(), strip);
    let trimmed_name: String = name.chars().skip(stripped_idx).collect();
    let stripped_name = format_name(name, true);
    let icon = get_icon(container, "", icons, false);
    let multi_icon = get_icon(container, " ", icons, true);
    let contents = format
        .replace("{name}", &trimmed_name)
        .replace("{stripped_name}", &stripped_name)
        .replace("{number}", &number.to_string())
        .replace("{icon}", &icon)
        .replace("{multi_icon}", &multi_icon);
    json!({
        "contents": contents,
    })
}

/// Port of `workspace()` segment from
/// `powerline/segments/i3wm.py:177-238`.
///
/// Returns the specified workspace's segment list. Python uses
/// `get_i3_connection()` + `conn.get_workspaces()` to resolve the
/// target workspace; Rust port takes the resolved workspace data
/// as args since the i3ipc connection isn't reachable.
///
/// `target` is the resolved workspace (`name`, `num`, `flags`,
/// `icon`, `multi_icon`); `format` is the format-string template
/// (default per [`workspace_default_format`]). Returns the
/// segment dict with `contents` per the format substitution and
/// `highlight_groups` per [`workspace_groups`].
pub fn workspace(
    name: &str,
    num: i64,
    flags: WorkspaceFlags,
    icon: &str,
    multi_icon: &str,
    strip: bool,
    format: Option<&str>,
) -> Map<String, Value> {
    // py:207  if format == None: format = '{stripped_name}' if strip else '{name}'
    let f = format.unwrap_or_else(|| workspace_default_format(strip));
    // py:232-236  build segment dict — substitute the format placeholders directly.
    let contents = f
        .replace("{name}", name)
        .replace("{stripped_name}", &format_name(name, true))
        .replace("{number}", &num.to_string())
        .replace("{icon}", icon)
        .replace("{multi_icon}", multi_icon);
    let mut seg = Map::new();
    seg.insert("contents".to_string(), Value::String(contents));
    seg.insert(
        "highlight_groups".to_string(),
        Value::Array(
            workspace_groups(flags)
                .into_iter()
                .map(Value::String)
                .collect(),
        ),
    );
    seg
}

/// Port of `scratchpad()` segment from
/// `powerline/segments/i3wm.py:276-293`.
///
/// Returns the segment list for windows currently on the i3
/// scratchpad. Python uses
/// `get_i3_connection().get_tree().descendants()` + filters by
/// `w.scratchpad_state != 'none'`. Rust port takes the pre-
/// filtered window list since i3ipc isn't reachable.
///
/// Each input tuple is `(scratchpad_state, flags)`. Output dicts
/// have `contents` from `icons[state]` (defaulting to `'changed'`
/// per py:288) and `highlight_groups` per [`scratchpad_groups`].
pub fn scratchpad(windows: &[(&str, ScratchpadFlags)], icons: &Map<String, Value>) -> Vec<Value> {
    // py:276  def scratchpad(pl, icons=SCRATCHPAD_ICONS):
    // py:286-293  list comprehension
    let mut out: Vec<Value> = Vec::with_capacity(windows.len());
    for (state, flags) in windows {
        if let Some(entry) = scratchpad_entry(state, flags, icons) {
            out.push(entry);
        }
    }
    out
}

/// Port of `workspaces()` segment body trace from
/// `powerline/segments/i3wm.py:65-174`.
pub fn workspaces() -> &'static str {
    // py:65  @requires_segment_info
    // py:66  def workspaces(pl, segment_info, only_show=None, output=None, strip=0, format='{name}',
    // py:67  icons=WS_ICONS, sort_workspaces=False, show_output=False, priority_workspaces=[],
    // py:68  hide_empty_workspaces=False):
    // py:69-109  docstring
    // py:110  conn = get_i3_connection()
    // py:112  if not output == "__all__":
    // py:113  output = output or segment_info.get('output')
    // py:114  else:
    // py:115  output = None
    // py:117  if output:
    // py:118  output = [output]
    // py:119  else:
    // py:120  output = [o.name for o in conn.get_outputs() if o.active]
    // py:134  ws_containers = {w_con.name : w_con for w_con in conn.get_tree().workspaces()}
    // py:136  if len(output) <= 1:
    // py:137  res = []
    // py:138  if show_output:
    // py:139  res += [{
    // py:140  'contents': output[0],
    // py:141  'highlight_groups': ['output']
    // py:142  }]
    // py:151  if (not only_show or any(getattr(w, tp) for tp in only_show)) \
    // py:152  if w.output == output[0] \
    // py:153  if not (hide_empty_workspaces and is_empty_workspace(w, ws_containers))]
    // py:154  return res
    // py:155  else:
    // py:156  res = []
    // py:157  for n in output:
    // py:170  for w in sort_ws(conn.get_workspaces()) \
    // py:171  if (not only_show or any(getattr(w, tp) for tp in only_show)) \
    // py:172  if w.output == n \
    // py:173  if not (hide_empty_workspaces and is_empty_workspace(w, ws_containers))]
    // py:174  return res
    "see powerline/segments/i3wm.py:65-174"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_regex_strips_leading_number_and_optional_space() {
        assert_eq!(WORKSPACE_REGEX().replace("1: web", ""), "web");
        assert_eq!(WORKSPACE_REGEX().replace("23:term", ""), "term");
        assert_eq!(WORKSPACE_REGEX().replace("plain", ""), "plain");
    }

    #[test]
    fn workspace_groups_focused_only() {
        let g = workspace_groups(WorkspaceFlags {
            focused: true,
            urgent: false,
            visible: false,
        });
        assert_eq!(g, vec!["workspace:focused", "w_focused", "workspace"]);
    }

    #[test]
    fn workspace_groups_urgent_only() {
        let g = workspace_groups(WorkspaceFlags {
            focused: false,
            urgent: true,
            visible: false,
        });
        assert_eq!(g, vec!["workspace:urgent", "w_urgent", "workspace"]);
    }

    #[test]
    fn workspace_groups_visible_only() {
        let g = workspace_groups(WorkspaceFlags {
            focused: false,
            urgent: false,
            visible: true,
        });
        assert_eq!(g, vec!["workspace:visible", "w_visible", "workspace"]);
    }

    #[test]
    fn workspace_groups_none_returns_just_workspace() {
        let g = workspace_groups(WorkspaceFlags {
            focused: false,
            urgent: false,
            visible: false,
        });
        assert_eq!(g, vec!["workspace"]);
    }

    #[test]
    fn workspace_groups_all_flags_set_returns_all_classes() {
        let g = workspace_groups(WorkspaceFlags {
            focused: true,
            urgent: true,
            visible: true,
        });
        assert_eq!(
            g,
            vec![
                "workspace:focused",
                "w_focused",
                "workspace:urgent",
                "w_urgent",
                "workspace:visible",
                "w_visible",
                "workspace"
            ]
        );
    }

    #[test]
    fn format_name_strip_true_removes_leading_number() {
        // py:27-28  WORKSPACE_REGEX.sub('', name, count=1)
        assert_eq!(format_name("1: web", true), "web");
        assert_eq!(format_name("42:term", true), "term");
    }

    #[test]
    fn format_name_strip_false_passes_through() {
        assert_eq!(format_name("1: web", false), "1: web");
    }

    #[test]
    fn is_empty_workspace_focused_returns_false() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let r = is_empty_workspace(
            WorkspaceFlags {
                focused: true,
                urgent: false,
                visible: false,
            },
            &c,
        );
        assert!(!r);
    }

    #[test]
    fn is_empty_workspace_visible_returns_false() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let r = is_empty_workspace(
            WorkspaceFlags {
                focused: false,
                urgent: false,
                visible: true,
            },
            &c,
        );
        assert!(!r);
    }

    #[test]
    fn is_empty_workspace_with_leaves_returns_false() {
        let c = WorkspaceContainer {
            window_classes: vec!["Firefox".to_string()],
            scratchpad_states: vec!["none".to_string()],
        };
        let r = is_empty_workspace(
            WorkspaceFlags {
                focused: false,
                urgent: false,
                visible: false,
            },
            &c,
        );
        assert!(!r);
    }

    #[test]
    fn is_empty_workspace_unfocused_invisible_no_leaves_returns_true() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let r = is_empty_workspace(
            WorkspaceFlags {
                focused: false,
                urgent: false,
                visible: false,
            },
            &c,
        );
        assert!(r);
    }

    #[test]
    fn ws_icons_contains_multiple_entry() {
        // py:39  WS_ICONS = {"multiple": "M"}
        let i = ws_icons();
        assert_eq!(i.get("multiple"), Some(&Value::String("M".into())));
    }

    #[test]
    fn get_icon_no_windows_returns_empty() {
        // py:49-50  if len(wins) == 0: return ''
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let icons = ws_icons();
        assert_eq!(get_icon(&c, "", &icons, false), "");
    }

    #[test]
    fn get_icon_matches_window_class_substring() {
        // py:55-57  if any(key in win.window_class for win in wins ...)
        let c = WorkspaceContainer {
            window_classes: vec!["Firefox".to_string()],
            scratchpad_states: vec!["none".to_string()],
        };
        let mut icons = Map::new();
        icons.insert("Firefox".to_string(), Value::String("🦊".into()));
        let r = get_icon(&c, "", &icons, true);
        assert_eq!(r, "🦊");
    }

    #[test]
    fn get_icon_collapses_multiple_to_multi_icon_when_not_show_all() {
        // py:58-62  if not show_multiple_icons and cnt > 1: return icons['multiple']
        let c = WorkspaceContainer {
            window_classes: vec!["Firefox".to_string(), "Terminal".to_string()],
            scratchpad_states: vec!["none".to_string(), "none".to_string()],
        };
        let mut icons = Map::new();
        icons.insert("Firefox".to_string(), Value::String("🦊".into()));
        icons.insert("Terminal".to_string(), Value::String("⚙".into()));
        let r = get_icon(&c, "", &icons, false);
        assert_eq!(r, "M"); // py:39 WS_ICONS multiple = 'M'
    }

    #[test]
    fn get_icon_skips_scratchpad_windows() {
        // py:46-48  wins = ... if win.parent.scratchpad_state == 'none'
        let c = WorkspaceContainer {
            window_classes: vec!["Hidden".to_string()],
            scratchpad_states: vec!["fresh".to_string()],
        };
        let mut icons = Map::new();
        icons.insert("Hidden".to_string(), Value::String("H".into()));
        let r = get_icon(&c, "", &icons, true);
        assert_eq!(r, "");
    }

    #[test]
    fn mode_segment_default_hidden_returns_none() {
        // py:248  names = {'default': None}
        let mut names = Map::new();
        names.insert("default".to_string(), Value::Null);
        assert_eq!(mode_segment("default", &names), None);
    }

    #[test]
    fn mode_segment_named_returns_translation() {
        let mut names = Map::new();
        names.insert("resize".to_string(), Value::String("[RESIZE]".into()));
        assert_eq!(mode_segment("resize", &names), Some("[RESIZE]".to_string()));
    }

    #[test]
    fn mode_segment_unknown_passes_through() {
        // py:254-255  return mode
        let names = Map::new();
        assert_eq!(mode_segment("normal", &names), Some("normal".to_string()));
    }

    #[test]
    fn scratchpad_groups_urgent_focused_visible() {
        // py:261-268
        let g = scratchpad_groups(&ScratchpadFlags {
            urgent: true,
            first_node_focused: true,
            workspace_name: "1: web".to_string(),
        });
        assert_eq!(
            g,
            vec![
                "scratchpad:urgent",
                "scratchpad:focused",
                "scratchpad:visible",
                "scratchpad"
            ]
        );
    }

    #[test]
    fn scratchpad_groups_on_scratch_workspace_omits_visible() {
        // py:266-267  if w.workspace().name != '__i3_scratch': append visible
        let g = scratchpad_groups(&ScratchpadFlags {
            urgent: false,
            first_node_focused: false,
            workspace_name: "__i3_scratch".to_string(),
        });
        assert_eq!(g, vec!["scratchpad"]);
    }

    #[test]
    fn scratchpad_icons_contains_fresh_and_changed() {
        let i = scratchpad_icons();
        assert_eq!(i.get("fresh"), Some(&Value::String("O".into())));
        assert_eq!(i.get("changed"), Some(&Value::String("X".into())));
    }

    #[test]
    fn active_window_returns_title_when_short() {
        // py:295-302  if len(cont) > cutoff: window_class
        let r = active_window_contents("My Window", "Firefox", 100);
        assert_eq!(r, "My Window");
    }

    #[test]
    fn active_window_returns_class_when_title_too_long() {
        let r = active_window_contents("very very long title", "Firefox", 5);
        assert_eq!(r, "Firefox");
    }

    #[test]
    fn build_workspace_entry_substitutes_name() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let icons = ws_icons();
        let e = build_workspace_entry("{name}", "web", 1, &c, &icons, 0);
        assert_eq!(e["contents"], "web");
    }

    #[test]
    fn build_workspace_entry_substitutes_stripped_name() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let icons = ws_icons();
        let e = build_workspace_entry("{stripped_name}", "1: web", 1, &c, &icons, 0);
        assert_eq!(e["contents"], "web");
    }

    #[test]
    fn build_workspace_entry_substitutes_number() {
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let icons = ws_icons();
        let e = build_workspace_entry("{number}", "1: web", 7, &c, &icons, 0);
        assert_eq!(e["contents"], "7");
    }

    #[test]
    fn build_workspace_entry_strip_skips_leading_chars() {
        // py:140  name = w.name[min(len(w.name), strip):]
        let c = WorkspaceContainer {
            window_classes: Vec::new(),
            scratchpad_states: Vec::new(),
        };
        let icons = ws_icons();
        // strip=3 → skip leading 3 chars
        let e = build_workspace_entry("{name}", "1: web", 1, &c, &icons, 3);
        assert_eq!(e["contents"], "web");
    }

    #[test]
    fn natural_key_orders_digits_numerically() {
        // py:125-127  digits should sort numerically not lexicographically
        let k2 = natural_key("ws2");
        let k10 = natural_key("ws10");
        assert!(k2 < k10);
    }

    #[test]
    fn natural_key_handles_pure_digits() {
        let k1 = natural_key("1");
        let k10 = natural_key("10");
        assert!(k1 < k10);
    }

    #[test]
    fn natural_key_handles_no_digits() {
        let k = natural_key("web");
        assert_eq!(k, vec!["web".to_string()]);
    }

    #[test]
    fn natural_key_alternating_chunks() {
        let k = natural_key("ws10:dev");
        // chunks: "ws", "10", ":dev"
        assert_eq!(k.len(), 3);
        assert_eq!(k[0], "ws");
        assert_eq!(k[2], ":dev");
        // digit chunk zero-padded
        assert!(k[1].ends_with("10"));
    }

    #[test]
    fn priority_sort_workspaces_puts_priority_names_first() {
        // py:130-132
        let ws = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        let priority = vec!["c".to_string(), "a".to_string()];
        let r = priority_sort_workspaces(&ws, &priority);
        assert_eq!(r, vec!["c", "a", "b", "d"]);
    }

    #[test]
    fn priority_sort_workspaces_empty_priority_preserves_order() {
        let ws = vec!["x".to_string(), "y".to_string(), "z".to_string()];
        let r = priority_sort_workspaces(&ws, &[]);
        assert_eq!(r, vec!["x", "y", "z"]);
    }

    #[test]
    fn priority_sort_workspaces_priority_not_in_list_is_skipped() {
        let ws = vec!["a".to_string(), "b".to_string()];
        let priority = vec!["missing".to_string()];
        let r = priority_sort_workspaces(&ws, &priority);
        assert_eq!(r, vec!["a", "b"]);
    }

    #[test]
    fn sort_ws_sort_workspaces_true_applies_natural_key_order() {
        // py:124-128
        let ws = vec!["10".to_string(), "2".to_string(), "1".to_string()];
        let r = sort_ws(&ws, true, &[]);
        assert_eq!(r, vec!["1", "2", "10"]);
    }

    #[test]
    fn sort_ws_sort_workspaces_false_preserves_input_order() {
        // py:124  if not sort_workspaces → keep order
        let ws = vec!["10".to_string(), "2".to_string(), "1".to_string()];
        let r = sort_ws(&ws, false, &[]);
        assert_eq!(r, vec!["10", "2", "1"]);
    }

    #[test]
    fn sort_ws_priority_workspaces_pin_to_front() {
        // py:130-132 combined with natural sort
        let ws = vec!["c".to_string(), "a".to_string(), "b".to_string()];
        let priority = vec!["b".to_string()];
        let r = sort_ws(&ws, true, &priority);
        assert_eq!(r, vec!["b", "a", "c"]);
    }

    #[test]
    fn mode_alias_dispatches_to_mode_segment() {
        let mut names = Map::new();
        names.insert("v".to_string(), Value::String("VIS".to_string()));
        assert_eq!(mode("v", &names), Some("VIS".to_string()));
        assert_eq!(mode("unmapped", &names), Some("unmapped".to_string()));
    }

    #[test]
    fn workspace_default_format_strip_returns_stripped_name() {
        // py:208
        assert_eq!(workspace_default_format(true), "{stripped_name}");
    }

    #[test]
    fn workspace_default_format_no_strip_returns_name() {
        assert_eq!(workspace_default_format(false), "{name}");
    }

    #[test]
    fn scratchpad_entry_none_state_returns_none() {
        // py:292  if w.scratchpad_state != 'none'
        let flags = ScratchpadFlags {
            urgent: false,
            first_node_focused: true,
            workspace_name: "1".to_string(),
        };
        let icons = scratchpad_icons();
        assert!(scratchpad_entry("none", &flags, &icons).is_none());
    }

    #[test]
    fn scratchpad_entry_known_state_uses_matching_icon() {
        // py:288  icons.get(state)
        let flags = ScratchpadFlags {
            urgent: false,
            first_node_focused: false,
            workspace_name: "__i3_scratch".to_string(),
        };
        let icons = scratchpad_icons();
        let entry = scratchpad_entry("fresh", &flags, &icons).unwrap();
        assert_eq!(entry["contents"], "O");
    }

    #[test]
    fn scratchpad_entry_unknown_state_falls_back_to_changed_icon() {
        // py:288  icons.get(state, icons['changed'])
        let flags = ScratchpadFlags {
            urgent: false,
            first_node_focused: false,
            workspace_name: "__i3_scratch".to_string(),
        };
        let icons = scratchpad_icons();
        let entry = scratchpad_entry("bogus_state", &flags, &icons).unwrap();
        assert_eq!(entry["contents"], "X");
    }

    #[test]
    fn scratchpad_entry_emits_highlight_groups_from_flags() {
        // py:289
        let flags = ScratchpadFlags {
            urgent: true,
            first_node_focused: false,
            workspace_name: "__i3_scratch".to_string(),
        };
        let icons = scratchpad_icons();
        let entry = scratchpad_entry("changed", &flags, &icons).unwrap();
        let groups = entry["highlight_groups"].as_array().unwrap();
        let strs: Vec<&str> = groups.iter().filter_map(|v| v.as_str()).collect();
        assert!(strs.contains(&"scratchpad:urgent"));
        assert!(strs.contains(&"scratchpad"));
    }

    #[test]
    fn active_window_returns_title_under_cutoff() {
        // py:295-302
        assert_eq!(active_window("short", "Class", 100), "short");
    }

    #[test]
    fn active_window_returns_class_when_title_exceeds_cutoff() {
        let long_title = "a".repeat(200);
        assert_eq!(active_window(&long_title, "MyClass", 100), "MyClass");
    }

    #[test]
    fn workspace_uses_supplied_format_string() {
        // py:207-208 / py:232-236
        let flags = WorkspaceFlags {
            focused: true,
            urgent: false,
            visible: false,
        };
        let seg = workspace("1: web", 1, flags, "🌐", "🌐 ", false, Some("{name}"));
        assert_eq!(seg["contents"], "1: web");
        let groups = seg["highlight_groups"].as_array().unwrap();
        assert!(groups.iter().any(|g| g == "workspace:focused"));
    }

    #[test]
    fn workspace_substitutes_stripped_name_when_strip_true() {
        // py:144-145  stripped_name = format_name(w.name, strip=True)
        let flags = WorkspaceFlags {
            focused: false,
            urgent: false,
            visible: false,
        };
        let seg = workspace("1: web", 1, flags, "", "", true, None);
        assert_eq!(seg["contents"], "web");
    }

    #[test]
    fn scratchpad_emits_one_entry_per_non_none_window() {
        // py:286-293
        let mut icons = Map::new();
        icons.insert("fresh".to_string(), Value::String("O".to_string()));
        icons.insert("changed".to_string(), Value::String("X".to_string()));
        let flags = ScratchpadFlags {
            urgent: false,
            first_node_focused: true,
            workspace_name: "1".to_string(),
        };
        let windows = vec![
            ("fresh", flags.clone()),
            ("changed", flags.clone()),
            ("none", flags),
        ];
        let r = scratchpad(&windows, &icons);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0]["contents"], "O");
        assert_eq!(r[1]["contents"], "X");
    }
}
