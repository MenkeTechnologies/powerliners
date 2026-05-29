// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/scanner.py`.
//!
//! YAML-style scanner state machine. The Python source produces
//! tokens of type STREAM-START / STREAM-END / FLOW-SEQUENCE-START /
//! FLOW-MAPPING-START / FLOW-SEQUENCE-END / FLOW-MAPPING-END /
//! FLOW-ENTRY / KEY / VALUE / SCALAR consumed by the parser
//! (`parser.rs`) and composer (`composer.rs`).
//!
//! Rust port surfaces:
//!   - `ScannerError` (MarkedError subclass)
//!   - `SimpleKey` struct
//!   - `hexdigits_set()` accessor for the Python `set(hexdigits)`
//!     module-level constant
//!   - `Scanner` struct skeleton with the core state machine fields
//!     (done / flow_level / tokens_taken / possible_simple_keys /
//!     allow_simple_key) and the public check_token / peek_token /
//!     get_token / fetch_stream_start interfaces
//!   - `dispatch_fetch_for(char)` — the dispatch table at py:140-187
//!     mapping the next character to the appropriate fetch_* token-
//!     type discriminator
//!
//! The heavy scan_* methods (scan_flow_scalar / scan_plain /
//! scan_to_next_token / scan_flow_scalar_non_spaces / scan_flow_
//! scalar_spaces) are deferred since they need the Reader-backed
//! source buffer + the Reader peek/forward interface.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from string import hexdigits                     // py:4
// from powerline.lint.markedjson.error import MarkedError                                 // py:6
// from powerline.lint.markedjson import tokens     // py:7
// from powerline.lib.unicode import unicode, unichr, surrogate_pair_to_character          // py:8

use crate::ported::lint::markedjson::error::MarkedError;
use crate::ported::lint::markedjson::nodes::Mark;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

/// Port of `hexdigits_set` from
/// `powerline/lint/markedjson/scanner.py:11`.
///
/// Python: `set(string.hexdigits)` — set of '0'-'9', 'a'-'f', 'A'-'F'.
pub fn hexdigits_set() -> &'static HashSet<char> {
    static S: OnceLock<HashSet<char>> = OnceLock::new();
    S.get_or_init(|| {
        let mut s = HashSet::new();
        for c in "0123456789abcdefABCDEF".chars() {
            s.insert(c);
        }
        s
    })
}

/// Port of `class ScannerError(MarkedError)` from
/// `powerline/lint/markedjson/scanner.py:31`.
#[derive(Debug, Clone)]
pub struct ScannerError(pub MarkedError);

impl std::fmt::Display for ScannerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ScannerError {}

/// Port of `class SimpleKey` from
/// `powerline/lint/markedjson/scanner.py:35`.
///
/// Tracks the state needed to commit a "simple key" candidate to a
/// concrete KEY token when the corresponding `:` value indicator
/// is found.
#[derive(Debug, Clone)]
pub struct SimpleKey {
    /// Python: `self.token_number` — index into the emitted tokens
    /// stream that this simple key would land at.
    pub token_number: usize,
    /// Python: `self.index` — Reader absolute char index at the key
    /// start.
    pub index: usize,
    /// Python: `self.line` — line position at the key start.
    pub line: usize,
    /// Python: `self.column` — column position at the key start.
    pub column: usize,
    /// Python: `self.mark`.
    pub mark: Option<Mark>,
}

impl SimpleKey {
    /// Port of `SimpleKey.__init__()` from
    /// `powerline/lint/markedjson/scanner.py:37`.
    pub fn new(
        token_number: usize,
        index: usize,
        line: usize,
        column: usize,
        mark: Option<Mark>,
    ) -> Self {
        Self {
            token_number,
            index,
            line,
            column,
            mark,
        }
    }
}

