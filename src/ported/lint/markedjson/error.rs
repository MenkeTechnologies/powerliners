// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/error.py`.
//!
//! Error-reporting primitives for the lint-time JSON loader. Defines
//! `NON_PRINTABLE_RE`, the `strtrans` non-printable substitutor, the
//! rich `Mark` carrying buffer/pointer for `get_snippet`, the
//! `format_error` multi-line error formatter, and the `MarkedError` /
//! `EchoErr` value types.
//!
//! Note: the leaner `Mark { line, column }` used by token/scanner code
//! lives in `nodes.rs`; this `RichMark` is the lint-error reporting
//! variant that knows about the source buffer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// import re                                         // py:5
// from powerline.lib.encoding import get_preferred_output_encoding                        // py:7

use regex::Regex;
use std::sync::OnceLock;

/// Port of `NON_PRINTABLE_RE` from
/// `powerline/lint/markedjson/error.py:10`.
///
/// Matches characters outside the JSON-allowed printable range.
/// The Python source builds this from `NON_PRINTABLE_STR` (py:10-32)
/// excluding `\t`, `\n`, `\x20-\x7E`, `U+0085`, the BMP printable
/// blocks, and the SMP range. Rust analog: a conservative ASCII-control
/// matcher covering the same forbidden range for control codes.
#[allow(non_snake_case)]
pub fn NON_PRINTABLE_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // py:10-33 NON_PRINTABLE_STR build.
        Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap()
    })
}

/// Port of `repl()` from `powerline/lint/markedjson/error.py:36`.
///
/// Python: `return '<x%04x>' % ord(s.group())`.
/// Given the matched character, returns the `<xNNNN>` escape.
pub fn repl(matched: &str) -> String {
    // py:37  ord(s.group())
    let cp = matched.chars().next().map(|c| c as u32).unwrap_or(0);
    format!("<x{:04x}>", cp)
}

