// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/pdb.py`.
//!
//! pdb segments — display current frame info (line, file, code name,
//! context, stack depth) in the pdb prompt.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// from powerline.theme import requires_segment_info                                       // py:6

use std::path::Path;

/// Per-frame info that pdb segments read.
///
/// Mirrors `segment_info['curframe']` shape: a Python frame object's
/// `f_lineno` + `f_code.co_filename` + `f_code.co_name` attributes.
#[derive(Debug, Clone, Default)]
pub struct PdbCurFrame {
    pub f_lineno: i32,
    pub co_filename: String,
    pub co_name: String,
}

/// pdb segment info shape: `curframe` + the lister-produced stack data.
#[derive(Debug, Clone, Default)]
pub struct PdbSegmentInfo {
    pub curframe: PdbCurFrame,
    pub stack_len: usize,
    pub initial_stack_length: usize,
}

/// Port of `current_line()` from `powerline/segments/pdb.py:10`.
///
/// Displays line number that is next to be run.
pub fn current_line(_pl: &(), segment_info: &PdbSegmentInfo) -> String {
    // py:13  return str(segment_info['curframe'].f_lineno)
    segment_info.curframe.f_lineno.to_string()
}

/// Port of `current_file()` from `powerline/segments/pdb.py:16`.
///
/// Displays current file name.
///
/// :param basename: If true only basename is displayed.
pub fn current_file(_pl: &(), segment_info: &PdbSegmentInfo, basename: bool) -> String {
    // py:22  filename = segment_info['curframe'].f_code.co_filename
    let filename = &segment_info.curframe.co_filename;
    // py:23-24  if basename: filename = os.path.basename(filename)
    if basename {
        Path::new(filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| filename.clone())
    } else {
        filename.clone()
    }
}

/// Port of `current_code_name()` from `powerline/segments/pdb.py:28`.
///
/// Displays name of the code object of the current frame.
pub fn current_code_name(_pl: &(), segment_info: &PdbSegmentInfo) -> String {
    // py:31  return segment_info['curframe'].f_code.co_name
    segment_info.curframe.co_name.clone()
}

/// Port of `current_context()` from `powerline/segments/pdb.py:34`.
///
/// Displays currently executed context name.
///
/// This is similar to `current_code_name`, but gives more details.
/// Currently it only gives module file name if code_name happens to be
/// `<module>`.
pub fn current_context(_pl: &(), segment_info: &PdbSegmentInfo) -> String {
    // py:41  name = segment_info['curframe'].f_code.co_name
    let name = &segment_info.curframe.co_name;
    // py:42-43  if name == '<module>': name = os.path.basename(...)
    if name == "<module>" {
        Path::new(&segment_info.curframe.co_filename)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| segment_info.curframe.co_filename.clone())
    } else {
        name.clone()
    }
}

/// Port of `stack_depth()` from `powerline/segments/pdb.py:47`.
///
/// Displays current stack depth.
///
/// :param full_stack: If true then absolute depth is used.
pub fn stack_depth(_pl: &(), segment_info: &PdbSegmentInfo, full_stack: bool) -> String {
    // py:54-55  str(len(segment_info['pdb'].stack) - (0 if full_stack else
    //               segment_info['initial_stack_length']))
    let subtract = if full_stack {
        0
    } else {
        segment_info.initial_stack_length
    };
    (segment_info.stack_len.saturating_sub(subtract)).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PdbSegmentInfo {
        PdbSegmentInfo {
            curframe: PdbCurFrame {
                f_lineno: 42,
                co_filename: "/home/user/work/main.py".into(),
                co_name: "frobnicate".into(),
            },
            stack_len: 5,
            initial_stack_length: 2,
        }
    }

    #[test]
    fn current_line_returns_lineno_as_string() {
        assert_eq!(current_line(&(), &sample()), "42");
    }

    #[test]
    fn current_file_basename_true_returns_filename_only() {
        assert_eq!(current_file(&(), &sample(), true), "main.py");
    }

    #[test]
    fn current_file_basename_false_returns_full_path() {
        assert_eq!(
            current_file(&(), &sample(), false),
            "/home/user/work/main.py"
        );
    }

    #[test]
    fn current_code_name_returns_co_name() {
        assert_eq!(current_code_name(&(), &sample()), "frobnicate");
    }

    #[test]
    fn current_context_returns_co_name_for_normal_function() {
        assert_eq!(current_context(&(), &sample()), "frobnicate");
    }

    #[test]
    fn current_context_substitutes_module_with_basename() {
        let mut info = sample();
        info.curframe.co_name = "<module>".into();
        assert_eq!(current_context(&(), &info), "main.py");
    }

    #[test]
    fn stack_depth_default_subtracts_initial() {
        assert_eq!(stack_depth(&(), &sample(), false), "3");
    }

    #[test]
    fn stack_depth_full_stack_returns_raw_len() {
        assert_eq!(stack_depth(&(), &sample(), true), "5");
    }
}