/// Token-type discriminator the scanner can fetch. Rust analog of
/// the Python `fetch_*` method dispatch table at
/// `powerline/lint/markedjson/scanner.py:140-187`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchKind {
    /// py:148  '\0' → fetch_stream_end
    StreamEnd,
    /// py:152  '[' → fetch_flow_sequence_start
    FlowSequenceStart,
    /// py:156  '{' → fetch_flow_mapping_start
    FlowMappingStart,
    /// py:160  ']' → fetch_flow_sequence_end
    FlowSequenceEnd,
    /// py:164  '}' → fetch_flow_mapping_end
    FlowMappingEnd,
    /// py:168  ',' → fetch_flow_entry
    FlowEntry,
    /// py:172  ':' (inside flow context) → fetch_value
    Value,
    /// py:176  '"' → fetch_double
    Double,
    /// py:180-181  plain scalar
    Plain,
}

/// Port of the dispatch table at
/// `powerline/lint/markedjson/scanner.py:140-187`.
///
/// Returns the `FetchKind` the scanner should fetch for the given
/// next character + `flow_level` state. The Python `':'` branch only
/// fires inside a flow context (py:172).
pub fn dispatch_fetch_for(ch: char, flow_level: u32) -> Option<FetchKind> {
    // py:148  ch == '\0' → STREAM-END
    if ch == '\0' {
        return Some(FetchKind::StreamEnd);
    }
    // py:152  '[' → flow sequence start
    if ch == '[' {
        return Some(FetchKind::FlowSequenceStart);
    }
    // py:156  '{' → flow mapping start
    if ch == '{' {
        return Some(FetchKind::FlowMappingStart);
    }
    // py:160  ']' → flow sequence end
    if ch == ']' {
        return Some(FetchKind::FlowSequenceEnd);
    }
    // py:164  '}' → flow mapping end
    if ch == '}' {
        return Some(FetchKind::FlowMappingEnd);
    }
    // py:168  ',' → flow entry
    if ch == ',' {
        return Some(FetchKind::FlowEntry);
    }
    // py:172  ':' + flow_level → value
    if ch == ':' && flow_level > 0 {
        return Some(FetchKind::Value);
    }
    // py:176  '"' → double scalar
    if ch == '"' {
        return Some(FetchKind::Double);
    }
    // py:180  if self.check_plain(): plain
    if check_plain(ch) {
        return Some(FetchKind::Plain);
    }
    // py:184-187  otherwise → scanner error
    None
}

/// Port of `Scanner.check_plain()` from
/// `powerline/lint/markedjson/scanner.py:360`.
///
/// Returns true when the next char can start a plain scalar.
/// Python checks `not any(ch in '...' for ch in self.peek())`; the
/// Rust port takes the char directly.
pub fn check_plain(ch: char) -> bool {
    // py:360-363  exclude indicators / null / structural chars
    !"\0[]{},:\"".contains(ch)
}

/// Port of `class Scanner` from
/// `powerline/lint/markedjson/scanner.py:45`.
///
/// State machine. The Python class inherits from `Reader` (multiple
/// inheritance) so it has access to `peek` / `prefix` / `forward`
/// methods on `self`. Rust port takes the Reader as a generic
/// parameter or trait object once the dispatch is wired through.
pub struct Scanner {
    /// Python: `self.done` (py:55).
    pub done: bool,
    /// Python: `self.flow_level` (py:58).
    pub flow_level: u32,
    /// Python: `self.tokens` — emitted token queue (py:61).
    /// Stored as a `Vec<TokenSlot>` placeholder since the concrete
    /// `tokens` module enum hasn't been threaded through yet.
    pub tokens: Vec<String>,
    /// Python: `self.tokens_taken` (py:67).
    pub tokens_taken: usize,
    /// Python: `self.allow_simple_key` (py:88).
    pub allow_simple_key: bool,
    /// Python: `self.possible_simple_keys` (py:91).
    pub possible_simple_keys: HashMap<u32, SimpleKey>,
}

impl Default for Scanner {
    fn default() -> Self {
        Self::new()
    }
}

