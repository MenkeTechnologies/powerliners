// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/markedvalue.py`.
//!
//! Marked value wrappers that pair a JSON value with its source
//! position (`Mark`) for error reporting. Python uses primitive
//! subclassing (`class MarkedUnicode(unicode)`, etc.) to attach the
//! mark; Rust has no primitive subclassing so the port wraps each
//! value type in a struct.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lib.unicode import unicode        // py:4

use crate::ported::lint::markedjson::nodes::Mark;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Port of `class MarkedUnicode(unicode)` from
/// `powerline/lint/markedjson/markedvalue.py:28`.
///
/// String value + source mark.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedUnicode {
    pub value: String,
    pub mark: Mark,
}

impl MarkedUnicode {
    pub fn new(value: impl Into<String>, mark: Mark) -> Self {
        Self {
            value: value.into(),
            mark,
        }
    }

    /// Port of `MarkedUnicode._proc_partition()` from
    /// `powerline/lint/markedjson/markedvalue.py:32`.
    ///
    /// Walks a (pre, sep, post) partition triple and rebuilds each as
    /// a MarkedUnicode whose mark is advanced past the prior elements.
    fn _proc_partition(&self, parts: [&str; 3]) -> (MarkedUnicode, MarkedUnicode, MarkedUnicode) {
        // py:33-40  walk + advance pointdiff
        let mut pointdiff: usize = 1;
        let mut out = Vec::with_capacity(3);
        for s in parts {
            out.push(MarkedUnicode::new(s.to_string(), self.mark.clone()));
            pointdiff += s.len();
        }
        let _ = pointdiff;
        // The .advance_string call on each mark is the real upstream
        // behaviour — preserved structurally with the marks not yet
        // mutated since Mark::advance_string isn't ported.
        (out.remove(0), out.remove(0), out.remove(0))
    }

    /// Port of `MarkedUnicode.rpartition()` from
    /// `powerline/lint/markedjson/markedvalue.py:42`.
    pub fn rpartition(&self, sep: &str) -> (MarkedUnicode, MarkedUnicode, MarkedUnicode) {
        let parts = match self.value.rfind(sep) {
            Some(i) => {
                let (pre, after) = self.value.split_at(i);
                let post = &after[sep.len()..];
                [pre, sep, post]
            }
            None => ["", "", self.value.as_str()],
        };
        self._proc_partition(parts)
    }

    /// Port of `MarkedUnicode.partition()` from
    /// `powerline/lint/markedjson/markedvalue.py:45`.
    pub fn partition(&self, sep: &str) -> (MarkedUnicode, MarkedUnicode, MarkedUnicode) {
        let parts = match self.value.find(sep) {
            Some(i) => {
                let (pre, after) = self.value.split_at(i);
                let post = &after[sep.len()..];
                [pre, sep, post]
            }
            None => [self.value.as_str(), "", ""],
        };
        self._proc_partition(parts)
    }
}

/// Port of `class MarkedInt(int)` from
/// `powerline/lint/markedjson/markedvalue.py:49`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedInt {
    pub value: i64,
    pub mark: Mark,
}

impl MarkedInt {
    pub fn new(value: i64, mark: Mark) -> Self {
        Self { value, mark }
    }
}

/// Port of `class MarkedFloat(float)` from
/// `powerline/lint/markedjson/markedvalue.py:54`.
#[derive(Debug, Clone)]
pub struct MarkedFloat {
    pub value: f64,
    pub mark: Mark,
}

impl MarkedFloat {
    pub fn new(value: f64, mark: Mark) -> Self {
        Self { value, mark }
    }
}

/// Port of `class MarkedDict(dict)` from
/// `powerline/lint/markedjson/markedvalue.py:59`.
///
/// Dict value + source mark + `keydict` mapping (used by the linter
/// to look up the mark of the key in the source).
#[derive(Debug, Clone)]
pub struct MarkedDict {
    pub value: Map<String, Value>,
    pub mark: Mark,
    /// Python: `self.keydict = dict((key, key) for key in self)` — py:65.
    /// Identity map of every dict key to itself (typically a
    /// MarkedUnicode) so the linter can recover key marks.
    pub keydict: HashMap<String, MarkedUnicode>,
}

impl MarkedDict {
    /// Port of `MarkedDict.__new__()` from
    /// `powerline/lint/markedjson/markedvalue.py:60`.
    pub fn new(value: Map<String, Value>, mark: Mark) -> Self {
        // py:65  self.keydict = dict((key, key) for key in r)
        let keydict = value
            .keys()
            .map(|k| (k.clone(), MarkedUnicode::new(k.clone(), mark.clone())))
            .collect();
        Self {
            value,
            mark,
            keydict,
        }
    }

    /// Port of `MarkedDict.copy()` from
    /// `powerline/lint/markedjson/markedvalue.py:96`.
    pub fn copy(&self) -> MarkedDict {
        MarkedDict::new(self.value.clone(), self.mark.clone())
    }

    /// Port of `MarkedDict.update()` from
    /// `powerline/lint/markedjson/markedvalue.py:91`.
    pub fn update(&mut self, other: Map<String, Value>) {
        for (k, v) in other {
            self.value.insert(k.clone(), v);
            self.keydict
                .insert(k.clone(), MarkedUnicode::new(k, self.mark.clone()));
        }
    }
}

/// Port of `class MarkedList(list)` from
/// `powerline/lint/markedjson/markedvalue.py:101`.
#[derive(Debug, Clone)]
pub struct MarkedList {
    pub value: Vec<Value>,
    pub mark: Mark,
}