/// Port of `strtrans()` from `powerline/lint/markedjson/error.py:40`.
///
/// Python: `NON_PRINTABLE_RE.sub(repl, s.replace('\t', '>---'))`.
/// Replaces tabs with `>---` then escapes non-printable characters.
pub fn strtrans(s: &str) -> String {
    // py:41  s.replace('\t', '>---')
    let tabs = s.replace('\t', ">---");
    // py:41  NON_PRINTABLE_RE.sub(repl, ...)
    NON_PRINTABLE_RE()
        .replace_all(&tabs, |caps: &regex::Captures<'_>| repl(&caps[0]))
        .into_owned()
}

/// Port of `class Mark` from `powerline/lint/markedjson/error.py:44`.
///
/// Lint-error mark: carries the source buffer so `get_snippet` can
/// extract the offending line. Distinct from the leaner
/// `nodes::Mark { line, column }` used by tokens.
#[derive(Debug, Clone)]
pub struct RichMark {
    /// Python: `self.name` — source name (e.g. file path).
    pub name: String,
    /// Python: `self.line` — 0-based line index.
    pub line: usize,
    /// Python: `self.column` — 0-based column index.
    pub column: usize,
    /// Python: `self.buffer` — full source buffer (Some(chars)) or None.
    pub buffer: Option<Vec<char>>,
    /// Python: `self.pointer` — absolute index into buffer.
    pub pointer: usize,
    /// Python: `self.old_mark` — chain pointer for value-replacement
    /// history (boxed because of recursion).
    pub old_mark: Option<Box<RichMark>>,
    /// Python: `self.merged_marks` — additional marks merged into this
    /// one.
    pub merged_marks: Vec<RichMark>,
}

impl RichMark {
    /// Port of `Mark.__init__()` from
    /// `powerline/lint/markedjson/error.py:45`.
    pub fn new(
        name: impl Into<String>,
        line: usize,
        column: usize,
        buffer: Option<Vec<char>>,
        pointer: usize,
    ) -> Self {
        Self {
            name: name.into(),
            line,
            column,
            buffer,
            pointer,
            old_mark: None,
            merged_marks: Vec::new(),
        }
    }

    /// Port of `Mark.copy()` from
    /// `powerline/lint/markedjson/error.py:53`.
    pub fn copy(&self) -> RichMark {
        // py:54  return Mark(self.name, self.line, ..., self.merged_marks[:])
        self.clone()
    }

    /// Port of `Mark.get_snippet()` from
    /// `powerline/lint/markedjson/error.py:56`.
    ///
    /// Extracts an `indent`-prefixed source-context snippet
    /// surrounding `pointer`, with a `^` caret marker on the
    /// following line.
    pub fn get_snippet(&self, indent: usize, max_length: usize) -> Option<String> {
        // py:57-58  if self.buffer is None: return None
        let buf = self.buffer.as_ref()?;
        // py:59-65  walk backwards from pointer to start of line
        let mut head = String::new();
        let mut start = self.pointer;
        while start > 0 && !matches!(buf.get(start - 1), Some('\0') | Some('\n')) {
            start -= 1;
            if self.pointer.saturating_sub(start) > max_length / 2 - 1 {
                head = " ... ".to_string();
                start += 5;
                break;
            }
        }
        // py:66-72  walk forward from pointer to end of line
        let mut tail = String::new();
        let mut end = self.pointer;
        while end < buf.len() && !matches!(buf.get(end), Some('\0') | Some('\n')) {
            end += 1;
            if end - self.pointer > max_length / 2 - 1 {
                tail = " ... ".to_string();
                end -= 5;
                break;
            }
        }
        // py:73-74  snippet = [pre, ch, post]; strtrans each piece
        let pre: String = buf[start..self.pointer].iter().collect();
        let ch: String = buf.get(self.pointer).copied().unwrap_or('\0').to_string();
        let post: String = buf
            .get(self.pointer + 1..end.min(buf.len()))
            .map(|s| s.iter().collect())
            .unwrap_or_default();
        let snippet = [strtrans(&pre), strtrans(&ch), strtrans(&post)];
        // py:76-79  format the line + caret line
        let indent_str = " ".repeat(indent);
        let caret_pad = " ".repeat(indent + head.len() + snippet[0].len());
        Some(format!(
            "{}{}{}{}{}{}\n{}^",
            indent_str, head, snippet[0], snippet[1], snippet[2], tail, caret_pad
        ))
    }

    /// Port of `Mark.advance_string()` from
    /// `powerline/lint/markedjson/error.py:81`.
    pub fn advance_string(&self, diff: usize) -> RichMark {
        // py:82-86  ret = self.copy(); ret.column += diff; ret.pointer += diff
        let mut ret = self.copy();
        ret.column += diff;
        ret.pointer += diff;
        ret
    }

    /// Port of `Mark.set_old_mark()` from
    /// `powerline/lint/markedjson/error.py:88`.
    ///
    /// Sets the old-mark chain. Detects recursive cycles and refuses.
    /// Returns `Err(())` on cycle (Python raises `ValueError`).
    pub fn set_old_mark(&mut self, old_mark: RichMark) -> Result<(), &'static str> {
        // py:89-90  if self is old_mark: return
        if std::ptr::eq(self as *const _, &old_mark as *const _) {
            return Ok(());
        }
        // py:91-99  walk old_mark.old_mark chain for cycles. Without
        // Python's id() identity we approximate by comparing the
        // (name, line, column) triple.
        let mut seen: Vec<(String, usize, usize)> =
            vec![(self.name.clone(), self.line, self.column)];
        let mut cursor: Option<&RichMark> = Some(&old_mark);
        while let Some(m) = cursor {
            let id = (m.name.clone(), m.line, m.column);
            if seen.contains(&id) {
                return Err("Trying to set recursive marks");
            }
            seen.push(id);
            cursor = m.old_mark.as_deref();
        }
        // py:100  self.old_mark = old_mark
        self.old_mark = Some(Box::new(old_mark));
        Ok(())
    }

    /// Port of `Mark.set_merged_mark()` from
    /// `powerline/lint/markedjson/error.py:102`.
    pub fn set_merged_mark(&mut self, merged_mark: RichMark) {
        // py:103  self.merged_marks.append(merged_mark)
        self.merged_marks.push(merged_mark);
    }

    /// Port of `Mark.to_string()` from
    /// `powerline/lint/markedjson/error.py:105`.
    pub fn to_string_marked(&self, indent: usize, head_text: &str, add_snippet: bool) -> String {
        // py:106-132  multi-line "in <name>, line N, column M:" + snippet
        // + recursive merged_marks / old_mark traversal.
        let mut where_str = String::new();
        let mut cursor: Option<&RichMark> = Some(self);
        let mut cur_indent = indent;
        // py:113  processed_marks = set()
        let mut processed: Vec<(String, usize, usize)> = Vec::new();
        while let Some(mark) = cursor {
            let indent_str = " ".repeat(cur_indent);
            // py:115-116  '%s  %s"%s", line %d, column %d'
            where_str.push_str(&format!(
                "{}  {}\"{}\", line {}, column {}",
                indent_str,
                head_text,
                mark.name,
                mark.line + 1,
                mark.column + 1,
            ));
            // py:117-119  add snippet
            if add_snippet {
                if let Some(snippet) = mark.get_snippet(cur_indent + 4, 75) {
                    where_str.push_str(":\n");
                    where_str.push_str(&snippet);
                }
            }
            // py:120-125  merged_marks recursion
            if !mark.merged_marks.is_empty() {
                where_str.push('\n');
                where_str.push_str(&indent_str);
                where_str.push_str("  with additionally merged\n");
                where_str.push_str(&mark.merged_marks[0].to_string_marked(
                    cur_indent + 4,
                    "",
                    false,
                ));
                for mm in &mark.merged_marks[1..] {
                    where_str.push('\n');
                    where_str.push_str(&indent_str);
                    where_str.push_str("  and\n");
                    where_str.push_str(&mm.to_string_marked(cur_indent + 4, "", false));
                }
            }
            // py:126-129  old_mark walks
            if add_snippet {
                let id = (mark.name.clone(), mark.line, mark.column);
                processed.push(id);
                if mark.old_mark.is_some() {
                    where_str.push('\n');
                    where_str.push_str(&indent_str);
                    where_str.push_str("  which replaced value\n");
                    cur_indent += 4;
                }
            }
            // py:130-131  recursion-cycle check
            cursor = mark.old_mark.as_deref();
            if let Some(m) = cursor {
                let id = (m.name.clone(), m.line, m.column);
                if processed.contains(&id) {
                    // py:131  raise ValueError — surface as marker text
                    where_str.push_str("\n<recursive mark>");
                    break;
                }
            }
        }
        where_str
    }
}

