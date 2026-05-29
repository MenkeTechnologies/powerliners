// vim:fileencoding=utf-8:noet
//! Port of `powerline/listers/vim.py`.
//!
//! Vim listers: yield one subsegment per tabpage (`tablister`) or per
//! buffer (`bufferlister`). Used by the vim tabline renderer to
//! produce a per-tab statusline.
//!
//! Without a live vim connection, the listers return empty Vec's.
//! The `*_for` testing variants drive the filter / multiplier logic
//! against synthetic tabpage/buffer lists.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import requires_segment_info                                       // py:4
// from powerline.bindings.vim import (current_tabpage, list_tabpages)                     // py:5
// try: import vim except ImportError: vim = object()                                      // py:7-10

use crate::ported::bindings::vim::{current_tabpage, list_tabpages, VimBuffer, VimTabpage};
use serde_json::{json, Map, Value};

/// Port of `tabpage_updated_segment_info()` from
/// `powerline/listers/vim.py:13`.
///
/// Returns a copy of `segment_info` with tabpage-local
/// `tabpage`/`tabnr`/`window`/`winnr`/`window_id`/`buffer`/`bufnr`
/// keys populated.
pub fn tabpage_updated_segment_info(
    segment_info: &Map<String, Value>,
    tabpage: &VimTabpage,
) -> Map<String, Value> {
    // py:13  def tabpage_updated_segment_info(segment_info, tabpage):
    // py:14  segment_info = segment_info.copy()
    // py:15  window = tabpage.window
    // py:16  buffer = window.buffer
    // py:17  segment_info.update(
    // py:18  tabpage=tabpage,
    // py:19  tabnr=tabpage.number,
    // py:20  window=window,
    // py:21  winnr=window.number,
    // py:22  window_id=int(window.vars.get('powerline_window_id', -1)),
    // py:23  buffer=buffer,
    // py:24  bufnr=buffer.number,
    // py:25  )
    let mut info = segment_info.clone();
    info.insert("tabnr".to_string(), Value::from(tabpage.number));
    info.insert("winnr".to_string(), Value::from(tabpage.window.number));
    info.insert(
        "window_id".to_string(),
        Value::from(tabpage.window.window_id),
    );
    info.insert(
        "bufnr".to_string(),
        Value::from(tabpage.window.buffer.number),
    );
    // py:26  return segment_info
    info
}

/// Port of `tablister()` from `powerline/listers/vim.py:28`.
///
/// List all tab pages in segment_info format.
///
/// Adds either `tab:` or `tab_nc:` prefix to all segment highlight
/// groups depending on whether the tabpage is current.
///
/// Returns empty Vec when no live vim connection (list_tabpages is empty).
pub fn tablister(
    _pl: &(),
    segment_info: &Map<String, Value>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    tablister_for(segment_info, &list_tabpages(), Some(&current_tabpage()))
}

/// Port of the inner `add_multiplier()` closure from
/// `powerline/listers/vim.py:45-47` (inside `tablister`).
///
/// Computes the `priority_multiplier` per py:46 as
/// `1 + 0.001 * abs(tabpage.number - cur_tabnr)`, then mutates the
/// supplied dict and returns it (Python returns the mutated dict so
/// the comprehension at py:49-57 can chain it).
///
/// Python captures `cur_tabnr` from the outer scope; the Rust port
/// takes both numbers as explicit args.
pub fn add_multiplier(
    tabpage_number: i64,
    cur_tabnr: i64,
    dct: &mut Map<String, Value>,
) -> &mut Map<String, Value> {
    // py:45  def add_multiplier(tabpage, dct):
    // py:46  dct['priority_multiplier'] = 1 + (0.001 * abs(tabpage.number - cur_tabnr))
    let mult = 1.0 + 0.001 * (tabpage_number - cur_tabnr).abs() as f64;
    dct.insert(
        "priority_multiplier".to_string(),
        serde_json::Number::from_f64(mult)
            .map(Value::Number)
            .unwrap_or(Value::Null),
    );
    // py:47  return dct
    dct
}

