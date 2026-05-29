// vim:fileencoding=utf-8:noet
//! Port of `powerline/listers/pdb.py`.
//!
//! Frame lister for pdb sessions — yields one subsegment per stack
//! frame so the pdb prompt can show breadcrumb path through the
//! current call chain.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import requires_segment_info                                       // py:4

use serde_json::{json, Map, Value};

/// One pdb stack frame entry (the leftmost field of each Python
/// stack tuple). Carries the frame object pointer / frame-id.
#[derive(Debug, Clone)]
pub struct PdbFrame(pub Value);

/// Segment info shape for pdb listers.
#[derive(Debug, Clone, Default)]
pub struct PdbSegmentInfo {
    /// Python: `segment_info['pdb'].stack` — list of (frame, lineno) tuples.
    /// Rust port holds the first element (the frame) since that's what
    /// `frame_lister` reads at py:30.
    pub stack: Vec<PdbFrame>,
    /// Python: `segment_info['initial_stack_length']`.
    pub initial_stack_length: usize,
}

/// Port of `frame_lister()` from `powerline/listers/pdb.py:8`.
///
/// List all frames in segment_info format.
///
/// :param full_stack: If true, then all frames in the stack are
///     listed. Normally N first frames are discarded where N is a
///     number of frames present at the first invocation of the prompt
///     minus one.
/// :param maxframes: Maximum number of frames to display.
pub fn frame_lister(
    _pl: &(),
    segment_info: &PdbSegmentInfo,
    full_stack: bool,
    maxframes: usize,
) -> Vec<(Map<String, Value>, Map<String, Value>)> {
    // py:7  @requires_segment_info
    // py:8  def frame_lister(pl, segment_info, full_stack=False, maxframes=3):
    // py:9-17  docstring
    // py:18  if full_stack:
    let (initial_stack_length, mut frames) = if full_stack {
        // py:19  initial_stack_length = 0
        // py:20  frames = segment_info['pdb'].stack
        (0, segment_info.stack.clone())
    } else {
        // py:21  else:
        // py:22  initial_stack_length = segment_info['initial_stack_length']
        let isl = segment_info.initial_stack_length;
        // py:23  frames = segment_info['pdb'].stack[initial_stack_length:]
        let sliced = segment_info.stack.iter().skip(isl).cloned().collect();
        (isl, sliced)
    };

    // py:25  if len(frames) > maxframes:
    if frames.len() > maxframes {
        // py:26  frames = frames[-maxframes:]
        let start = frames.len() - maxframes;
        frames = frames[start..].to_vec();
    }

    // py:28  return (
    // py:29  (
    // py:30  {
    frames
        .into_iter()
        .map(|frame| {
            let mut info = Map::new();
            // py:31  'curframe': frame[0],
            info.insert("curframe".to_string(), frame.0);
            // py:32  'initial_stack_length': initial_stack_length,
            info.insert(
                "initial_stack_length".to_string(),
                json!(initial_stack_length as u64),
            );
            // py:33  },
            // py:34  {}
            // py:35  )
            // py:36  for frame in frames
            // py:37  )
            (info, Map::new())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frames(n: usize) -> Vec<PdbFrame> {
        (0..n)
            .map(|i| PdbFrame(json!({"frame_id": i as u64})))
            .collect()
    }

    #[test]
    fn frame_lister_full_stack_returns_all_frames() {
        let info = PdbSegmentInfo {
            stack: frames(5),
            initial_stack_length: 0,
        };
        let result = frame_lister(&(), &info, true, 10);
        assert_eq!(result.len(), 5);
        // First frame curframe → frame_id 0
        assert_eq!(result[0].0.get("curframe").unwrap()["frame_id"], 0);
    }

    #[test]
    fn frame_lister_drops_initial_stack_length() {
        let info = PdbSegmentInfo {
            stack: frames(5),
            initial_stack_length: 2,
        };
        let result = frame_lister(&(), &info, false, 10);
        // Should skip first 2 → 3 remaining frames (frame_id 2, 3, 4)
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0.get("curframe").unwrap()["frame_id"], 2);
        assert_eq!(result[0].0.get("initial_stack_length").unwrap(), 2);
    }

    #[test]
    fn frame_lister_truncates_to_maxframes_tail() {
        let info = PdbSegmentInfo {
            stack: frames(10),
            initial_stack_length: 0,
        };
        let result = frame_lister(&(), &info, true, 3);
        // Should keep the last 3
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].0.get("curframe").unwrap()["frame_id"], 7);
        assert_eq!(result[2].0.get("curframe").unwrap()["frame_id"], 9);
    }

    #[test]
    fn frame_lister_empty_stack_returns_empty() {
        let info = PdbSegmentInfo::default();
        let result = frame_lister(&(), &info, true, 10);
        assert!(result.is_empty());
    }

    #[test]
    fn frame_lister_second_tuple_is_empty_dict() {
        let info = PdbSegmentInfo {
            stack: frames(2),
            initial_stack_length: 0,
        };
        let result = frame_lister(&(), &info, true, 10);
        // py:35-36  yield (info_dict, {}) — second element is empty map
        for (_, second) in &result {
            assert!(second.is_empty());
        }
    }
}
