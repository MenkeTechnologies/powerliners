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
    // py:14  segment_info = segment_info.copy()
    let mut info = segment_info.clone();
    // py:15-23  segment_info.update(tabpage=..., tabnr=..., window=..., winnr=..., ...)
    info.insert(
        "tabnr".to_string(),
        Value::from(tabpage.number),
    );
    info.insert(
        "winnr".to_string(),
        Value::from(tabpage.window.number),
    );
    info.insert(
        "window_id".to_string(),
        Value::from(tabpage.window.window_id),
    );
    info.insert(
        "bufnr".to_string(),
        Value::from(tabpage.window.buffer.number),
    );
    // (`tabpage`, `window`, `buffer` raw object refs are Python-only —
    //  not modelled in the JSON Value carrier)
    info                                              // py:24  return segment_info
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

/// Test-driveable variant of `tablister` — drives the prefix-and-
/// multiplier logic against a caller-supplied tabpage list.
pub fn tablister_for(
    segment_info: &Map<String, Value>,
    tabpages: &[VimTabpage],
    current: Option<&VimTabpage>,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:39  cur_tabnr = cur_tabpage.number
    let cur_tabnr = current.map(|c| c.number).unwrap_or(1);

    tabpages
        .iter()
        .map(|tabpage| {
            // py:46-49  prefix selection
            let prefix = if tabpage.number == cur_tabnr {
                "tab"
            } else {
                "tab_nc"
            };
            // py:41-43  priority_multiplier = 1 + 0.001 * abs(tabnr - cur_tabnr)
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
            second.insert(
                "priority_multiplier".to_string(),
                json!(multiplier),
            );
            (
                tabpage_updated_segment_info(segment_info, tabpage),
                second,
            )
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
    // py:65  segment_info = segment_info.copy()
    let mut info = segment_info.clone();
    // py:66-72  segment_info.update(window=None, winnr=None, window_id=None,
    //                              buffer=buffer, bufnr=buffer.number)
    info.insert("window".to_string(), Value::Null);
    info.insert("winnr".to_string(), Value::Null);
    info.insert("window_id".to_string(), Value::Null);
    info.insert("bufnr".to_string(), Value::from(buffer.number));
    info                                              // py:73
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
    let cur_bufnr = current.map(|b| b.number).unwrap_or(1);
    let cur_bufnr_matched = current.map(|b| b.number);

    buffers
        .iter()
        .filter(|buffer| {
            // py:107-118  buffer is cur_buffer or show_unlisted or buflisted
            let is_current = Some(buffer.number) == cur_bufnr_matched;
            is_current || show_unlisted || buffer.listed
        })
        .map(|buffer| {
            // py:104  current = 'buf' if buffer is cur_buffer else 'buf_nc'
            let current_pfx = if Some(buffer.number) == cur_bufnr_matched {
                "buf"
            } else {
                "buf_nc"
            };
            // py:105  modified = '_mod' if &modified else ''
            let modified_pfx = if buffer.modified { "_mod" } else { "" };
            let prefix = format!("{}{}", current_pfx, modified_pfx);
            // py:96-98  priority_multiplier = 1 + 0.001 * abs(bufnr - cur_bufnr)
            let multiplier = 1.0_f64 + 0.001 * ((buffer.number - cur_bufnr).abs() as f64);
            let mut second = Map::new();
            second.insert(
                "highlight_group_prefix".to_string(),
                Value::String(prefix),
            );
            second.insert(
                "divider_highlight_group".to_string(),
                Value::String("tab:divider".to_string()),
            );
            second.insert(
                "priority_multiplier".to_string(),
                json!(multiplier),
            );
            (
                buffer_updated_segment_info(segment_info, buffer),
                second,
            )
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
        assert_eq!(result[0].1.get("highlight_group_prefix"), Some(&json!("tab_nc")));
        assert_eq!(result[1].1.get("highlight_group_prefix"), Some(&json!("tab")));
        assert_eq!(result[2].1.get("highlight_group_prefix"), Some(&json!("tab_nc")));
    }

    #[test]
    fn tablister_for_priority_multiplier_grows_with_distance() {
        let seg = Map::new();
        let cur = tab(5);
        let tabs = vec![tab(1), tab(5), tab(10)];
        let result = tablister_for(&seg, &tabs, Some(&cur));
        // Distance 4 → 1.004; distance 0 → 1.0; distance 5 → 1.005
        let m0 = result[0].1.get("priority_multiplier").unwrap().as_f64().unwrap();
        let m1 = result[1].1.get("priority_multiplier").unwrap().as_f64().unwrap();
        let m2 = result[2].1.get("priority_multiplier").unwrap().as_f64().unwrap();
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
            buf(2, true, true),   // listed + modified, non-current
        ];
        let result = bufferlister_for(&seg, &buffers, Some(&cur), false);
        assert_eq!(result.len(), 2);
        // Current, not modified → "buf"
        assert_eq!(result[0].1.get("highlight_group_prefix"), Some(&json!("buf")));
        // Non-current, modified → "buf_nc_mod"
        assert_eq!(result[1].1.get("highlight_group_prefix"), Some(&json!("buf_nc_mod")));
    }

    #[test]
    fn bufferlister_for_current_always_shown_even_if_unlisted() {
        let seg = Map::new();
        let cur = buf(1, false, false); // current but unlisted
        let buffers = vec![cur.clone()];
        let result = bufferlister_for(&seg, &buffers, Some(&cur), false);
        assert_eq!(result.len(), 1);
    }
}