/// Test-driveable variant of `tablister` — drives the prefix-and-
/// multiplier logic against a caller-supplied tabpage list.
pub fn tablister_for(
    segment_info: &Map<String, Value>,
    tabpages: &[VimTabpage],
    current: Option<&VimTabpage>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:29  @requires_segment_info
    // py:30  def tablister(pl, segment_info, **kwargs):
    // py:31-41  docstring
    // py:42  cur_tabpage = current_tabpage()
    // py:43  cur_tabnr = cur_tabpage.number
    let cur_tabnr = current.map(|c| c.number).unwrap_or(1);

    // py:45  def add_multiplier(tabpage, dct):
    // py:46  dct['priority_multiplier'] = 1 + (0.001 * abs(tabpage.number - cur_tabnr))
    // py:47  return dct
    // py:49  return (
    // py:50  (lambda tabpage, prefix: (
    // py:51  tabpage_updated_segment_info(segment_info, tabpage),
    // py:52  add_multiplier(tabpage, {
    // py:53  'highlight_group_prefix': prefix,
    // py:54  'divider_highlight_group': 'tab:divider'
    // py:55  })
    // py:56  ))(tabpage, 'tab' if tabpage == cur_tabpage else 'tab_nc')
    // py:57  for tabpage in list_tabpages()
    // py:58  )
    tabpages
        .iter()
        .map(|tabpage| {
            let prefix = if tabpage.number == cur_tabnr {
                "tab"
            } else {
                "tab_nc"
            };
            let multiplier = 1.0_f64 + 0.001 * ((tabpage.number - cur_tabnr).abs() as f64);
            let mut second = Map::new();
            second.insert(
                "highlight_group_prefix".to_string(),
                Value::String(prefix.to_string()),
            );
            second.insert(
                "divider_highlight_group".to_string(),
                Value::String("tab:divider".to_string()),
            );
            second.insert("priority_multiplier".to_string(), json!(multiplier));
            (tabpage_updated_segment_info(segment_info, tabpage), second)
        })
        .collect()
}

/// Port of `buffer_updated_segment_info()` from
/// `powerline/listers/vim.py:64`.
///
/// Returns a copy of `segment_info` with buffer-local keys populated
/// (window/winnr/window_id set to null per py:67-69, buffer/bufnr set
/// to the supplied buffer).
pub fn buffer_updated_segment_info(
    segment_info: &Map<String, Value>,
    buffer: &VimBuffer,
) -> Map<String, Value> {
    // py:61  def buffer_updated_segment_info(segment_info, buffer):
    // py:62  segment_info = segment_info.copy()
    // py:63  segment_info.update(
    // py:64  window=None,
    // py:65  winnr=None,
    // py:66  window_id=None,
    // py:67  buffer=buffer,
    // py:68  bufnr=buffer.number,
    // py:69  )
    let mut info = segment_info.clone();
    info.insert("window".to_string(), Value::Null);
    info.insert("winnr".to_string(), Value::Null);
    info.insert("window_id".to_string(), Value::Null);
    info.insert("bufnr".to_string(), Value::from(buffer.number));
    // py:70  return segment_info
    info
}

/// Port of `bufferlister()` from `powerline/listers/vim.py:77`.
///
/// List all buffers in segment_info format.
///
/// Adds one of `buf:`, `buf_nc:`, `buf_mod:`, or `buf_nc_mod`
/// prefix to all segment highlight groups.
///
/// :param show_unlisted: True if unlisted buffers should be shown as
///     well. Current buffer is always shown.
///
/// Without a live vim connection the buffers list is empty so the
/// result is empty.
pub fn bufferlister(
    _pl: &(),
    segment_info: &Map<String, Value>,
    _show_unlisted: bool,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // No vim connection → no buffers.
    bufferlister_for(segment_info, &[], None, false)
}

