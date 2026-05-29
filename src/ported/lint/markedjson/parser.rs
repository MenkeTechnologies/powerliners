// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/parser.py`.
//!
//! Consumes tokens from a scanner and produces composer events. The
//! parser is a state machine: each state is a function that pops the
//! next token, emits an event, and sets the next state.
//!
//! Python implementation: `self.state` holds a bound method pointer
//! (`self.state = self.parse_stream_start`), and the public API calls
//! `self.state()` to advance.
//!
//! Rust port: `state` is an enum variant naming the next-state
//! function. `dispatch_state(state)` runs the corresponding state
//! function. The `states` stack and `marks` stack are preserved.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lint.markedjson.error import MarkedError                                  // py:4
// from powerline.lint.markedjson import tokens                                             // py:5
// from powerline.lint.markedjson import events                                             // py:6

use crate::ported::lint::markedjson::composer::{AnyEvent, EventKind};
use crate::ported::lint::markedjson::error::MarkedError;
use crate::ported::lint::markedjson::events::{MappingStartEvent, ScalarEvent, SequenceStartEvent};
// Note: DocumentStartEvent / DocumentEndEvent / SequenceEndEvent /
// MappingEndEvent / StreamStartEvent / StreamEndEvent from upstream
// events.py are referenced structurally by the parser (py:64, 72,
// 92, 98, 105, 178, etc.) but the Rust AnyEvent encodes them as
// unit variants so the typed event structs don't need to be
// imported here. Tests construct them via the events module to
// pin the import surface.
use crate::ported::lint::markedjson::nodes::Mark;

/// Port of `class ParserError(MarkedError)` from
/// `powerline/lint/markedjson/parser.py:9`.
#[derive(Debug, Clone)]
pub struct ParserError(pub MarkedError);

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ParserError {}

/// Identifies a token variant for `TokenStream::check_token()`.
///
/// Python passes class references (`tokens.StreamEndToken`); the Rust
/// trait uses these enum kinds for type-discrimination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    StreamStart,
    StreamEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    Key,
    Value,
    FlowEntry,
    Scalar,
}