impl std::fmt::Display for RichMark {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // py:139-142  __str__ = to_string()
        write!(f, "{}", self.to_string_marked(0, "in ", true))
    }
}

impl PartialEq for RichMark {
    fn eq(&self, other: &Self) -> bool {
        // py:144-149  self is other or (name == name and line == line and column == column)
        std::ptr::eq(self, other)
            || (self.name == other.name && self.line == other.line && self.column == other.column)
    }
}

/// Port of `format_error()` from
/// `powerline/lint/markedjson/error.py:166`.
///
/// Multi-line error formatter combining context/problem messages with
/// their respective marks and an optional trailing note.
pub fn format_error(
    context: Option<&str>,
    context_mark: Option<&RichMark>,
    problem: Option<&str>,
    problem_mark: Option<&RichMark>,
    note: Option<&str>,
    indent: usize,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    let indent_str = " ".repeat(indent);
    // py:168-170  if context is not None: lines.append(...)
    if let Some(c) = context {
        lines.push(format!("{}{}", indent_str, c));
    }
    // py:171-178  context_mark (only when different from problem_mark)
    if let Some(cm) = context_mark {
        let same_as_problem = matches!((problem, problem_mark), (Some(_), Some(pm)) if cm == pm);
        if !same_as_problem {
            lines.push(cm.to_string_marked(indent, "in ", true));
        }
    }
    // py:179-181  if problem is not None: lines.append(...)
    if let Some(p) = problem {
        lines.push(format!("{}{}", indent_str, p));
    }
    // py:182-183  problem_mark.to_string
    if let Some(pm) = problem_mark {
        lines.push(pm.to_string_marked(indent, "in ", true));
    }
    // py:184-185  note
    if let Some(n) = note {
        lines.push(format!("{}{}", indent_str, n));
    }
    lines.join("\n")
}