/// Test-driveable variant of `bufferlister`.
pub fn bufferlister_for(
    segment_info: &Map<String, Value>,
    buffers: &[VimBuffer],
    current: Option<&VimBuffer>,
    show_unlisted: bool,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:73  @requires_segment_info
    // py:74  def bufferlister(pl, segment_info, show_unlisted=False, **kwargs):
    // py:75-87  docstring
    // py:88  cur_buffer = vim.current.buffer
    // py:89  cur_bufnr = cur_buffer.number
    let cur_bufnr = current.map(|b| b.number).unwrap_or(1);
    let cur_bufnr_matched = current.map(|b| b.number);

    // py:91  def add_multiplier(buffer, dct):
    // py:92  dct['priority_multiplier'] = 1 + (0.001 * abs(buffer.number - cur_bufnr))
    // py:93  return dct
    // py:95  return (
    // py:96  (lambda buffer, current, modified: (
    // py:97  buffer_updated_segment_info(segment_info, buffer),
    // py:98  add_multiplier(buffer, {
    // py:99  'highlight_group_prefix': '{0}{1}'.format(current, modified),
    // py:100  'divider_highlight_group': 'tab:divider'
    // py:101  })
    // py:102  ))(
    // py:103  buffer,
    // py:104  'buf' if buffer is cur_buffer else 'buf_nc',
    // py:105  '_mod' if int(vim.eval('getbufvar({0}, \'&modified\')'.format(buffer.number))) > 0 else ''
    // py:106  )
    // py:107  for buffer in vim.buffers if (
    // py:108  buffer is cur_buffer
    // py:109  or show_unlisted
    // py:110-120  comment block
    // py:121  or int(vim.eval('buflisted(%s)' % buffer.number)) > 0
    // py:122  )
    // py:123  )
    buffers
        .iter()
        .filter(|buffer| {
            let is_current = Some(buffer.number) == cur_bufnr_matched;
            is_current || show_unlisted || buffer.listed
        })
        .map(|buffer| {
            let current_pfx = if Some(buffer.number) == cur_bufnr_matched {
                "buf"
            } else {
                "buf_nc"
            };
            let modified_pfx = if buffer.modified { "_mod" } else { "" };
            let prefix = format!("{}{}", current_pfx, modified_pfx);
            let multiplier = 1.0_f64 + 0.001 * ((buffer.number - cur_bufnr).abs() as f64);
            let mut second = Map::new();
            second.insert("highlight_group_prefix".to_string(), Value::String(prefix));
            second.insert(
                "divider_highlight_group".to_string(),
                Value::String("tab:divider".to_string()),
            );
            second.insert("priority_multiplier".to_string(), json!(multiplier));
            (buffer_updated_segment_info(segment_info, buffer), second)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(n: i32) -> VimTabpage {
        VimTabpage {
            number: n,
            window: crate::ported::bindings::vim::VimWindow {
                number: 1,
                window_id: -1,
                buffer: VimBuffer {
                    number: 1,
                    name: None,
                    modified: false,
                    listed: true,
                },
            },
        }
    }

    fn buf(n: i32, modified: bool, listed: bool) -> VimBuffer {
        VimBuffer {
            number: n,
            name: None,
            modified,
            listed,
        }
    }

    #[test]
    fn tablister_returns_empty_without_vim() {
        let seg = Map::new();
        let result = tablister(&(), &seg);
        assert!(result.is_empty());
    }

    #[test]
    fn tablister_for_marks_current_with_tab_prefix() {
        let seg = Map::new();
        let cur = tab(2);
        let tabs = vec![tab(1), tab(2), tab(3)];
        let result = tablister_for(&seg, &tabs, Some(&cur));
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0].1.get("highlight_group_prefix"),
            Some(&json!("tab_nc"))
        );
        assert_eq!(
            result[1].1.get("highlight_group_prefix"),
            Some(&json!("tab"))
        );
        assert_eq!(
            result[2].1.get("highlight_group_prefix"),
            Some(&json!("tab_nc"))
        );
    }

    #[test]
    fn tablister_for_priority_multiplier_grows_with_distance() {
        let seg = Map::new();
        let cur = tab(5);
        let tabs = vec![tab(1), tab(5), tab(10)];
        let result = tablister_for(&seg, &tabs, Some(&cur));
        // Distance 4 → 1.004; distance 0 → 1.0; distance 5 → 1.005
        let m0 = result[0]
            .1
            .get("priority_multiplier")
            .unwrap()
            .as_f64()
            .unwrap();
        let m1 = result[1]
            .1
            .get("priority_multiplier")
            .unwrap()
            .as_f64()
            .unwrap();
        let m2 = result[2]
            .1
            .get("priority_multiplier")
            .unwrap()
            .as_f64()
            .unwrap();
        assert!((m0 - 1.004).abs() < 1e-9);
        assert!((m1 - 1.0).abs() < 1e-9);
        assert!((m2 - 1.005).abs() < 1e-9);
    }

    #[test]
    fn bufferlister_for_filters_unlisted_unless_show_or_current() {
        let seg = Map::new();
        let cur = buf(2, false, true);
        let buffers = vec![
            buf(1, false, true),  // listed
            buf(2, false, true),  // current
            buf(3, false, false), // unlisted
        ];
        // Without show_unlisted: unlisted buffer #3 filtered out.
        let result = bufferlister_for(&seg, &buffers, Some(&cur), false);
        assert_eq!(result.len(), 2);

        // With show_unlisted: all 3 included.
        let result_all = bufferlister_for(&seg, &buffers, Some(&cur), true);
        assert_eq!(result_all.len(), 3);
    }

    #[test]
    fn bufferlister_for_prefix_includes_mod_when_buffer_modified() {
        let seg = Map::new();
        let cur = buf(1, false, true);
        let buffers = vec![
            buf(1, false, true),
            buf(2, true, true), // listed + modified, non-current
        ];
        let result = bufferlister_for(&seg, &buffers, Some(&cur), false);
        assert_eq!(result.len(), 2);
        // Current, not modified → "buf"
        assert_eq!(
            result[0].1.get("highlight_group_prefix"),
            Some(&json!("buf"))
        );
        // Non-current, modified → "buf_nc_mod"
        assert_eq!(
            result[1].1.get("highlight_group_prefix"),
            Some(&json!("buf_nc_mod"))
        );
    }

    #[test]
    fn bufferlister_for_current_always_shown_even_if_unlisted() {
        let seg = Map::new();
        let cur = buf(1, false, false); // current but unlisted
        let buffers = vec![cur.clone()];
        let result = bufferlister_for(&seg, &buffers, Some(&cur), false);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn add_multiplier_sets_priority_multiplier_from_distance() {
        // py:46  1 + 0.001 * abs(tabnr - cur)
        let mut dct = Map::new();
        let returned = add_multiplier(5, 3, &mut dct);
        // |5 - 3| * 0.001 = 0.002 → 1.002
        let v = returned.get("priority_multiplier").unwrap();
        assert!((v.as_f64().unwrap() - 1.002).abs() < 1e-9);
    }

    #[test]
    fn add_multiplier_returns_one_when_distance_is_zero() {
        let mut dct = Map::new();
        add_multiplier(7, 7, &mut dct);
        assert_eq!(
            dct.get("priority_multiplier").unwrap().as_f64().unwrap(),
            1.0
        );
    }

    #[test]
    fn add_multiplier_handles_negative_distance_via_abs() {
        let mut dct = Map::new();
        add_multiplier(1, 10, &mut dct);
        // |1 - 10| = 9, so 1 + 0.009 = 1.009
        let v = dct.get("priority_multiplier").unwrap().as_f64().unwrap();
        assert!((v - 1.009).abs() < 1e-9);
    }
}