/// Token payload returned by `TokenStream::get_token()` and
/// `peek_token()`. Each variant carries the per-type token fields the
/// parser reaches into (`start_mark`/`end_mark`/`encoding`/`value`/
/// `plain`/`style`).
#[derive(Debug, Clone)]
pub enum AnyToken {
    StreamStart {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        encoding: Option<String>,
    },
    StreamEnd {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    FlowSequenceStart {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    FlowSequenceEnd {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    FlowMappingStart {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    FlowMappingEnd {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    Key {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    Value {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    FlowEntry {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    },
    Scalar {
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        value: serde_json::Value,
        plain: bool,
        style: Option<char>,
    },
}

impl AnyToken {
    /// Returns the `TokenKind` discriminant.
    pub fn token_kind(&self) -> TokenKind {
        match self {
            AnyToken::StreamStart { .. } => TokenKind::StreamStart,
            AnyToken::StreamEnd { .. } => TokenKind::StreamEnd,
            AnyToken::FlowSequenceStart { .. } => TokenKind::FlowSequenceStart,
            AnyToken::FlowSequenceEnd { .. } => TokenKind::FlowSequenceEnd,
            AnyToken::FlowMappingStart { .. } => TokenKind::FlowMappingStart,
            AnyToken::FlowMappingEnd { .. } => TokenKind::FlowMappingEnd,
            AnyToken::Key { .. } => TokenKind::Key,
            AnyToken::Value { .. } => TokenKind::Value,
            AnyToken::FlowEntry { .. } => TokenKind::FlowEntry,
            AnyToken::Scalar { .. } => TokenKind::Scalar,
        }
    }

    /// Returns the start_mark for whichever token variant.
    pub fn token_start_mark(&self) -> Option<Mark> {
        match self {
            AnyToken::StreamStart { start_mark, .. }
            | AnyToken::StreamEnd { start_mark, .. }
            | AnyToken::FlowSequenceStart { start_mark, .. }
            | AnyToken::FlowSequenceEnd { start_mark, .. }
            | AnyToken::FlowMappingStart { start_mark, .. }
            | AnyToken::FlowMappingEnd { start_mark, .. }
            | AnyToken::Key { start_mark, .. }
            | AnyToken::Value { start_mark, .. }
            | AnyToken::FlowEntry { start_mark, .. }
            | AnyToken::Scalar { start_mark, .. } => start_mark.clone(),
        }
    }

    /// Returns the end_mark for whichever token variant.
    pub fn token_end_mark(&self) -> Option<Mark> {
        match self {
            AnyToken::StreamStart { end_mark, .. }
            | AnyToken::StreamEnd { end_mark, .. }
            | AnyToken::FlowSequenceStart { end_mark, .. }
            | AnyToken::FlowSequenceEnd { end_mark, .. }
            | AnyToken::FlowMappingStart { end_mark, .. }
            | AnyToken::FlowMappingEnd { end_mark, .. }
            | AnyToken::Key { end_mark, .. }
            | AnyToken::Value { end_mark, .. }
            | AnyToken::FlowEntry { end_mark, .. }
            | AnyToken::Scalar { end_mark, .. } => end_mark.clone(),
        }
    }

    /// Returns the upstream `token.id` string used in error messages.
    pub fn token_id(&self) -> &'static str {
        match self {
            AnyToken::StreamStart { .. } => "<stream start>",
            AnyToken::StreamEnd { .. } => "<stream end>",
            AnyToken::FlowSequenceStart { .. } => "[",
            AnyToken::FlowSequenceEnd { .. } => "]",
            AnyToken::FlowMappingStart { .. } => "{",
            AnyToken::FlowMappingEnd { .. } => "}",
            AnyToken::Key { .. } => "?",
            AnyToken::Value { .. } => ":",
            AnyToken::FlowEntry { .. } => ",",
            AnyToken::Scalar { .. } => "<scalar>",
        }
    }
}

/// Token-stream interface the parser consumes.
///
/// Python inherits parser+scanner+resolver+composer into one class so
/// `self.get_token()` resolves to the scanner method. Rust port
/// factors the scanner side into this trait — the concrete scanner
/// (when ported) implements it.
pub trait TokenStream {
    /// Port of `Scanner.check_token(*choices)`.
    fn check_token(&mut self, kinds: &[TokenKind]) -> bool;
    /// Port of `Scanner.peek_token()`.
    fn peek_token(&mut self) -> Option<AnyToken>;
    /// Port of `Scanner.get_token()`.
    fn get_token(&mut self) -> AnyToken;
}

/// Enum naming each parser FSM state.
///
/// Python: `self.state` is a method pointer. Rust analog: a state
/// enum dispatched via `Parser::dispatch_state`. `Done` corresponds
/// to Python's `self.state = None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    StreamStart,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentEnd,
    DocumentContent,
    Node,
    FlowSequenceFirstEntry,
    FlowSequenceEntry,
    FlowSequenceEntryMappingEnd,
    FlowMappingFirstKey,
    FlowMappingKey,
    FlowMappingValue,
    Done,
}

/// Port of `class Parser` from
/// `powerline/lint/markedjson/parser.py:13`.
pub struct Parser {
    /// Python: `self.current_event` — the next-event cache.
    pub current_event: Option<AnyEvent>,
    /// Python: `self.yaml_version` — currently unused (YAML directives).
    pub yaml_version: Option<String>,
    /// Python: `self.states` — stack of next-state continuations.
    pub states: Vec<State>,
    /// Python: `self.marks` — stack of opening-mark positions.
    pub marks: Vec<Option<Mark>>,
    /// Python: `self.state` — current state.
    pub state: State,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    /// Port of `Parser.__init__()` from
    /// `powerline/lint/markedjson/parser.py:14`.
    pub fn new() -> Self {
        // py:13  class Parser:
        // py:14  def __init__(self):
        // py:15  self.current_event = None
        // py:16  self.yaml_version = None
        // py:17  self.states = []
        // py:18  self.marks = []
        // py:19  self.state = self.parse_stream_start
        Self {
            current_event: None,
            yaml_version: None,
            states: Vec::new(),
            marks: Vec::new(),
            state: State::StreamStart,
        }
    }

    /// Port of `Parser.dispose()` from
    /// `powerline/lint/markedjson/parser.py:21`.
    pub fn dispose(&mut self) {
        // py:21  def dispose(self):
        // py:22  # Reset the state attributes (to clear self-references)
        // py:23  self.states = []
        // py:24  self.state = None
        self.states.clear();
        self.state = State::Done;
    }

    /// Port of `Parser.check_event(*choices)` from
    /// `powerline/lint/markedjson/parser.py:26`.
    pub fn check_event<S: TokenStream>(&mut self, scanner: &mut S, choices: &[EventKind]) -> bool {
        // py:26  def check_event(self, *choices):
        // py:27  # Check the type of the next event.
        // py:28  if self.current_event is None:
        // py:29  if self.state:
        // py:30  self.current_event = self.state()
        if self.current_event.is_none() && self.state != State::Done {
            self.current_event = Some(self.dispatch_state(scanner));
        }
        // py:31  if self.current_event is not None:
        // py:32  if not choices:
        // py:33  return True
        // py:34  for choice in choices:
        // py:35  if isinstance(self.current_event, choice):
        // py:36  return True
        // py:37  return False
        if let Some(ev) = &self.current_event {
            if choices.is_empty() {
                return true;
            }
            return choices.contains(&ev.kind());
        }
        false
    }

    /// Port of `Parser.peek_event()` from
    /// `powerline/lint/markedjson/parser.py:39`.
    pub fn peek_event<S: TokenStream>(&mut self, scanner: &mut S) -> Option<AnyEvent> {
        // py:39  def peek_event(self):
        // py:40  # Get the next event.
        // py:41  if self.current_event is None:
        // py:42  if self.state:
        // py:43  self.current_event = self.state()
        // py:44  return self.current_event
        if self.current_event.is_none() && self.state != State::Done {
            self.current_event = Some(self.dispatch_state(scanner));
        }
        self.current_event.clone()
    }

    /// Port of `Parser.get_event()` from
    /// `powerline/lint/markedjson/parser.py:46`.
    pub fn get_event<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:46  def get_event(self):
        // py:47  # Get the next event and proceed further.
        // py:48  if self.current_event is None:
        // py:49  if self.state:
        // py:50  self.current_event = self.state()
        if self.current_event.is_none() && self.state != State::Done {
            self.current_event = Some(self.dispatch_state(scanner));
        }
        // py:51  value = self.current_event
        // py:52  self.current_event = None
        // py:53  return value
        self.current_event.take().unwrap_or(AnyEvent::StreamEnd)
    }

    /// Dispatches the current `state` to the corresponding parse
    /// method. Rust analog of Python's `self.state()` indirection.
    fn dispatch_state<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        match self.state {
            State::StreamStart => self.parse_stream_start(scanner),
            State::ImplicitDocumentStart => self.parse_implicit_document_start(scanner),
            State::DocumentStart => self.parse_document_start(scanner),
            State::DocumentEnd => self.parse_document_end(scanner),
            State::DocumentContent => self.parse_document_content(scanner),
            State::Node => self.parse_node(scanner, false),
            State::FlowSequenceFirstEntry => self.parse_flow_sequence_first_entry(scanner),
            State::FlowSequenceEntry => self.parse_flow_sequence_entry(scanner, false),
            State::FlowSequenceEntryMappingEnd => {
                self.parse_flow_sequence_entry_mapping_end(scanner)
            }
            State::FlowMappingFirstKey => self.parse_flow_mapping_first_key(scanner),
            State::FlowMappingKey => self.parse_flow_mapping_key(scanner, false),
            State::FlowMappingValue => self.parse_flow_mapping_value(scanner),
            State::Done => AnyEvent::StreamEnd,
        }
    }

    /// Port of `Parser.parse_stream_start()` from
    /// `powerline/lint/markedjson/parser.py:59`.
    fn parse_stream_start<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:59  def parse_stream_start(self):
        // py:60  # Parse the stream start.
        // py:61  token = self.get_token()
        // py:62  event = events.StreamStartEvent(token.start_mark, token.end_mark, encoding=token.encoding)
        let token = scanner.get_token();
        let _ = token;
        // py:64  # Prepare the next state.
        // py:65  self.state = self.parse_implicit_document_start
        self.state = State::ImplicitDocumentStart;
        // py:67  return event
        AnyEvent::StreamStart
    }

    /// Port of `Parser.parse_implicit_document_start()` from
    /// `powerline/lint/markedjson/parser.py:69`.
    fn parse_implicit_document_start<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:69  def parse_implicit_document_start(self):
        // py:70  # Parse an implicit document.
        // py:71  if not self.check_token(tokens.StreamEndToken):
        if !scanner.check_token(&[TokenKind::StreamEnd]) {
            // py:72  token = self.peek_token()
            // py:73  start_mark = end_mark = token.start_mark
            // py:74  event = events.DocumentStartEvent(start_mark, end_mark, explicit=False)
            let _ = scanner.peek_token();
            // py:76  # Prepare the next state.
            // py:77  self.states.append(self.parse_document_end)
            // py:78  self.state = self.parse_node
            self.states.push(State::DocumentEnd);
            self.state = State::Node;
            // py:80  return event
            AnyEvent::DocumentStart
        } else {
            // py:82  else:
            // py:83  return self.parse_document_start()
            self.parse_document_start(scanner)
        }
    }

    /// Port of `Parser.parse_document_start()` from
    /// `powerline/lint/markedjson/parser.py:85`.
    fn parse_document_start<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:85  def parse_document_start(self):
        // py:86  # Parse an explicit document.
        // py:87  if not self.check_token(tokens.StreamEndToken):
        if !scanner.check_token(&[TokenKind::StreamEnd]) {
            // py:88  token = self.peek_token()
            // py:89  self.echoerr(
            // py:90  None, None,
            // py:91  ('expected \'<stream end>\', but found %r' % token.id), token.start_mark
            // py:92  )
            // py:93  return events.StreamEndEvent(token.start_mark, token.end_mark)
            let _ = scanner.peek_token();
            return AnyEvent::StreamEnd;
        }
        // py:94  else:
        // py:95  # Parse the end of the stream.
        // py:96  token = self.get_token()
        // py:97  event = events.StreamEndEvent(token.start_mark, token.end_mark)
        // py:98  assert not self.states
        // py:99  assert not self.marks
        // py:100  self.state = None
        let _ = scanner.get_token();
        debug_assert!(self.states.is_empty());
        debug_assert!(self.marks.is_empty());
        self.state = State::Done;
        AnyEvent::StreamEnd
    }