/// Port of `class MarkedError(Exception)` from
/// `powerline/lint/markedjson/error.py:188`.
#[derive(Debug, Clone)]
pub struct MarkedError {
    pub message: String,
}

impl MarkedError {
    /// Port of `MarkedError.__init__()` from
    /// `powerline/lint/markedjson/error.py:189`.
    pub fn new(
        context: Option<&str>,
        context_mark: Option<&RichMark>,
        problem: Option<&str>,
        problem_mark: Option<&RichMark>,
        note: Option<&str>,
    ) -> Self {
        // py:190  Exception.__init__(self, format_error(...))
        Self {
            message: format_error(context, context_mark, problem, problem_mark, note, 0),
        }
    }
}

impl std::fmt::Display for MarkedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MarkedError {}

/// Port of `class EchoErr` from
/// `powerline/lint/markedjson/error.py:193`.
///
/// Wraps an `echoerr` callback + logger with a fixed indent. Rust
/// stub: holds the indent only; the actual `echoerr` callback is left
/// as a generic trait once a callback dispatch is needed.
pub struct EchoErr {
    pub indent: usize,
}

impl EchoErr {
    /// Port of `EchoErr.__init__()` from
    /// `powerline/lint/markedjson/error.py:196`.
    pub fn new(indent: usize) -> Self {
        Self { indent }
    }
}