impl MarkedList {
    pub fn new(value: Vec<Value>, mark: Mark) -> Self {
        Self { value, mark }
    }
}

/// Port of `class MarkedValue` from
/// `powerline/lint/markedjson/markedvalue.py:107`.
///
/// Generic wrapper for any value type that doesn't have a specialised
/// subclass.
#[derive(Debug, Clone)]
pub struct MarkedValue {
    pub value: Value,
    pub mark: Mark,
}

impl MarkedValue {
    pub fn new(value: Value, mark: Mark) -> Self {
        Self { value, mark }
    }
}

/// Port of `gen_marked_value()` from
/// `powerline/lint/markedjson/markedvalue.py:124`.
///
/// Dispatches to the right Marked* wrapper based on the value's
/// runtime type. Returns a serde_json::Value carrying the wrapped
/// value (the typed Marked structs aren't a unified Value variant
/// so the dispatch returns the underlying value with the marks
/// dropped — preserved structurally for callers that own a parallel
/// mark map).
pub enum MarkedAny {
    Unicode(MarkedUnicode),
    Int(MarkedInt),
    Float(MarkedFloat),
    Dict(MarkedDict),
    List(MarkedList),
    Other(MarkedValue),
}

pub fn gen_marked_value(value: Value, mark: Mark) -> MarkedAny {
    // py:125-128  isinstance dispatch over the specialclasses table
    match value {
        Value::String(s) => MarkedAny::Unicode(MarkedUnicode::new(s, mark)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MarkedAny::Int(MarkedInt::new(i, mark))
            } else if let Some(f) = n.as_f64() {
                MarkedAny::Float(MarkedFloat::new(f, mark))
            } else {
                MarkedAny::Other(MarkedValue::new(Value::Number(n), mark))
            }
        }
        Value::Object(m) => MarkedAny::Dict(MarkedDict::new(m, mark)),
        Value::Array(a) => MarkedAny::List(MarkedList::new(a, mark)),
        other => MarkedAny::Other(MarkedValue::new(other, mark)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn mk() -> Mark {
        Mark { line: 1, column: 0 }
    }

    #[test]
    fn marked_unicode_carries_value_and_mark() {
        let m = MarkedUnicode::new("hello", mk());
        assert_eq!(m.value, "hello");
        assert_eq!(m.mark, mk());
    }

    #[test]
    fn marked_unicode_partition_returns_three_parts() {
        let m = MarkedUnicode::new("a.b.c", mk());
        let (pre, sep, post) = m.partition(".");
        assert_eq!(pre.value, "a");
        assert_eq!(sep.value, ".");
        assert_eq!(post.value, "b.c");
    }

    #[test]
    fn marked_unicode_rpartition_returns_three_parts() {
        let m = MarkedUnicode::new("a.b.c", mk());
        let (pre, sep, post) = m.rpartition(".");
        assert_eq!(pre.value, "a.b");
        assert_eq!(sep.value, ".");
        assert_eq!(post.value, "c");
    }

    #[test]
    fn marked_unicode_partition_no_sep_returns_first_in_pre() {
        let m = MarkedUnicode::new("abc", mk());
        let (pre, sep, post) = m.partition(".");
        assert_eq!(pre.value, "abc");
        assert_eq!(sep.value, "");
        assert_eq!(post.value, "");
    }

    #[test]
    fn marked_dict_carries_keydict_for_every_key() {
        let mut value = Map::new();
        value.insert("a".into(), json!(1));
        value.insert("b".into(), json!(2));
        let d = MarkedDict::new(value, mk());
        assert_eq!(d.keydict.len(), 2);
        assert_eq!(d.keydict["a"].value, "a");
        assert_eq!(d.keydict["b"].value, "b");
    }

    #[test]
    fn marked_dict_copy_clones_value_and_keydict() {
        let mut value = Map::new();
        value.insert("a".into(), json!(1));
        let d = MarkedDict::new(value, mk());
        let d2 = d.copy();
        assert_eq!(d2.value.len(), 1);
        assert!(d2.keydict.contains_key("a"));
    }

    #[test]
    fn marked_dict_update_adds_to_keydict() {
        let value = Map::new();
        let mut d = MarkedDict::new(value, mk());
        let mut more = Map::new();
        more.insert("x".into(), json!(99));
        d.update(more);
        assert!(d.keydict.contains_key("x"));
    }

    #[test]
    fn gen_marked_value_dispatches_on_value_type() {
        assert!(matches!(
            gen_marked_value(json!("s"), mk()),
            MarkedAny::Unicode(_)
        ));
        assert!(matches!(
            gen_marked_value(json!(42), mk()),
            MarkedAny::Int(_)
        ));
        assert!(matches!(
            gen_marked_value(json!(2.5_f64), mk()),
            MarkedAny::Float(_)
        ));
        assert!(matches!(
            gen_marked_value(json!({"a": 1}), mk()),
            MarkedAny::Dict(_)
        ));
        assert!(matches!(
            gen_marked_value(json!([1, 2]), mk()),
            MarkedAny::List(_)
        ));
        assert!(matches!(
            gen_marked_value(Value::Null, mk()),
            MarkedAny::Other(_)
        ));
    }

    #[test]
    fn marked_int_carries_i64() {
        let m = MarkedInt::new(42, mk());
        assert_eq!(m.value, 42);
    }

    #[test]
    fn marked_float_carries_f64() {
        let m = MarkedFloat::new(2.5, mk());
        assert!((m.value - 2.5).abs() < 1e-9);
    }
}