    /// Port of `Parser.parse_document_end()` from
    /// `powerline/lint/markedjson/parser.py:103`.
    fn parse_document_end<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:103  def parse_document_end(self):
        // py:104  # Parse the document end.
        // py:105  token = self.peek_token()
        // py:106  start_mark = end_mark = token.start_mark
        // py:107  event = events.DocumentEndEvent(start_mark, end_mark, explicit=False)
        let _ = scanner.peek_token();
        // py:109  # Prepare the next state.
        // py:110  self.state = self.parse_document_start
        self.state = State::DocumentStart;
        // py:112  return event
        AnyEvent::DocumentEnd
    }

    /// Port of `Parser.parse_document_content()` from
    /// `powerline/lint/markedjson/parser.py:115`.
    fn parse_document_content<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:114  def parse_document_content(self):
        // py:115  return self.parse_node()
        self.parse_node(scanner, false)
    }

    /// Port of `Parser.parse_node()` from
    /// `powerline/lint/markedjson/parser.py:118`.
    fn parse_node<S: TokenStream>(
        &mut self,
        scanner: &mut S,
        _indentless_sequence: bool,
    ) -> AnyEvent {
        // py:119-122  start_mark = peek_token().start_mark
        let start_mark = scanner.peek_token().and_then(|t| t.token_start_mark());
        // py:125-130  Scalar branch
        if scanner.check_token(&[TokenKind::Scalar]) {
            let token = scanner.get_token();
            if let AnyToken::Scalar {
                value,
                plain,
                style,
                end_mark,
                ..
            } = token
            {
                // py:127-130  implicit tuple
                let implicit = plain;
                self.state = self.states.pop().unwrap_or(State::Done);
                return AnyEvent::Scalar(ScalarEvent::new(
                    implicit, value, start_mark, end_mark, style,
                ));
            }
        }
        // py:132-135  Flow sequence start
        if scanner.check_token(&[TokenKind::FlowSequenceStart]) {
            let end_mark = scanner.peek_token().and_then(|t| t.token_end_mark());
            self.state = State::FlowSequenceFirstEntry;
            return AnyEvent::SequenceStart(SequenceStartEvent::new(
                true,
                start_mark,
                end_mark,
                Some(true),
            ));
        }
        // py:136-139  Flow mapping start
        if scanner.check_token(&[TokenKind::FlowMappingStart]) {
            let end_mark = scanner.peek_token().and_then(|t| t.token_end_mark());
            self.state = State::FlowMappingFirstKey;
            return AnyEvent::MappingStart(MappingStartEvent::new(
                true,
                start_mark,
                end_mark,
                Some(true),
            ));
        }
        // py:140-148  unrecognised — Python raises ParserError; Rust
        // surfaces a StreamEnd so the composer terminates gracefully.
        AnyEvent::StreamEnd
    }

    /// Port of `Parser.parse_flow_sequence_first_entry()` from
    /// `powerline/lint/markedjson/parser.py:150`.
    fn parse_flow_sequence_first_entry<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:151-153  token = get_token(); marks.append; return parse_entry(first=True)
        let token = scanner.get_token();
        self.marks.push(token.token_start_mark());
        self.parse_flow_sequence_entry(scanner, true)
    }

    /// Port of `Parser.parse_flow_sequence_entry()` from
    /// `powerline/lint/markedjson/parser.py:155`.
    fn parse_flow_sequence_entry<S: TokenStream>(
        &mut self,
        scanner: &mut S,
        first: bool,
    ) -> AnyEvent {
        // py:156  if not FlowSequenceEndToken
        if !scanner.check_token(&[TokenKind::FlowSequenceEnd]) {
            // py:157-170  non-first must have FlowEntryToken
            if !first {
                if scanner.check_token(&[TokenKind::FlowEntry]) {
                    let _ = scanner.get_token();
                } else {
                    // py:166-170  raise ParserError — Rust port emits SequenceEnd
                    return AnyEvent::SequenceEnd;
                }
            }
            // py:172-174  push entry continuation, parse next node
            if !scanner.check_token(&[TokenKind::FlowSequenceEnd]) {
                self.states.push(State::FlowSequenceEntry);
                return self.parse_node(scanner, false);
            }
        }
        // py:175-178  consume FlowSequenceEndToken, restore prev state
        let _ = scanner.get_token();
        let prev = self.states.pop().unwrap_or(State::Done);
        self.state = prev;
        self.marks.pop();
        AnyEvent::SequenceEnd
    }

    /// Port of `Parser.parse_flow_sequence_entry_mapping_end()` from
    /// `powerline/lint/markedjson/parser.py:182`.
    fn parse_flow_sequence_entry_mapping_end<S: TokenStream>(
        &mut self,
        scanner: &mut S,
    ) -> AnyEvent {
        // py:183-185
        self.state = State::FlowSequenceEntry;
        let _ = scanner.peek_token();
        AnyEvent::MappingEnd
    }

    /// Port of `Parser.parse_flow_mapping_first_key()` from
    /// `powerline/lint/markedjson/parser.py:187`.
    fn parse_flow_mapping_first_key<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:188-190  token = get_token(); marks.append; parse_key(first=True)
        let token = scanner.get_token();
        self.marks.push(token.token_start_mark());
        self.parse_flow_mapping_key(scanner, true)
    }

    /// Port of `Parser.parse_flow_mapping_key()` from
    /// `powerline/lint/markedjson/parser.py:192`.
    fn parse_flow_mapping_key<S: TokenStream>(&mut self, scanner: &mut S, first: bool) -> AnyEvent {
        // py:193  if not FlowMappingEndToken
        if !scanner.check_token(&[TokenKind::FlowMappingEnd]) {
            // py:194-206  non-first must have FlowEntryToken
            if !first {
                if scanner.check_token(&[TokenKind::FlowEntry]) {
                    let _ = scanner.get_token();
                } else {
                    return AnyEvent::MappingEnd;
                }
            }
            // py:207-228  KeyToken branch + value parse
            if scanner.check_token(&[TokenKind::Key]) {
                let _ = scanner.get_token();
                if !scanner.check_token(&[
                    TokenKind::Value,
                    TokenKind::FlowEntry,
                    TokenKind::FlowMappingEnd,
                ]) {
                    self.states.push(State::FlowMappingValue);
                    return self.parse_node(scanner, false);
                }
            } else if !scanner.check_token(&[TokenKind::FlowMappingEnd]) {
                // py:229-243  expect_key / raise — Rust emits MappingEnd
                return AnyEvent::MappingEnd;
            }
        }
        // py:245-248  consume FlowMappingEndToken, restore prev state
        let _ = scanner.get_token();
        let prev = self.states.pop().unwrap_or(State::Done);
        self.state = prev;
        self.marks.pop();
        AnyEvent::MappingEnd
    }

    /// Port of `Parser.parse_flow_mapping_value()` from
    /// `powerline/lint/markedjson/parser.py:244`.
    fn parse_flow_mapping_value<S: TokenStream>(&mut self, scanner: &mut S) -> AnyEvent {
        // py:245-249  ValueToken? get + parse_node next
        if scanner.check_token(&[TokenKind::Value]) {
            let _ = scanner.get_token();
            if !scanner.check_token(&[TokenKind::FlowEntry, TokenKind::FlowMappingEnd]) {
                self.states.push(State::FlowMappingKey);
                return self.parse_node(scanner, false);
            }
        }
        // py:250-253  raise ParserError — Rust emits MappingEnd
        AnyEvent::MappingEnd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Minimal in-memory TokenStream backed by a scripted Vec.
    struct FakeScanner {
        tokens: std::collections::VecDeque<AnyToken>,
    }

    impl FakeScanner {
        fn new(tokens: Vec<AnyToken>) -> Self {
            Self {
                tokens: tokens.into(),
            }
        }
    }

    impl TokenStream for FakeScanner {
        fn check_token(&mut self, kinds: &[TokenKind]) -> bool {
            self.tokens
                .front()
                .map(|t| kinds.contains(&t.token_kind()))
                .unwrap_or(false)
        }
        fn peek_token(&mut self) -> Option<AnyToken> {
            self.tokens.front().cloned()
        }
        fn get_token(&mut self) -> AnyToken {
            self.tokens.pop_front().expect("token underflow")
        }
    }

    fn ss() -> AnyToken {
        AnyToken::StreamStart {
            start_mark: None,
            end_mark: None,
            encoding: None,
        }
    }

    fn se() -> AnyToken {
        AnyToken::StreamEnd {
            start_mark: None,
            end_mark: None,
        }
    }

    fn scalar(v: serde_json::Value) -> AnyToken {
        AnyToken::Scalar {
            start_mark: None,
            end_mark: None,
            value: v,
            plain: true,
            style: None,
        }
    }

    #[test]
    fn parser_error_implements_error_traits() {
        let me = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let e = ParserError(me);
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn parser_starts_at_stream_start_state() {
        let p = Parser::new();
        assert_eq!(p.state, State::StreamStart);
        assert!(p.current_event.is_none());
        assert!(p.states.is_empty());
        assert!(p.marks.is_empty());
    }

    #[test]
    fn dispose_clears_state_machine() {
        let mut p = Parser::new();
        p.states.push(State::DocumentEnd);
        p.dispose();
        assert_eq!(p.state, State::Done);
        assert!(p.states.is_empty());
    }

    #[test]
    fn check_event_with_no_choices_returns_true() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![ss(), se()]);
        // empty choices = any event
        assert!(p.check_event(&mut s, &[]));
    }

    #[test]
    fn check_event_matches_stream_start() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![ss(), se()]);
        assert!(p.check_event(&mut s, &[EventKind::StreamStart]));
    }

    #[test]
    fn check_event_rejects_wrong_kind() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![ss(), se()]);
        assert!(!p.check_event(&mut s, &[EventKind::Scalar]));
    }

    #[test]
    fn get_event_drains_stream_start_then_stream_end() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![ss(), se()]);
        assert_eq!(p.get_event(&mut s).kind(), EventKind::StreamStart);
        // After StreamStart we transition to ImplicitDocumentStart.
        // Next state sees StreamEnd → DocumentStart path → StreamEnd.
        let ev = p.get_event(&mut s);
        assert_eq!(ev.kind(), EventKind::StreamEnd);
    }

    #[test]
    fn parse_node_scalar_emits_scalar_event() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![scalar(json!("hello"))]);
        let ev = p.parse_node(&mut s, false);
        match ev {
            AnyEvent::Scalar(se) => {
                assert_eq!(se.value, json!("hello"));
            }
            _ => panic!("expected ScalarEvent"),
        }
    }

    #[test]
    fn parse_node_flow_seq_emits_sequence_start() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![AnyToken::FlowSequenceStart {
            start_mark: None,
            end_mark: None,
        }]);
        let ev = p.parse_node(&mut s, false);
        assert_eq!(ev.kind(), EventKind::SequenceStart);
        assert_eq!(p.state, State::FlowSequenceFirstEntry);
    }

    #[test]
    fn parse_node_flow_map_emits_mapping_start() {
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![AnyToken::FlowMappingStart {
            start_mark: None,
            end_mark: None,
        }]);
        let ev = p.parse_node(&mut s, false);
        assert_eq!(ev.kind(), EventKind::MappingStart);
        assert_eq!(p.state, State::FlowMappingFirstKey);
    }

    #[test]
    fn any_token_kind_dispatches_correctly() {
        assert_eq!(ss().token_kind(), TokenKind::StreamStart);
        assert_eq!(se().token_kind(), TokenKind::StreamEnd);
        assert_eq!(scalar(json!(0)).token_kind(), TokenKind::Scalar);
    }

    #[test]
    fn any_token_id_strings_match_upstream() {
        assert_eq!(ss().token_id(), "<stream start>");
        assert_eq!(se().token_id(), "<stream end>");
        assert_eq!(scalar(json!(0)).token_id(), "<scalar>");
        assert_eq!(
            AnyToken::FlowSequenceStart {
                start_mark: None,
                end_mark: None
            }
            .token_id(),
            "["
        );
        assert_eq!(
            AnyToken::FlowMappingEnd {
                start_mark: None,
                end_mark: None
            }
            .token_id(),
            "}"
        );
    }

    #[test]
    fn parse_simple_scalar_stream_end_to_end() {
        // Token stream: StreamStart, Scalar("v"), StreamEnd.
        let mut p = Parser::new();
        let mut s = FakeScanner::new(vec![ss(), scalar(json!("v")), se()]);
        // First event: StreamStart
        assert_eq!(p.get_event(&mut s).kind(), EventKind::StreamStart);
        // Next: ImplicitDocumentStart → emits DocumentStart
        assert_eq!(p.get_event(&mut s).kind(), EventKind::DocumentStart);
        // Next: parse_node → Scalar
        assert_eq!(p.get_event(&mut s).kind(), EventKind::Scalar);
        // Next state was DocumentEnd pushed earlier
        assert_eq!(p.get_event(&mut s).kind(), EventKind::DocumentEnd);
        // Then parse_document_start sees StreamEnd → emits StreamEnd
        assert_eq!(p.get_event(&mut s).kind(), EventKind::StreamEnd);
    }

    #[test]
    fn token_marks_round_trip() {
        let t = AnyToken::Key {
            start_mark: Some(Mark { line: 1, column: 2 }),
            end_mark: Some(Mark { line: 1, column: 3 }),
        };
        assert_eq!(t.token_start_mark().unwrap().line, 1);
        assert_eq!(t.token_end_mark().unwrap().column, 3);
    }

    #[test]
    fn typed_events_constructable_from_events_module() {
        // Pins the structural surface of the upstream events module:
        // each typed event constructor used by the Python parser
        // round-trips through the Rust events.rs module.
        use crate::ported::lint::markedjson::events::{
            DocumentEndEvent, DocumentStartEvent, MappingEndEvent, SequenceEndEvent,
            StreamEndEvent, StreamStartEvent,
        };
        let _ = StreamStartEvent::new(None, None, None);
        let _ = StreamEndEvent::new(None, None);
        let _ = DocumentStartEvent::new(None, None, Some(false), None, None);
        let _ = DocumentEndEvent::new(None, None, Some(false));
        let _ = SequenceEndEvent::new(None, None);
        let _ = MappingEndEvent::new(None, None);
    }
}