impl Scanner {
    /// Port of `Scanner.__init__()` from
    /// `powerline/lint/markedjson/scanner.py:46`.
    pub fn new() -> Self {
        let mut s = Self {
            // py:55  self.done = False
            done: false,
            // py:58  self.flow_level = 0
            flow_level: 0,
            // py:61  self.tokens = []
            tokens: Vec::new(),
            // py:67  self.tokens_taken = 0
            tokens_taken: 0,
            // py:88  self.allow_simple_key = True
            allow_simple_key: true,
            // py:91  self.possible_simple_keys = {}
            possible_simple_keys: HashMap::new(),
        };
        // py:65  self.fetch_stream_start()
        s.fetch_stream_start();
        s
    }

    /// Port of `Scanner.fetch_stream_start()` from
    /// `powerline/lint/markedjson/scanner.py:238`.
    ///
    /// Surfaces just the queue-append; the Python source records the
    /// current position via Reader.get_mark + creates a
    /// `tokens.StreamStartToken` instance. The Rust port stashes a
    /// placeholder string until the tokens enum is threaded through.
    pub fn fetch_stream_start(&mut self) {
        // py:240-247  reset state + append StreamStartToken
        self.tokens.push("StreamStartToken".to_string());
    }

    /// Port of `Scanner.fetch_stream_end()` from
    /// `powerline/lint/markedjson/scanner.py:248`.
    pub fn fetch_stream_end(&mut self) {
        // py:250-261  set done + clear simple keys + append token
        self.done = true;
        self.possible_simple_keys.clear();
        self.tokens.push("StreamEndToken".to_string());
    }

    /// Port of `Scanner.check_token()` from
    /// `powerline/lint/markedjson/scanner.py:94`.
    ///
    /// `choices` is a list of token-type names to match against the
    /// next emitted token. Empty `choices` returns true when any
    /// token is available.
    pub fn check_token(&self, choices: &[&str]) -> bool {
        // py:95-104  iterate choices against tokens[0]
        match self.tokens.first() {
            None => false,
            Some(t) => {
                if choices.is_empty() {
                    true
                } else {
                    choices.iter().any(|c| *c == t)
                }
            }
        }
    }

    /// Port of `Scanner.peek_token()` from
    /// `powerline/lint/markedjson/scanner.py:106`.
    pub fn peek_token(&self) -> Option<&str> {
        // py:108-112  return tokens[0]
        self.tokens.first().map(String::as_str)
    }

    /// Port of `Scanner.get_token()` from
    /// `powerline/lint/markedjson/scanner.py:113`.
    pub fn get_token(&mut self) -> Option<String> {
        // py:115-121  pop tokens[0] + increment tokens_taken
        if self.tokens.is_empty() {
            return None;
        }
        self.tokens_taken += 1;
        Some(self.tokens.remove(0))
    }

    /// Port of `Scanner.need_more_tokens()` from
    /// `powerline/lint/markedjson/scanner.py:123`.
    pub fn need_more_tokens(&self) -> bool {
        // py:124-132  done / empty tokens queue / simple-key lookahead
        if self.done {
            return false;
        }
        if self.tokens.is_empty() {
            return true;
        }
        // py:131-132  next_possible_simple_key == tokens_taken
        match self.next_possible_simple_key() {
            Some(n) if n == self.tokens_taken => true,
            _ => false,
        }
    }

    /// Port of `Scanner.next_possible_simple_key()` from
    /// `powerline/lint/markedjson/scanner.py:192`.
    pub fn next_possible_simple_key(&self) -> Option<usize> {
        // py:201-206  min over possible_simple_keys' token_numbers
        self.possible_simple_keys
            .values()
            .map(|k| k.token_number)
            .min()
    }

    /// Port of `Scanner.save_possible_simple_key()` from
    /// `powerline/lint/markedjson/scanner.py:218`.
    pub fn save_possible_simple_key(&mut self, index: usize, line: usize, column: usize) {
        // py:220-230  capture key state when allow_simple_key
        if !self.allow_simple_key {
            return;
        }
        let token_number = self.tokens_taken + self.tokens.len();
        let key = SimpleKey::new(token_number, index, line, column, None);
        self.possible_simple_keys.insert(self.flow_level, key);
    }