// `DelayedEchoErr` (py:207-241) ports separately — its
// `next_variant`/`echo_all` flow encodes a multi-variant error
// accumulator that touches the (currently unported) logger and
// `__call__` plumbing. Deferred.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_printable_re_matches_ctrl_chars() {
        assert!(NON_PRINTABLE_RE().is_match("\x07"));
        assert!(NON_PRINTABLE_RE().is_match("\x1f"));
        assert!(!NON_PRINTABLE_RE().is_match("abc"));
    }

    #[test]
    fn non_printable_re_allows_tab_newline_cr() {
        // py:NON_PRINTABLE_STR includes \t \n in the allowed set
        assert!(!NON_PRINTABLE_RE().is_match("\t"));
        assert!(!NON_PRINTABLE_RE().is_match("\n"));
        // CR is at 0x0D which is between 0x0B/0x0C and 0x0E-0x1F, so
        // CR (0x0D) is NOT in the forbidden range either.
        assert!(!NON_PRINTABLE_RE().is_match("\r"));
    }

    #[test]
    fn repl_formats_codepoint_as_hex() {
        // py:37  '<x%04x>' % ord(s.group())
        assert_eq!(repl("\x07"), "<x0007>");
        assert_eq!(repl("\x1f"), "<x001f>");
        assert_eq!(repl("A"), "<x0041>");
    }

    #[test]
    fn strtrans_replaces_tab_with_dashes() {
        // py:41  s.replace('\t', '>---')
        assert_eq!(strtrans("a\tb"), "a>---b");
    }

    #[test]
    fn strtrans_escapes_non_printable() {
        // py:41  NON_PRINTABLE_RE.sub(repl, ...)
        assert_eq!(strtrans("a\x07b"), "a<x0007>b");
    }

    #[test]
    fn strtrans_passes_printable_through() {
        assert_eq!(strtrans("hello world"), "hello world");
    }

    #[test]
    fn rich_mark_new_has_no_old_or_merged() {
        let m = RichMark::new("f.json", 0, 0, None, 0);
        assert!(m.old_mark.is_none());
        assert!(m.merged_marks.is_empty());
    }

    #[test]
    fn rich_mark_copy_is_equal() {
        let m = RichMark::new("f.json", 3, 4, None, 0);
        let c = m.copy();
        assert_eq!(m.name, c.name);
        assert_eq!(m.line, c.line);
        assert_eq!(m.column, c.column);
    }

    #[test]
    fn rich_mark_get_snippet_none_when_no_buffer() {
        let m = RichMark::new("f.json", 0, 0, None, 0);
        assert!(m.get_snippet(4, 75).is_none());
    }

    #[test]
    fn rich_mark_get_snippet_extracts_line() {
        let buf: Vec<char> = "hello world\n".chars().collect();
        // pointer at 'w' (index 6), max_length large enough
        let m = RichMark::new("f.json", 0, 6, Some(buf), 6);
        let snippet = m.get_snippet(4, 75).unwrap();
        // contains "hello world" and a caret on the next line
        assert!(snippet.contains("hello world"));
        assert!(snippet.contains('^'));
        // caret indent = 4 (indent) + 0 (head) + 6 (pre length "hello ")
        let caret_line = snippet.lines().nth(1).unwrap();
        assert_eq!(caret_line, "          ^"); // 10 spaces then ^
    }

    #[test]
    fn rich_mark_advance_string_offsets_column_and_pointer() {
        let m = RichMark::new("f.json", 1, 5, None, 10);
        let advanced = m.advance_string(3);
        assert_eq!(advanced.column, 8);
        assert_eq!(advanced.pointer, 13);
        assert_eq!(advanced.line, 1);
    }

    #[test]
    fn rich_mark_set_merged_mark_appends() {
        let mut m = RichMark::new("a", 0, 0, None, 0);
        m.set_merged_mark(RichMark::new("b", 1, 0, None, 0));
        m.set_merged_mark(RichMark::new("c", 2, 0, None, 0));
        assert_eq!(m.merged_marks.len(), 2);
        assert_eq!(m.merged_marks[0].name, "b");
        assert_eq!(m.merged_marks[1].name, "c");
    }

    #[test]
    fn rich_mark_set_old_mark_chains() {
        let mut m = RichMark::new("a", 0, 0, None, 0);
        let old = RichMark::new("b", 1, 0, None, 0);
        m.set_old_mark(old).unwrap();
        assert!(m.old_mark.is_some());
        assert_eq!(m.old_mark.as_ref().unwrap().name, "b");
    }

    #[test]
    fn rich_mark_eq_compares_name_line_column() {
        let a = RichMark::new("f", 1, 2, None, 10);
        let b = RichMark::new("f", 1, 2, None, 99); // diff pointer
        assert_eq!(a, b);
        let c = RichMark::new("f", 1, 3, None, 10); // diff col
        assert_ne!(a, c);
    }

    #[test]
    fn rich_mark_to_string_marked_emits_line_column() {
        let m = RichMark::new("f.json", 2, 4, None, 0);
        let s = m.to_string_marked(0, "in ", false);
        // py:115-116  '%s  %s"%s", line %d, column %d'
        //   line+1 = 3, column+1 = 5
        assert!(s.contains("\"f.json\", line 3, column 5"));
        assert!(s.contains("in "));
    }

    #[test]
    fn format_error_combines_context_and_problem() {
        let cm = RichMark::new("ctx.json", 0, 0, None, 0);
        let pm = RichMark::new("prob.json", 5, 0, None, 0);
        let s = format_error(
            Some("found error"),
            Some(&cm),
            Some("bad token"),
            Some(&pm),
            Some("hint: check syntax"),
            0,
        );
        assert!(s.contains("found error"));
        assert!(s.contains("bad token"));
        assert!(s.contains("ctx.json"));
        assert!(s.contains("prob.json"));
        assert!(s.contains("hint: check syntax"));
    }

    #[test]
    fn format_error_omits_context_mark_when_same_as_problem_mark() {
        // py:171-178  skip context_mark if it equals problem_mark
        let m = RichMark::new("f.json", 0, 0, None, 0);
        let s = format_error(Some("ctx"), Some(&m), Some("prob"), Some(&m), None, 0);
        // The "f.json" mark line should appear once (from problem_mark),
        // not twice.
        let count = s.matches("\"f.json\"").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn format_error_indent_prefixes_text() {
        let s = format_error(Some("ctx"), None, None, None, None, 4);
        assert!(s.starts_with("    ctx"));
    }

    #[test]
    fn marked_error_format_includes_problem() {
        let pm = RichMark::new("f.json", 1, 1, None, 0);
        let e = MarkedError::new(
            Some("syntax error"),
            None,
            Some("unexpected token"),
            Some(&pm),
            None,
        );
        let s = e.to_string();
        assert!(s.contains("syntax error"));
        assert!(s.contains("unexpected token"));
        assert!(s.contains("f.json"));
    }

    #[test]
    fn marked_error_implements_error_traits() {
        let e = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn echoerr_holds_indent() {
        let e = EchoErr::new(4);
        assert_eq!(e.indent, 4);
    }
}
