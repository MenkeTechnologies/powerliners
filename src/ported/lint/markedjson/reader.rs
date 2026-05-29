// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/reader.py`.
//!
//! Reader: input stream abstraction for the markedjson parser.
//! - determines the data encoding and converts it to a unicode string
//! - checks if characters are in allowed range
//! - adds '\0' to the end
//!
//! Rust port: operates over a `String` (already UTF-8 by construction),
//! tracks line/column for `get_mark()`, exposes `peek` / `prefix` /
//! `forward` cursor operations + `check_printable` for non-printable
//! character detection.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import codecs                                    // py:4
// from powerline.lint.markedjson.error import MarkedError, Mark, NON_PRINTABLE_RE         // py:6
// from powerline.lib.unicode import unicode        // py:7

use crate::ported::lint::markedjson::nodes::Mark;
use regex::Regex;
use std::sync::OnceLock;

/// Compiled regex matching non-printable characters per
/// `powerline/lint/markedjson/error.py:NON_PRINTABLE_RE` —
/// control codes outside the JSON-allowed range.
#[allow(non_snake_case)]
fn NON_PRINTABLE_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // py:reader uses the upstream NON_PRINTABLE_RE which excludes
        // \t / \n / \r and printable ASCII + printable unicode.
        // Conservative approximation: control chars except those.
        Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap()
    })
}

/// Port of `class ReaderError(MarkedError)` from
/// `powerline/lint/markedjson/reader.py:14`.
#[derive(Debug, Clone)]
pub struct ReaderError {
    pub context: String,
    pub problem: String,
    pub mark: Option<Mark>,
}

impl std::fmt::Display for ReaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReaderError: {} ({})", self.problem, self.context)
    }
}

impl std::error::Error for ReaderError {}

/// Port of `class Reader` from `powerline/lint/markedjson/reader.py:17`.
///
/// Cursor-based reader over a UTF-8 string. Tracks `(index, line,
/// column)` for `get_mark()` and exposes peek/prefix/forward
/// operations the scanner needs.
pub struct Reader {
    /// Python: `self.name` — input source name (e.g. file path).
    pub name: String,
    /// Python: `self.buffer` — the UTF-8 string we're reading (with
    /// terminating '\0' appended per py:124).
    pub buffer: Vec<char>,
    /// Python: `self.pointer` — current cursor position in `buffer`.
    pub pointer: usize,
    /// Python: `self.index` — absolute char index from stream start.
    pub index: usize,
    /// Python: `self.line` — current 0-based line number.
    pub line: usize,
    /// Python: `self.column` — current 0-based column number.
    pub column: usize,
}

impl Reader {
    /// Port of `Reader.__init__()` from
    /// `powerline/lint/markedjson/reader.py:28`.
    pub fn new(stream: impl Into<String>, name: impl Into<String>) -> Self {
        let s: String = stream.into();
        // py:124  self.buffer += '\0'  — terminator
        let mut buffer: Vec<char> = s.chars().collect();
        buffer.push('\0');
        Self {
            name: name.into(),
            buffer,
            pointer: 0,
            index: 0,
            line: 0,
            column: 0,
        }
    }

    /// Port of `Reader.peek()` from
    /// `powerline/lint/markedjson/reader.py:51`.
    ///
    /// Returns the char at `pointer + index`, or '\0' past EOF.
    pub fn peek(&self, index: usize) -> char {
        self.buffer
            .get(self.pointer + index)
            .copied()
            .unwrap_or('\0')
    }

    /// Port of `Reader.prefix()` from
    /// `powerline/lint/markedjson/reader.py:58`.
    ///
    /// Returns the substring [pointer, pointer+length) of the buffer.
    pub fn prefix(&self, length: usize) -> String {
        let end = std::cmp::min(self.pointer + length, self.buffer.len());
        self.buffer[self.pointer..end].iter().collect()
    }

