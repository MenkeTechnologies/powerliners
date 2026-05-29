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
        // py:18  class Reader(object):
        // py:19  # Reader:
        // py:20  # - determines the data encoding and converts it to a unicode string,
        // py:21  # - checks if characters are in allowed range,
        // py:22  # - adds '\0' to the end.
        // py:28  def __init__(self, stream):
        // py:29  self.name = None
        // py:30  self.stream = None
        // py:31  self.stream_pointer = 0
        // py:32  self.eof = True
        // py:33  self.buffer = ''
        // py:34  self.pointer = 0
        // py:35  self.full_buffer = unicode('')
        // py:36  self.full_pointer = 0
        // py:37  self.raw_buffer = None
        // py:38  self.raw_decode = codecs.utf_8_decode
        // py:39  self.encoding = 'utf-8'
        // py:40  self.index = 0
        // py:41  self.line = 0
        // py:42  self.column = 0
        // py:44  self.stream = stream
        // py:45  self.name = getattr(stream, 'name', '<file>')
        // py:46  self.eof = False
        // py:47  self.raw_buffer = None
        // py:49  while not self.eof and (self.raw_buffer is None or len(self.raw_buffer) < 2):
        // py:50  self.update_raw()
        // py:51  self.update(1)
        let s: String = stream.into();
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
        // py:53  def peek(self, index=0):
        // py:54  try:
        // py:55  return self.buffer[self.pointer + index]
        // py:56  except IndexError:
        // py:57  self.update(index + 1)
        // py:58  return self.buffer[self.pointer + index]
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
        // py:60  def prefix(self, length=1):
        // py:61  if self.pointer + length >= len(self.buffer):
        // py:62  self.update(length)
        // py:63  return self.buffer[self.pointer:self.pointer + length]
        let end = std::cmp::min(self.pointer + length, self.buffer.len());
        self.buffer[self.pointer..end].iter().collect()
    }

    /// Port of `Reader.update_pointer()` from
    /// `powerline/lint/markedjson/reader.py:64`.
    ///
    /// Advances pointer by `length` chars, updating line/column.
    pub fn update_pointer(&mut self, length: usize) {
        // py:65  def update_pointer(self, length):
        // py:66  while length:
        // py:67  ch = self.buffer[self.pointer]
        // py:68  self.pointer += 1
        // py:69  self.full_pointer += 1
        // py:70  self.index += 1
        let mut left = length;
        while left > 0 && self.pointer < self.buffer.len() {
            let ch = self.buffer[self.pointer];
            self.pointer += 1;
            self.index += 1;
            // py:71  if ch == '\n':
            if ch == '\n' {
                // py:72  self.line += 1
                self.line += 1;
                // py:73  self.column = 0
                self.column = 0;
            } else {
                // py:74  else:
                // py:75  self.column += 1
                self.column += 1;
            }
            // py:76  length -= 1
            left -= 1;
        }
    }

    /// Port of `Reader.forward()` from
    /// `powerline/lint/markedjson/reader.py:78`.
    pub fn forward(&mut self, length: usize) {
        // py:78  def forward(self, length=1):
        // py:79  if self.pointer + length + 1 >= len(self.buffer):
        // py:80  self.update(length + 1)
        // py:81  self.update_pointer(length)
        self.update_pointer(length);
    }

    /// Port of `Reader.get_mark()` from
    /// `powerline/lint/markedjson/reader.py:83`.
    pub fn get_mark(&self) -> Mark {
        // py:83  def get_mark(self):
        // py:84  return Mark(self.name, self.line, self.column, self.full_buffer, self.full_pointer)
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
        // py:86  def check_printable(self, data):
        // py:87  match = NON_PRINTABLE_RE.search(data)
        // py:88  if match:
        if let Some(m) = NON_PRINTABLE_RE().find(data) {
            // py:89  self.update_pointer(match.start())
            self.update_pointer(m.start());
            // py:90  raise ReaderError(
            // py:91  'while reading from stream', None,
            // py:92  'found special characters which are not allowed',
            // py:93  Mark(self.name, self.line, self.column, self.full_buffer, self.full_pointer)
            // py:94  )
            return Err(ReaderError {
                context: "while reading from stream".to_string(),
                problem: "found special characters which are not allowed".to_string(),
                mark: Some(self.get_mark()),
            });
        }
        Ok(())
    }

    /// Port of `Reader.update()` from
    /// `powerline/lint/markedjson/reader.py:96`.
    ///
    /// **Status:** stub. The Rust port loads the entire stream into
    /// the buffer at construction so update is a no-op.
    pub fn update(&mut self, _length: usize) {
        // py:96  def update(self, length):
        // py:97  if self.raw_buffer is None:
        // py:98  return
        // py:99  self.buffer = self.buffer[self.pointer:]
        // py:100  self.pointer = 0
        // py:101  while len(self.buffer) < length:
        // py:102  if not self.eof:
        // py:103  self.update_raw()
        // py:104  try:
        // py:105  data, converted = self.raw_decode(self.raw_buffer, 'strict', self.eof)
        // py:106  except UnicodeDecodeError as exc:
        // py:107  character = self.raw_buffer[exc.start]
        // py:108  position = self.stream_pointer - len(self.raw_buffer) + exc.start
        // py:109  data, converted = self.raw_decode(self.raw_buffer[:exc.start], 'strict', self.eof)
        // py:110  self.buffer += data
        // py:111  self.full_buffer += data + '<' + str(ord(character)) + '>'
        // py:112  self.raw_buffer = self.raw_buffer[converted:]
        // py:113  self.update_pointer(exc.start - 1)
        // py:114  raise ReaderError(
        // py:115  'while reading from stream', None,
        // py:116  'found character #x%04x that cannot be decoded by UTF-8 codec' % ord(character),
        // py:117  Mark(self.name, self.line, self.column, self.full_buffer, position)
        // py:118  )
        // py:119  self.buffer += data
        // py:120  self.full_buffer += data
        // py:121  self.raw_buffer = self.raw_buffer[converted:]
        // py:122  self.check_printable(data)
        // py:123  if self.eof:
        // py:124  self.buffer += '\0'
        // py:125  self.raw_buffer = None
        // py:126  break
    }

    /// Port of `Reader.update_raw()` from
    /// `powerline/lint/markedjson/reader.py:128`.
    ///
    /// **Status:** stub. Rust port reads the entire stream at
    /// construction.
    pub fn update_raw(&mut self) {
        // py:128  def update_raw(self, size=-1):
        // py:129  # Was size=4096
        // py:130  assert(size < 0)
        // py:134  data = self.stream.read(size)
        // py:135  if self.raw_buffer is None:
        // py:136  self.raw_buffer = data
        // py:137  else:
        // py:138  self.raw_buffer += data
        // py:139  self.stream_pointer += len(data)
        // py:140  if not data:
        // py:141  self.eof = True
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