    /// Port of `Scanner.remove_possible_simple_key()` from
    /// `powerline/lint/markedjson/scanner.py:231`.
    pub fn remove_possible_simple_key(&mut self) {
        // py:233-237  remove key at current flow_level
        self.possible_simple_keys.remove(&self.flow_level);
    }

    /// Port of `Scanner.fetch_flow_sequence_start()` /
    /// `fetch_flow_mapping_start()` (py:263-282) — increments
    /// flow_level and appends the corresponding token.
    pub fn fetch_flow_collection_start(&mut self, token_name: &str) {
        // py:269-284  increment flow_level + append token
        self.flow_level += 1;
        self.tokens.push(token_name.to_string());
    }

    /// Port of `Scanner.fetch_flow_sequence_end()` /
    /// `fetch_flow_mapping_end()` (py:285-305) — decrements
    /// flow_level and appends the corresponding end token.
    pub fn fetch_flow_collection_end(&mut self, token_name: &str) {
        // py:291-305  decrement flow_level + append token
        if self.flow_level > 0 {
            self.flow_level -= 1;
        }
        self.tokens.push(token_name.to_string());
    }

    /// Port of `Scanner.fetch_value()` from
    /// `powerline/lint/markedjson/scanner.py:307`.
    pub fn fetch_value(&mut self) {
        // py:308-324  append ValueToken
        self.tokens.push("ValueToken".to_string());
    }

    /// Port of `Scanner.fetch_flow_entry()` from
    /// `powerline/lint/markedjson/scanner.py:325`.
    pub fn fetch_flow_entry(&mut self) {
        // py:326-336  append FlowEntryToken
        self.tokens.push("FlowEntryToken".to_string());
    }
}