    /// Port of `Reader.update_pointer()` from
    /// `powerline/lint/markedjson/reader.py:64`.
    ///
    /// Advances pointer by `length` chars, updating line/column.
    pub fn update_pointer(&mut self, length: usize) {
        let mut left = length;
        while left > 0 && self.pointer < self.buffer.len() {
            let ch = self.buffer[self.pointer];
            self.pointer += 1;
            self.index += 1;
            if ch == '\n' {
                // py:71-72  line += 1; column = 0
                self.line += 1;
                self.column = 0;
            } else {
                // py:73-74  column += 1
                self.column += 1;
            }
            left -= 1;
        }
    }

    /// Port of `Reader.forward()` from
    /// `powerline/lint/markedjson/reader.py:78`.
    pub fn forward(&mut self, length: usize) {
        self.update_pointer(length);
    }

    /// Port of `Reader.get_mark()` from
    /// `powerline/lint/markedjson/reader.py:83`.
    pub fn get_mark(&self) -> Mark {
        Mark {
            line: self.line,
            column: self.column,
        }
    }

    /// Port of `Reader.check_printable()` from
    /// `powerline/lint/markedjson/reader.py:86`.
    ///
    /// Raises ReaderError when `data` contains non-printable chars.
    pub fn check_printable(&mut self, data: &str) -> Result<(), ReaderError> {
        // py:87-94  NON_PRINTABLE_RE.search(data); if match: error
        if let Some(m) = NON_PRINTABLE_RE().find(data) {
            self.update_pointer(m.start());
            return Err(ReaderError {
                context: "while reading from stream".to_string(),
                problem: "found special characters which are not allowed".to_string(),
                mark: Some(self.get_mark()),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_reader_buffer_has_null_terminator() {
        let r = Reader::new("abc", "test");
        // 'a', 'b', 'c', '\0' = 4 chars
        assert_eq!(r.buffer.len(), 4);
        assert_eq!(r.buffer[3], '\0');
    }

    #[test]
    fn peek_returns_char_at_offset() {
        let r = Reader::new("abc", "t");
        assert_eq!(r.peek(0), 'a');
        assert_eq!(r.peek(1), 'b');
        assert_eq!(r.peek(2), 'c');
        assert_eq!(r.peek(3), '\0');
    }

    #[test]
    fn peek_past_eof_returns_null() {
        let r = Reader::new("a", "t");
        assert_eq!(r.peek(99), '\0');
    }

    #[test]
    fn prefix_returns_substring() {
        let r = Reader::new("hello world", "t");
        assert_eq!(r.prefix(5), "hello");
        assert_eq!(r.prefix(0), "");
    }

    #[test]
    fn forward_advances_pointer_and_column() {
        let mut r = Reader::new("abc", "t");
        r.forward(2);
        assert_eq!(r.pointer, 2);
        assert_eq!(r.column, 2);
        assert_eq!(r.line, 0);
        assert_eq!(r.index, 2);
    }

    #[test]
    fn forward_across_newline_resets_column_increments_line() {
        let mut r = Reader::new("a\nb", "t");
        r.forward(2);
        assert_eq!(r.line, 1);
        assert_eq!(r.column, 0);
    }

    #[test]
    fn get_mark_returns_current_position() {
        let mut r = Reader::new("a\nbc", "t");
        r.forward(3);
        let m = r.get_mark();
        assert_eq!(m.line, 1);
        assert_eq!(m.column, 1);
    }

    #[test]
    fn check_printable_accepts_normal_text() {
        let mut r = Reader::new("hello world", "t");
        assert!(r.check_printable("hello world").is_ok());
    }

    #[test]
    fn check_printable_rejects_control_chars() {
        let mut r = Reader::new("ok", "t");
        assert!(r.check_printable("ok\x07bad").is_err());
    }

    #[test]
    fn check_printable_accepts_tab_newline_cr() {
        let mut r = Reader::new("ok", "t");
        assert!(r.check_printable("ok\tnext\nline\rmore").is_ok());
    }

    #[test]
    fn reader_error_implements_error_traits() {
        let e = ReaderError {
            context: "ctx".to_string(),
            problem: "prob".to_string(),
            mark: None,
        };
        assert!(e.to_string().contains("ReaderError"));
        let _: &dyn std::error::Error = &e;
    }
}
