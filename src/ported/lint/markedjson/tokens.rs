// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/tokens.py`.
//!
//! YAML scanner token types used by `lint/markedjson/scanner.py` to
//! tokenize the input stream. Each token carries `start_mark`/`end_mark`
//! for error reporting; specialised tokens add type-specific fields
//! (encoding for StreamStartToken, value/plain/style for ScalarToken).

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use crate::ported::lint::markedjson::nodes::Mark;

/// Port of `class Token` from `powerline/lint/markedjson/tokens.py:5`.
///
/// Base class for all scanner tokens.
#[derive(Debug, Clone)]
pub struct Token {
    pub start_mark: Option<Mark>,
    pub end_mark: Option<Mark>,
}

impl Token {
    /// Port of `Token.__init__()` from
    /// `powerline/lint/markedjson/tokens.py:6`.
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        // py:5  class Token(object):
        // py:6  def __init__(self, start_mark, end_mark):
        // py:7  self.start_mark = start_mark
        // py:8  self.end_mark = end_mark
        Self {
            start_mark,
            end_mark,
        }
    }

    /// Port of `Token.__repr__()` from
    /// `powerline/lint/markedjson/tokens.py:10`.
    ///
    /// Python builds `ClassName(key=value, ...)` excluding any
    /// `*_mark` attributes; Rust port returns the formatted shape.
    #[allow(non_snake_case)]
    pub fn __repr__(class_name: &str) -> String {
        // py:10  def __repr__(self):
        // py:11  attributes = [
        // py:12  key for key in self.__dict__
        // py:13  if not key.endswith('_mark')
        // py:14  ]
        // py:15  attributes.sort()
        // py:16  arguments = ', '.join([
        // py:17  '%s=%r' % (key, getattr(self, key))
        // py:18  for key in attributes
        // py:19  ])
        // py:20  return '%s(%s)' % (self.__class__.__name__, arguments)
        format!("{}()", class_name)
    }
}

/// Port of `class StreamStartToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:23`.
#[derive(Debug, Clone)]
pub struct StreamStartToken {
    pub token: Token,
    pub encoding: Option<String>,
}

impl StreamStartToken {
    // py:23  class StreamStartToken(Token):
    // py:24  id = '<stream start>'
    pub const ID: &'static str = "<stream start>";

    /// Port of `StreamStartToken.__init__()` from
    /// `powerline/lint/markedjson/tokens.py:26`.
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>, encoding: Option<String>) -> Self {
        // py:26  def __init__(self, start_mark=None, end_mark=None, encoding=None):
        // py:27  self.start_mark = start_mark
        // py:28  self.end_mark = end_mark
        // py:29  self.encoding = encoding
        Self {
            token: Token::new(start_mark, end_mark),
            encoding,
        }
    }
}

/// Port of `class StreamEndToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:32`.
#[derive(Debug, Clone)]
pub struct StreamEndToken {
    pub token: Token,
}

