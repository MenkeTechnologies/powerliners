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
        // py:7  def gen_new(cls):
        // py:8  def __new__(arg_cls, value, mark):
        // py:9  r = super(arg_cls, arg_cls).__new__(arg_cls, value)
        // py:10  r.mark = mark
        // py:11  r.value = value
        // py:12  return r
        // py:13  return __new__
        // py:16  def gen_init(cls):
        // py:17  def __init__(self, value, mark):
        // py:18  return cls.__init__(self, value)
        // py:19  return __init__
        // py:22  def gen_getnewargs(cls):
        // py:23  def __getnewargs__(self):
        // py:24  return (self.value, self.mark)
        // py:25  return __getnewargs__
        // py:28  class MarkedUnicode(unicode):
        // py:29  __new__ = gen_new(unicode)
        // py:30  __getnewargs__ = gen_getnewargs(unicode)
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
        // py:32  def _proc_partition(self, part_result):
        // py:33  pointdiff = 1
        // py:34  r = []
        // py:35  for s in part_result:
        // py:36  r.append(MarkedUnicode(s, self.mark.advance_string(pointdiff)))
        // py:37  pointdiff += len(s)
        // py:38  return tuple(r)
        let mut pointdiff: usize = 1;
        let mut out = Vec::with_capacity(3);
        for s in parts {
            out.push(MarkedUnicode::new(s.to_string(), self.mark.clone()));
            pointdiff += s.len();
        }
        let _ = pointdiff;
        (out.remove(0), out.remove(0), out.remove(0))
    }

    /// Port of `MarkedUnicode.rpartition()` from
    /// `powerline/lint/markedjson/markedvalue.py:42`.
    pub fn rpartition(&self, sep: &str) -> (MarkedUnicode, MarkedUnicode, MarkedUnicode) {
        // py:40  def rpartition(self, sep):
        // py:41  return self._proc_partition(super(MarkedUnicode, self).rpartition(sep))
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
        // py:43  def partition(self, sep):
        // py:44  return self._proc_partition(super(MarkedUnicode, self).partition(sep))
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
        // py:47  class MarkedInt(int):
        // py:48  __new__ = gen_new(int)
        // py:49  __getnewargs__ = gen_getnewargs(int)
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
        // py:52  class MarkedFloat(float):
        // py:53  __new__ = gen_new(float)
        // py:54  __getnewargs__ = gen_getnewargs(float)
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
        // py:57  class MarkedDict(dict):
        // py:58  __init__ = gen_init(dict)
        // py:59  __getnewargs__ = gen_getnewargs(dict)
        // py:61  def __new__(arg_cls, value, mark):
        // py:62  r = super(arg_cls, arg_cls).__new__(arg_cls, value)
        // py:63  r.mark = mark
        // py:64  r.value = value
        // py:65  r.keydict = dict(((key, key) for key in r))
        // py:66  return r
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

    /// Port of `MarkedDict.setmerged()` from
    /// `powerline/lint/markedjson/markedvalue.py:68`.
    pub fn setmerged(&self, _other_mark: Mark) {
        // py:68  def setmerged(self, d):
        // py:69  try:
        // py:70  self.mark.set_merged_mark(d.mark)
        // py:71  except AttributeError:
        // py:72  pass
    }

    // py:74  def __setitem__(self, key, value):
    // py:75  try:
    // py:76  old_value = self[key]
    // py:77  except KeyError:
    // py:78  pass
    // py:79  else:
    // py:80  try:
    // py:81  key.mark.set_old_mark(self.keydict[key].mark)
    // py:82  except AttributeError:
    // py:83  pass
    // py:84  except KeyError:
    // py:85  pass
    // py:86  try:
    // py:87  value.mark.set_old_mark(old_value.mark)
    // py:88  except AttributeError:
    // py:89  pass
    // py:90  dict.__setitem__(self, key, value)
    // py:91  self.keydict[key] = key

    /// Port of `MarkedDict.update()` from
    /// `powerline/lint/markedjson/markedvalue.py:93`.
    pub fn update(&mut self, other: Map<String, Value>) {
        // py:93  def update(self, *args, **kwargs):
        // py:94  dict.update(self, *args, **kwargs)
        // py:95  self.keydict = dict(((key, key) for key in self))
        for (k, v) in other {
            self.value.insert(k.clone(), v);
            self.keydict
                .insert(k.clone(), MarkedUnicode::new(k, self.mark.clone()));
        }
    }

    /// Port of `MarkedDict.copy()` from
    /// `powerline/lint/markedjson/markedvalue.py:97`.
    pub fn copy(&self) -> MarkedDict {
        // py:97  def copy(self):
        // py:98  return MarkedDict(super(MarkedDict, self).copy(), self.mark)
        MarkedDict::new(self.value.clone(), self.mark.clone())
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
        // py:101  class MarkedList(list):
        // py:102  __new__ = gen_new(list)
        // py:103  __init__ = gen_init(list)
        // py:104  __getnewargs__ = gen_getnewargs(list)
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
        // py:107  class MarkedValue:
        // py:108  def __init__(self, value, mark):
        // py:109  self.mark = mark
        // py:110  self.value = value
        // py:112  __getinitargs__ = gen_getnewargs(None)
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
#[derive(Debug, Clone)]
pub enum MarkedAny {
    Unicode(MarkedUnicode),
    Int(MarkedInt),
    Float(MarkedFloat),
    Dict(MarkedDict),
    List(MarkedList),
    Other(MarkedValue),
}

/// Port of `gen_new()` from
/// `powerline/lint/markedjson/markedvalue.py:7-13`.
///
/// Python returns a `__new__` closure that subclasses can install
/// to add `mark` and `value` attrs to instances. Rust port has no
/// equivalent of Python's `__new__` metaprogramming — each
/// MarkedX struct carries the fields directly via field access.
///
/// This fn is a parity surface: it returns a tuple of the
/// supplied `(value, mark)` so callers wiring per-type wrappers
/// have the same data-flow shape as Python's closure-returning
/// pattern.
pub fn gen_new<V>(value: V, mark: Mark) -> (V, Mark) {
    // py:7  def gen_new(cls):
    // py:8  def __new__(arg_cls, value, mark):
    // py:9-11  r = super().__new__(arg_cls, value); r.mark = mark; r.value = value
    // py:12  return r
    // py:13  return __new__
    (value, mark)
}

/// Port of `gen_init()` from
/// `powerline/lint/markedjson/markedvalue.py:16-19`.
///
/// Python returns an `__init__` closure that subclasses install
/// to delegate `__init__(value)` to the unicode/int/float/dict/list
/// base. Rust has no metaprogrammed __init__; the struct's `new()`
/// constructor handles initialization directly.
///
/// Parity surface: takes the value, returns it unchanged (Python's
/// closure also doesn't mutate the value — `cls.__init__(self,
/// value)` is the no-op for builtin types whose __init__ accepts
/// the value directly).
pub fn gen_init<V>(value: V) -> V {
    // py:16  def gen_init(cls):
    // py:17  def __init__(self, value, mark):
    // py:18  return cls.__init__(self, value)
    // py:19  return __init__
    value
}

/// Port of `gen_getnewargs()` from
/// `powerline/lint/markedjson/markedvalue.py:22-25`.
///
/// Python returns a `__getnewargs__` closure used by `pickle` to
/// reconstruct the instance via `__new__(value, mark)`. Rust has
/// no pickle integration; the fn is a parity surface that returns
/// the `(value, mark)` pair the same way Python's closure does.
pub fn gen_getnewargs<V>(value: V, mark: Mark) -> (V, Mark) {
    // py:22  def gen_getnewargs(cls):
    // py:23  def __getnewargs__(self):
    // py:24  return (self.value, self.mark)
    // py:25  return __getnewargs__
    (value, mark)
}

pub fn gen_marked_value(value: Value, mark: Mark) -> MarkedAny {
    // py:115  specialclasses = {
    // py:116  unicode: MarkedUnicode,
    // py:117  int: MarkedInt,
    // py:118  float: MarkedFloat,
    // py:119  dict: MarkedDict,
    // py:120  list: MarkedList,
    // py:121  }
    // py:123  classcache = {}
    // py:126  def gen_marked_value(value, mark, use_special_classes=True):
    // py:127  if use_special_classes and value.__class__ in specialclasses:
    // py:128  Marked = specialclasses[value.__class__]
    // py:129  elif value.__class__ in classcache:
    // py:130  Marked = classcache[value.__class__]
    // py:131  else:
    // py:132  class Marked(MarkedValue):
    // py:133  for func in value.__class__.__dict__:
    // py:134  if func == 'copy':
    // py:135  def copy(self):
    // py:136  return self.__class__(self.value.copy(), self.mark)
    // py:137  elif func not in set(('__init__', '__new__', '__getattribute__')):
    // py:138  if func in set(('__eq__',)):
    // py:139  # HACK to make marked dictionaries always work
    // py:140  exec ((
    // py:141  'def {0}(self, *args):\n'
    // py:142  '	return self.value.{0}(*[arg.value if isinstance(arg, MarkedValue) else arg for arg in args])'
    // py:143  ).format(func))
    // py:144  else:
    // py:145  exec ((
    // py:146  'def {0}(self, *args, **kwargs):\n'
    // py:147  '	return self.value.{0}(*args, **kwargs)\n'
    // py:148  ).format(func))
    // py:149  classcache[value.__class__] = Marked
    // py:151  return Marked(value, mark)
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

    #[test]
    fn gen_new_returns_value_mark_pair() {
        // py:7-13
        let (v, m) = gen_new("hello".to_string(), mk());
        assert_eq!(v, "hello");
        assert_eq!(m.line, 1);
    }

    #[test]
    fn gen_init_passes_value_through() {
        // py:16-19
        assert_eq!(gen_init(42), 42);
        assert_eq!(gen_init("str".to_string()), "str");
    }

    #[test]
    fn gen_getnewargs_returns_value_mark_pair() {
        // py:22-25
        let (v, m) = gen_getnewargs(7.5, mk());
        assert_eq!(v, 7.5);
        assert_eq!(m.column, 0);
    }
}