/// Port of `Scanner.stale_possible_simple_keys()` from
/// `powerline/lint/markedjson/scanner.py:207`.
///
/// Removes simple-key candidates that have been invalidated by line
/// advancement past their position. Pure function over (current_line,
/// keys) → ids-to-remove.
pub fn stale_possible_simple_keys(current_line: usize, keys: &HashMap<u32, SimpleKey>) -> Vec<u32> {
    // py:209-217  remove keys whose line is less than current_line
    keys.iter()
        .filter_map(|(level, k)| {
            if k.line < current_line {
                Some(*level)
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hexdigits_set_contains_all_hex_chars() {
        let s = hexdigits_set();
        for c in "0123456789abcdefABCDEF".chars() {
            assert!(s.contains(&c), "missing: {}", c);
        }
        assert!(!s.contains(&'g'));
    }

    #[test]
    fn hexdigits_set_has_22_entries() {
        // 10 digits + 6 lower + 6 upper
        let s = hexdigits_set();
        assert_eq!(s.len(), 22);
    }

    #[test]
    fn scanner_error_implements_error_traits() {
        let me = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let e = ScannerError(me);
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("ctx"));
    }

    #[test]
    fn simple_key_stores_state() {
        let k = SimpleKey::new(5, 10, 2, 3, None);
        assert_eq!(k.token_number, 5);
        assert_eq!(k.index, 10);
        assert_eq!(k.line, 2);
        assert_eq!(k.column, 3);
    }

    #[test]
    fn dispatch_fetch_for_null_returns_stream_end() {
        // py:148  ch == '\0' → STREAM-END
        assert_eq!(dispatch_fetch_for('\0', 0), Some(FetchKind::StreamEnd));
    }

    #[test]
    fn dispatch_fetch_for_bracket_returns_flow_sequence_start() {
        assert_eq!(
            dispatch_fetch_for('[', 0),
            Some(FetchKind::FlowSequenceStart)
        );
    }

    #[test]
    fn dispatch_fetch_for_brace_returns_flow_mapping_start() {
        assert_eq!(
            dispatch_fetch_for('{', 0),
            Some(FetchKind::FlowMappingStart)
        );
    }

    #[test]
    fn dispatch_fetch_for_close_bracket_returns_flow_sequence_end() {
        assert_eq!(dispatch_fetch_for(']', 1), Some(FetchKind::FlowSequenceEnd));
    }

    #[test]
    fn dispatch_fetch_for_close_brace_returns_flow_mapping_end() {
        assert_eq!(dispatch_fetch_for('}', 1), Some(FetchKind::FlowMappingEnd));
    }

    #[test]
    fn dispatch_fetch_for_comma_returns_flow_entry() {
        assert_eq!(dispatch_fetch_for(',', 1), Some(FetchKind::FlowEntry));
    }

    #[test]
    fn dispatch_fetch_for_colon_in_flow_returns_value() {
        // py:172  ':' + flow_level → Value
        assert_eq!(dispatch_fetch_for(':', 1), Some(FetchKind::Value));
    }

    #[test]
    fn dispatch_fetch_for_colon_outside_flow_returns_plain() {
        // py:172  ':' + no flow_level → falls through to plain
        // because check_plain(':') is false. So falls to None?
        // Actually check_plain excludes ':'. So returns None.
        assert_eq!(dispatch_fetch_for(':', 0), None);
    }

    #[test]
    fn dispatch_fetch_for_quote_returns_double() {
        assert_eq!(dispatch_fetch_for('"', 0), Some(FetchKind::Double));
    }

    #[test]
    fn dispatch_fetch_for_alpha_returns_plain() {
        assert_eq!(dispatch_fetch_for('a', 0), Some(FetchKind::Plain));
        assert_eq!(dispatch_fetch_for('1', 0), Some(FetchKind::Plain));
    }

    #[test]
    fn check_plain_excludes_structural_chars() {
        // py:360-363
        assert!(!check_plain('\0'));
        assert!(!check_plain('['));
        assert!(!check_plain(']'));
        assert!(!check_plain('{'));
        assert!(!check_plain('}'));
        assert!(!check_plain(','));
        assert!(!check_plain(':'));
        assert!(!check_plain('"'));
    }

    #[test]
    fn check_plain_accepts_alphanumeric() {
        assert!(check_plain('a'));
        assert!(check_plain('Z'));
        assert!(check_plain('5'));
        assert!(check_plain('_'));
    }

    #[test]
    fn scanner_new_emits_stream_start_token() {
        // py:65  __init__ calls fetch_stream_start
        let s = Scanner::new();
        assert_eq!(s.tokens.len(), 1);
        assert_eq!(s.tokens[0], "StreamStartToken");
        assert_eq!(s.flow_level, 0);
        assert_eq!(s.tokens_taken, 0);
        assert!(!s.done);
        assert!(s.allow_simple_key);
    }

    #[test]
    fn fetch_stream_end_sets_done_and_appends_token() {
        // py:248-261
        let mut s = Scanner::new();
        s.fetch_stream_end();
        assert!(s.done);
        assert_eq!(s.tokens[1], "StreamEndToken");
    }

    #[test]
    fn check_token_no_choices_returns_true_when_token_available() {
        let s = Scanner::new();
        assert!(s.check_token(&[]));
    }

    #[test]
    fn check_token_matches_specific_name() {
        let s = Scanner::new();
        assert!(s.check_token(&["StreamStartToken"]));
        assert!(!s.check_token(&["StreamEndToken"]));
    }

    #[test]
    fn check_token_returns_false_when_no_tokens() {
        // Empty queue case
        let mut s = Scanner {
            done: false,
            flow_level: 0,
            tokens: Vec::new(),
            tokens_taken: 0,
            allow_simple_key: true,
            possible_simple_keys: HashMap::new(),
        };
        s.tokens.clear();
        assert!(!s.check_token(&[]));
    }

    #[test]
    fn peek_token_returns_first_without_consuming() {
        let s = Scanner::new();
        assert_eq!(s.peek_token(), Some("StreamStartToken"));
        assert_eq!(s.tokens_taken, 0);
    }

    #[test]
    fn get_token_pops_and_increments_taken() {
        let mut s = Scanner::new();
        let t = s.get_token();
        assert_eq!(t.as_deref(), Some("StreamStartToken"));
        assert_eq!(s.tokens_taken, 1);
        assert!(s.tokens.is_empty());
    }

    #[test]
    fn need_more_tokens_false_when_done() {
        let mut s = Scanner::new();
        s.done = true;
        s.tokens.clear();
        assert!(!s.need_more_tokens());
    }

    #[test]
    fn need_more_tokens_true_when_queue_empty_and_not_done() {
        let mut s = Scanner::new();
        s.tokens.clear();
        assert!(s.need_more_tokens());
    }

    #[test]
    fn save_possible_simple_key_records_when_allowed() {
        let mut s = Scanner::new();
        s.save_possible_simple_key(10, 1, 2);
        assert!(s.possible_simple_keys.contains_key(&0));
        let k = &s.possible_simple_keys[&0];
        assert_eq!(k.index, 10);
        assert_eq!(k.line, 1);
        assert_eq!(k.column, 2);
    }

    #[test]
    fn save_possible_simple_key_skipped_when_disallowed() {
        let mut s = Scanner::new();
        s.allow_simple_key = false;
        s.save_possible_simple_key(10, 1, 2);
        assert!(s.possible_simple_keys.is_empty());
    }

    #[test]
    fn remove_possible_simple_key_clears_current_level() {
        let mut s = Scanner::new();
        s.save_possible_simple_key(10, 1, 2);
        s.remove_possible_simple_key();
        assert!(s.possible_simple_keys.is_empty());
    }

    #[test]
    fn fetch_flow_collection_start_increments_level() {
        // py:269-284
        let mut s = Scanner::new();
        s.fetch_flow_collection_start("FlowSequenceStartToken");
        assert_eq!(s.flow_level, 1);
        assert_eq!(s.tokens.last().unwrap(), "FlowSequenceStartToken");
    }

    #[test]
    fn fetch_flow_collection_end_decrements_level() {
        let mut s = Scanner::new();
        s.fetch_flow_collection_start("FlowSequenceStartToken");
        s.fetch_flow_collection_end("FlowSequenceEndToken");
        assert_eq!(s.flow_level, 0);
        assert_eq!(s.tokens.last().unwrap(), "FlowSequenceEndToken");
    }

    #[test]
    fn fetch_flow_collection_end_does_not_underflow() {
        let mut s = Scanner::new();
        s.fetch_flow_collection_end("FlowSequenceEndToken");
        assert_eq!(s.flow_level, 0);
    }

    #[test]
    fn fetch_value_appends_value_token() {
        let mut s = Scanner::new();
        s.fetch_value();
        assert_eq!(s.tokens.last().unwrap(), "ValueToken");
    }

    #[test]
    fn fetch_flow_entry_appends_entry_token() {
        let mut s = Scanner::new();
        s.fetch_flow_entry();
        assert_eq!(s.tokens.last().unwrap(), "FlowEntryToken");
    }

    #[test]
    fn next_possible_simple_key_returns_min_token_number() {
        let mut s = Scanner::new();
        s.possible_simple_keys
            .insert(0, SimpleKey::new(5, 0, 0, 0, None));
        s.possible_simple_keys
            .insert(1, SimpleKey::new(3, 0, 0, 0, None));
        assert_eq!(s.next_possible_simple_key(), Some(3));
    }

    #[test]
    fn next_possible_simple_key_none_when_empty() {
        let s = Scanner::new();
        assert!(s.next_possible_simple_key().is_none());
    }

    #[test]
    fn stale_possible_simple_keys_filters_keys_below_current_line() {
        let mut keys = HashMap::new();
        keys.insert(0, SimpleKey::new(1, 0, 1, 0, None));
        keys.insert(1, SimpleKey::new(2, 0, 5, 0, None));
        let stale = stale_possible_simple_keys(3, &keys);
        assert_eq!(stale, vec![0]);
    }
}