impl StreamEndToken {
    // py:32  class StreamEndToken(Token):
    // py:33  id = '<stream end>'
    pub const ID: &'static str = "<stream end>";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class FlowSequenceStartToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:36`.
#[derive(Debug, Clone)]
pub struct FlowSequenceStartToken {
    pub token: Token,
}

impl FlowSequenceStartToken {
    // py:36  class FlowSequenceStartToken(Token):
    // py:37  id = '['
    pub const ID: &'static str = "[";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class FlowMappingStartToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:40`.
#[derive(Debug, Clone)]
pub struct FlowMappingStartToken {
    pub token: Token,
}

impl FlowMappingStartToken {
    // py:40  class FlowMappingStartToken(Token):
    // py:41  id = '{'
    pub const ID: &'static str = "{";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class FlowSequenceEndToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:44`.
#[derive(Debug, Clone)]
pub struct FlowSequenceEndToken {
    pub token: Token,
}

impl FlowSequenceEndToken {
    // py:44  class FlowSequenceEndToken(Token):
    // py:45  id = ']'
    pub const ID: &'static str = "]";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class FlowMappingEndToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:48`.
#[derive(Debug, Clone)]
pub struct FlowMappingEndToken {
    pub token: Token,
}

impl FlowMappingEndToken {
    // py:48  class FlowMappingEndToken(Token):
    // py:49  id = '}'
    pub const ID: &'static str = "}";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class KeyToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:52`.
#[derive(Debug, Clone)]
pub struct KeyToken {
    pub token: Token,
}

impl KeyToken {
    // py:52  class KeyToken(Token):
    // py:53  id = '?'
    pub const ID: &'static str = "?";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class ValueToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:56`.
#[derive(Debug, Clone)]
pub struct ValueToken {
    pub token: Token,
}

impl ValueToken {
    // py:56  class ValueToken(Token):
    // py:57  id = ':'
    pub const ID: &'static str = ":";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class FlowEntryToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:60`.
#[derive(Debug, Clone)]
pub struct FlowEntryToken {
    pub token: Token,
}

impl FlowEntryToken {
    // py:60  class FlowEntryToken(Token):
    // py:61  id = ','
    pub const ID: &'static str = ",";

    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            token: Token::new(start_mark, end_mark),
        }
    }
}

/// Port of `class ScalarToken(Token)` from
/// `powerline/lint/markedjson/tokens.py:64`.
///
/// The only token type carrying a literal value. `plain` is true for
/// unquoted scalars; `style` carries the quote/literal char for
/// quoted scalars (`'`/`"`/`>`/`|`).
#[derive(Debug, Clone)]
pub struct ScalarToken {
    pub token: Token,
    pub value: String,
    pub plain: bool,
    pub style: Option<char>,
}

impl ScalarToken {
    // py:64  class ScalarToken(Token):
    // py:65  id = '<scalar>'
    pub const ID: &'static str = "<scalar>";

    /// Port of `ScalarToken.__init__()` from
    /// `powerline/lint/markedjson/tokens.py:67`.
    pub fn new(
        value: impl Into<String>,
        plain: bool,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        style: Option<char>,
    ) -> Self {
        // py:67  def __init__(self, value, plain, start_mark, end_mark, style=None):
        // py:68  self.value = value
        // py:69  self.plain = plain
        // py:70  self.start_mark = start_mark
        // py:71  self.end_mark = end_mark
        // py:72  self.style = style
        Self {
            token: Token::new(start_mark, end_mark),
            value: value.into(),
            plain,
            style,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_carries_marks() {
        let s = Mark { line: 1, column: 0 };
        let e = Mark { line: 1, column: 5 };
        let t = Token::new(Some(s.clone()), Some(e.clone()));
        assert_eq!(t.start_mark, Some(s));
        assert_eq!(t.end_mark, Some(e));
    }

    #[test]
    fn stream_start_id_matches_upstream() {
        assert_eq!(StreamStartToken::ID, "<stream start>");
    }

    #[test]
    fn stream_end_id_matches_upstream() {
        assert_eq!(StreamEndToken::ID, "<stream end>");
    }

    #[test]
    fn flow_token_ids_match_upstream() {
        assert_eq!(FlowSequenceStartToken::ID, "[");
        assert_eq!(FlowMappingStartToken::ID, "{");
        assert_eq!(FlowSequenceEndToken::ID, "]");
        assert_eq!(FlowMappingEndToken::ID, "}");
    }

    #[test]
    fn key_value_flow_entry_ids_match_upstream() {
        assert_eq!(KeyToken::ID, "?");
        assert_eq!(ValueToken::ID, ":");
        assert_eq!(FlowEntryToken::ID, ",");
    }

    #[test]
    fn scalar_token_carries_value_and_style() {
        let t = ScalarToken::new("hello", true, None, None, Some('"'));
        assert_eq!(t.value, "hello");
        assert!(t.plain);
        assert_eq!(t.style, Some('"'));
        assert_eq!(ScalarToken::ID, "<scalar>");
    }

    #[test]
    fn stream_start_token_carries_encoding() {
        let t = StreamStartToken::new(None, None, Some("utf-8".to_string()));
        assert_eq!(t.encoding.as_deref(), Some("utf-8"));
    }
}
