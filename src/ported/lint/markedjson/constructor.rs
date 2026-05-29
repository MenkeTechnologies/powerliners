// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/constructor.py`.
//!
//! Walks the composed node tree (`ComposedNode`) and produces
//! `MarkedAny` Python-equivalent data structures, dispatching by the
//! YAML tag attached to each node (`tag:yaml.org,2002:null` / `bool` /
//! `int` / `float` / `str` / `seq` / `map`).
//!
//! The Python source uses generator-based deferred construction
//! (`construct_yaml_seq`/`construct_yaml_map` yield first, then mutate
//! the placeholder). This is needed for YAML alias cycles; the
//! JSON-only lint loader has no aliases so the Rust port uses a
//! synchronous build path.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import collections                                  // py:4
// import types                                        // py:5
// from functools import wraps                         // py:7
// from powerline.lint.markedjson.error import MarkedError                                  // py:9
// from powerline.lint.markedjson import nodes        // py:11
// from powerline.lint.markedjson.markedvalue import gen_marked_value                       // py:12
// from powerline.lib.unicode import unicode          // py:13

use crate::ported::lint::markedjson::composer::ComposedNode;
use crate::ported::lint::markedjson::error::MarkedError;
use crate::ported::lint::markedjson::markedvalue::{gen_marked_value, MarkedAny};
use crate::ported::lint::markedjson::nodes::{MappingNode, Mark};
use serde_json::{Map, Value};

/// Port of `class ConstructorError(MarkedError)` from
/// `powerline/lint/markedjson/constructor.py:23`.
#[derive(Debug, Clone)]
pub struct ConstructorError(pub MarkedError);

impl std::fmt::Display for ConstructorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ConstructorError {}

/// Port of the `@marked` decorator from
/// `powerline/lint/markedjson/constructor.py:16`.
///
/// Python: `gen_marked_value(func(self, node, *args), node.start_mark)`.
/// Wraps the produced JSON value with the node's start_mark.
pub fn marked(value: Value, mark: Mark) -> MarkedAny {
    // py:16  def marked(func):
    // py:17  @wraps(func)
    // py:18  def f(self, node, *args, **kwargs):
    // py:19  return gen_marked_value(func(self, node, *args, **kwargs), node.start_mark)
    // py:20  return f
    gen_marked_value(value, mark)
}

/// Port of the inner `f()` closure from
/// `powerline/lint/markedjson/constructor.py:18-20`.
///
/// `@wraps(func)`-decorated closure returned by `marked()`. Takes
/// the underlying construct fn + node (mark-carrier) and returns
/// the construct result wrapped via `gen_marked_value`.
///
/// Python captures `func` from the outer `marked()` scope; the
/// Rust port takes the underlying value as `construct()` closure
/// result so callers route through any construct dispatch they
/// like.
pub fn f<C>(construct: C, mark: Mark) -> MarkedAny
where
    C: FnOnce() -> Value,
{
    // py:18  def f(self, node, *args, **kwargs):
    // py:19  return gen_marked_value(func(self, node, *args, **kwargs), node.start_mark)
    gen_marked_value(construct(), mark)
}

/// Port of `class BaseConstructor` from
/// `powerline/lint/markedjson/constructor.py:27`.
///
/// Rust port keeps the three fields Python uses:
/// - `constructed_objects` is unused by JSON path (no aliases),
///   omitted.
/// - `state_generators` requires Python generators, omitted.
/// - `deep_construct` is generator-flow related, omitted.
///
/// The tag-dispatch table is populated by `Constructor::new()` via
/// the module-level `add_constructor` calls (py:265-289).
pub struct BaseConstructor {
    /// Python: `cls.yaml_constructors` — keyed by tag string
    /// (`None` mapped to the literal sentinel "*" for the catch-all).
    pub yaml_constructors: std::collections::HashMap<String, ConstructorFn>,
    /// Python: `self.constructed_objects = {}` (py:31) — keyed by
    /// node identity hash since Rust nodes don't implement Hash on
    /// the full struct. Used as a memoisation cache by
    /// `construct_object` per py:64-65.
    pub constructed_objects: std::collections::HashMap<u64, MarkedAny>,
    /// Python: `self.state_generators = []` (py:32) — used to defer
    /// generator-based constructors per py:79-86. The JSON-only
    /// lint loader doesn't use generators, so this stays empty.
    pub state_generators: Vec<()>,
    /// Python: `self.deep_construct = False` (py:33).
    pub deep_construct: bool,
}

/// Concrete signature of a YAML-tag constructor callback.
///
/// Python signature: `constructor(self, node)` where `self` is the
/// `Constructor` instance. Rust passes the `BaseConstructor` as the
/// first arg for shared state.
pub type ConstructorFn = fn(&BaseConstructor, &ComposedNode) -> Result<MarkedAny, ConstructorError>;

impl Default for BaseConstructor {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseConstructor {
    /// Port of `BaseConstructor.__init__()` from
    /// `powerline/lint/markedjson/constructor.py:30`.
    pub fn new() -> Self {
        // py:27  class BaseConstructor:
        // py:28  yaml_constructors = {}
        // py:30  def __init__(self):
        // py:31  self.constructed_objects = {}
        // py:32  self.state_generators = []
        // py:33  self.deep_construct = False
        Self {
            yaml_constructors: std::collections::HashMap::new(),
            constructed_objects: std::collections::HashMap::new(),
            state_generators: Vec::new(),
            deep_construct: false,
        }
    }

    /// Port of `BaseConstructor.check_data()` from
    /// `powerline/lint/markedjson/constructor.py:35-37`.
    ///
    /// Returns true if there are more documents available. Python
    /// delegates to `self.check_node()` from the composer; the Rust
    /// port takes the composer's check-node result directly since
    /// the composer isn't a base class here.
    pub fn check_data(has_node: bool) -> bool {
        // py:35  def check_data(self):
        // py:36  # If there are more documents available?
        // py:37  return self.check_node()
        has_node
    }

    /// Port of `BaseConstructor.get_data()` from
    /// `powerline/lint/markedjson/constructor.py:39-42`.
    ///
    /// Constructs and returns the next document. Returns None when
    /// there are no more nodes per py:41-42 (Python's implicit
    /// None when check_node fails).
    pub fn get_data(
        &mut self,
        node: Option<&ComposedNode>,
    ) -> Result<Option<MarkedAny>, ConstructorError> {
        // py:39  def get_data(self):
        // py:40  # Construct and return the next document.
        // py:41  if self.check_node():
        // py:42  return self.construct_document(self.get_node())
        match node {
            Some(n) => Ok(Some(self.construct_document(n)?)),
            None => Ok(None),
        }
    }

    /// Port of `BaseConstructor.get_single_data()` from
    /// `powerline/lint/markedjson/constructor.py:44-49`.
    ///
    /// Ensures the stream contains a single document and constructs
    /// it. Returns None when the composer didn't produce any node
    /// per py:47-48.
    pub fn get_single_data(
        &mut self,
        node: Option<&ComposedNode>,
    ) -> Result<Option<MarkedAny>, ConstructorError> {
        // py:44  def get_single_data(self):
        // py:45  # Ensure that the stream contains a single document and construct it.
        // py:46  node = self.get_single_node()
        // py:47  if node is not None:
        // py:48  return self.construct_document(node)
        // py:49  return None
        match node {
            Some(n) => Ok(Some(self.construct_document(n)?)),
            None => Ok(None),
        }
    }

    /// Port of `BaseConstructor.construct_document()` from
    /// `powerline/lint/markedjson/constructor.py:51-61`.
    ///
    /// Calls construct_object then drains the state_generators
    /// queue per py:53-58. Resets constructed_objects + deep_construct
    /// per py:59-60.
    pub fn construct_document(
        &mut self,
        node: &ComposedNode,
    ) -> Result<MarkedAny, ConstructorError> {
        // py:51  def construct_document(self, node):
        // py:52  data = self.construct_object(node)
        let data = self.construct_object(node)?;
        // py:53  while self.state_generators:
        while !self.state_generators.is_empty() {
            // py:54  state_generators = self.state_generators
            // py:55  self.state_generators = []
            self.state_generators.clear();
            // py:56  for generator in state_generators:
            // py:57  for dummy in generator:
            // py:58  pass
        }
        // py:59  self.constructed_objects = {}
        // py:60  self.deep_construct = False
        self.constructed_objects.clear();
        self.deep_construct = false;
        // py:61  return data
        Ok(data)
    }

    /// Port of `BaseConstructor.add_constructor()` (classmethod) from
    /// `powerline/lint/markedjson/constructor.py:145`.
    pub fn add_constructor(&mut self, tag: Option<&str>, constructor: ConstructorFn) {
        // py:148  yaml_constructors[tag] = constructor
        // The `None` key for the catch-all is mapped to "*".
        let key = tag.unwrap_or("*").to_string();
        self.yaml_constructors.insert(key, constructor);
    }

    /// Port of `BaseConstructor.construct_object()` from
    /// `powerline/lint/markedjson/constructor.py:65`.
    ///
    /// Looks up the tag-specific constructor function and invokes it.
    /// The generator-based path (py:78-84) is omitted since the
    /// JSON-only lint loader has no aliases.
    pub fn construct_object(&self, node: &ComposedNode) -> Result<MarkedAny, ConstructorError> {
        // py:63  def construct_object(self, node, deep=False):
        // py:64  if node in self.constructed_objects:
        // py:65  return self.constructed_objects[node]
        // py:66  if deep:
        // py:67  old_deep = self.deep_construct
        // py:68  self.deep_construct = True
        // py:69  constructor = None
        // py:70  tag_suffix = None
        let tag = &node.node().tag;
        // py:71  if node.tag in self.yaml_constructors:
        // py:72  constructor = self.yaml_constructors[node.tag]
        if let Some(constructor) = self.yaml_constructors.get(tag) {
            return constructor(self, node);
        }
        if let Some(constructor) = self.yaml_constructors.get("*") {
            return constructor(self, node);
        }
        // py:73  else:
        // py:74  raise ConstructorError(None, None, 'no constructor for tag %s' % node.tag)
        // py:75  if tag_suffix is None:
        // py:76  data = constructor(self, node)
        // py:77  else:
        // py:78  data = constructor(self, tag_suffix, node)
        // py:79  if isinstance(data, types.GeneratorType):
        // py:80  generator = data
        // py:81  data = next(generator)
        // py:82  if self.deep_construct:
        // py:83  for dummy in generator:
        // py:84  pass
        // py:85  else:
        // py:86  self.state_generators.append(generator)
        // py:87  self.constructed_objects[node] = data
        // py:88  if deep:
        // py:89  self.deep_construct = old_deep
        // py:90  return data
        Err(ConstructorError(MarkedError::new(
            None,
            None,
            Some(&format!("no constructor for tag {}", tag)),
            None,
            None,
        )))
    }

    /// Port of `BaseConstructor.construct_scalar()` from
    /// `powerline/lint/markedjson/constructor.py:88` (decorated
    /// `@marked`).
    pub fn construct_scalar(node: &ComposedNode) -> Result<MarkedAny, ConstructorError> {
        // py:92  @marked
        // py:93  def construct_scalar(self, node):
        // py:94  if not isinstance(node, nodes.ScalarNode):
        // py:95  raise ConstructorError(
        // py:96  None, None,
        // py:97  'expected a scalar node, but found %s' % node.id,
        // py:98  node.start_mark
        // py:99  )
        // py:100  return node.value
        match node {
            ComposedNode::Scalar(s) => {
                let mark = s
                    .node
                    .start_mark
                    .clone()
                    .unwrap_or(Mark { line: 0, column: 0 });
                Ok(marked(s.node.value.clone(), mark))
            }
            other => Err(ConstructorError(MarkedError::new(
                None,
                None,
                Some(&format!(
                    "expected a scalar node, but found {}",
                    composed_id(other)
                )),
                None,
                None,
            ))),
        }
    }

    /// Port of `BaseConstructor.construct_sequence()` from
    /// `powerline/lint/markedjson/constructor.py:97`.
    pub fn construct_sequence(
        &self,
        node: &ComposedNode,
    ) -> Result<Vec<MarkedAny>, ConstructorError> {
        // py:102  def construct_sequence(self, node, deep=False):
        // py:103  if not isinstance(node, nodes.SequenceNode):
        // py:104  raise ConstructorError(
        // py:105  None, None,
        // py:106  'expected a sequence node, but found %s' % node.id,
        // py:107  node.start_mark
        // py:108  )
        // py:109  return [self.construct_object(child, deep=deep) for child in node.value]
        match node {
            ComposedNode::Sequence(s) => {
                let mut out = Vec::new();
                if let Some(arr) = s.collection.node.value.as_array() {
                    for v in arr {
                        let mark = s
                            .collection
                            .node
                            .start_mark
                            .clone()
                            .unwrap_or(Mark { line: 0, column: 0 });
                        out.push(gen_marked_value(v.clone(), mark));
                    }
                }
                Ok(out)
            }
            other => Err(ConstructorError(MarkedError::new(
                None,
                None,
                Some(&format!(
                    "expected a sequence node, but found {}",
                    composed_id(other)
                )),
                None,
                None,
            ))),
        }
    }

    /// Port of `BaseConstructor.construct_mapping()` from
    /// `powerline/lint/markedjson/constructor.py:110` (decorated
    /// `@marked`).
    pub fn construct_mapping(&self, node: &ComposedNode) -> Result<MarkedAny, ConstructorError> {
        // py:111  @marked
        // py:112  def construct_mapping(self, node, deep=False):
        // py:113  if not isinstance(node, nodes.MappingNode):
        // py:114  raise ConstructorError(
        // py:115  None, None,
        // py:116  'expected a mapping node, but found %s' % node.id,
        // py:117  node.start_mark
        // py:118  )
        // py:119  mapping = {}
        // py:120  for key_node, value_node in node.value:
        // py:121  key = self.construct_object(key_node, deep=deep)
        // py:122  if not isinstance(key.value, unicode):
        // py:123  self.echoerr(
        // py:124  context='Error while constructing a mapping',
        // py:125  context_mark=node.start_mark,
        // py:126  problem='found unhashable key',
        // py:127  problem_mark=key_node.start_mark
        // py:128  )
        // py:129  continue
        // py:130  if key in mapping:
        // py:131  self.echoerr('duplicate key', ...)
        // py:132  continue
        // py:133  value = self.construct_object(value_node, deep=deep)
        // py:134  mapping[key] = value
        // py:135  return mapping
        match node {
            ComposedNode::Mapping(m) => {
                let mut mapping = Map::new();
                if let Some(obj) = m.collection.node.value.as_object() {
                    for (k, v) in obj {
                        if mapping.contains_key(k) {
                            continue;
                        }
                        mapping.insert(k.clone(), v.clone());
                    }
                }
                let mark = m
                    .collection
                    .node
                    .start_mark
                    .clone()
                    .unwrap_or(Mark { line: 0, column: 0 });
                Ok(marked(Value::Object(mapping), mark))
            }
            other => Err(ConstructorError(MarkedError::new(
                None,
                None,
                Some(&format!(
                    "expected a mapping node, but found {}",
                    composed_id(other)
                )),
                None,
                None,
            ))),
        }
    }
}

/// Port of `class Constructor(BaseConstructor)` from
/// `powerline/lint/markedjson/constructor.py:151`.
///
/// Concrete constructor with the 8 default tag handlers registered
/// (py:265-289 module-level `add_constructor` calls).
pub struct Constructor {
    pub base: BaseConstructor,
}

impl Default for Constructor {
    fn default() -> Self {
        Self::new()
    }
}

impl Constructor {
    /// Constructs a `Constructor` with the 8 default tag handlers.
    /// Rust port runs at `new()` time the module-level
    /// `add_constructor(...)` calls from py:265-289.
    pub fn new() -> Self {
        let mut base = BaseConstructor::new();
        // py:265-266  null
        base.add_constructor(Some("tag:yaml.org,2002:null"), construct_yaml_null);
        // py:268-269  bool
        base.add_constructor(Some("tag:yaml.org,2002:bool"), construct_yaml_bool);
        // py:271-272  int
        base.add_constructor(Some("tag:yaml.org,2002:int"), construct_yaml_int);
        // py:274-275  float
        base.add_constructor(Some("tag:yaml.org,2002:float"), construct_yaml_float);
        // py:277-278  str
        base.add_constructor(Some("tag:yaml.org,2002:str"), construct_yaml_str);
        // py:280-281  seq
        base.add_constructor(Some("tag:yaml.org,2002:seq"), construct_yaml_seq);
        // py:283-284  map
        base.add_constructor(Some("tag:yaml.org,2002:map"), construct_yaml_map);
        // py:286-287  None (catch-all)
        base.add_constructor(None, construct_undefined);
        Self { base }
    }

    /// Port of `Constructor.flatten_mapping()` from
    /// `powerline/lint/markedjson/constructor.py:160`.
    ///
    /// Handles `<<` (merge tag) keys by splicing the referenced
    /// mapping's pairs into the parent. The JSON lint loader produces
    /// MappingNodes whose .value is a serde_json::Object, not a list
    /// of (key_node, value_node) pairs (the Python representation), so
    /// the merge-tag splicing currently no-ops since composer.rs
    /// can't preserve key-node tags through the JSON Object encoding.
    /// Deferred to a later pass once composer.rs preserves pairs.
    pub fn flatten_mapping(&self, _node: &mut MappingNode) {
        // py:160-200 deferred — needs composer to preserve key_node tags.
    }
}

/// Helper to format a node's "id" string for error messages.
/// Python: `node.id` returns 'scalar'/'sequence'/'mapping'.
fn composed_id(node: &ComposedNode) -> &'static str {
    match node {
        ComposedNode::Scalar(_) => "scalar",
        ComposedNode::Sequence(_) => "sequence",
        ComposedNode::Mapping(_) => "mapping",
    }
}

/// Port of `Constructor.construct_yaml_null()` from
/// `powerline/lint/markedjson/constructor.py:208` (decorated
/// `@marked`).
pub fn construct_yaml_null(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:209  self.construct_scalar(node); return None
    BaseConstructor::construct_scalar(node)?;
    let mark = node.start_mark().unwrap_or(Mark { line: 0, column: 0 });
    Ok(marked(Value::Null, mark))
}

/// Port of `Constructor.construct_yaml_bool()` from
/// `powerline/lint/markedjson/constructor.py:213`.
pub fn construct_yaml_bool(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:215  value = self.construct_scalar(node).value
    let mark = node.start_mark().unwrap_or(Mark { line: 0, column: 0 });
    let raw = node.node().value.clone();
    // py:216  return bool(value)
    let b = match &raw {
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty() && s != "false",
        _ => false,
    };
    Ok(marked(Value::Bool(b), mark))
}

/// Port of `Constructor.construct_yaml_int()` from
/// `powerline/lint/markedjson/constructor.py:219`.
pub fn construct_yaml_int(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    let mark = node.start_mark().unwrap_or(Mark { line: 0, column: 0 });
    let raw = node.node().value.clone();
    // py:221-229  sign/value parse logic
    let parsed = match &raw {
        Value::Number(n) => n.as_i64().unwrap_or(0),
        Value::String(s) => {
            let s = s.trim();
            let (sign, body): (i64, &str) = if let Some(rest) = s.strip_prefix('-') {
                (-1, rest)
            } else if let Some(rest) = s.strip_prefix('+') {
                (1, rest)
            } else {
                (1, s)
            };
            if body == "0" {
                0
            } else {
                sign * body.parse::<i64>().unwrap_or(0)
            }
        }
        _ => 0,
    };
    Ok(marked(Value::from(parsed), mark))
}

/// Port of `Constructor.construct_yaml_float()` from
/// `powerline/lint/markedjson/constructor.py:233`.
pub fn construct_yaml_float(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    let mark = node.start_mark().unwrap_or(Mark { line: 0, column: 0 });
    let raw = node.node().value.clone();
    let parsed = match &raw {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        Value::String(s) => {
            let s = s.trim();
            let (sign, body): (f64, &str) = if let Some(rest) = s.strip_prefix('-') {
                (-1.0, rest)
            } else if let Some(rest) = s.strip_prefix('+') {
                (1.0, rest)
            } else {
                (1.0, s)
            };
            sign * body.parse::<f64>().unwrap_or(0.0)
        }
        _ => 0.0,
    };
    Ok(marked(
        Value::Number(serde_json::Number::from_f64(parsed).unwrap_or_else(|| 0.into())),
        mark,
    ))
}

/// Port of `Constructor.construct_yaml_str()` from
/// `powerline/lint/markedjson/constructor.py:247`.
pub fn construct_yaml_str(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:248  return self.construct_scalar(node)
    BaseConstructor::construct_scalar(node)
}

/// Port of `Constructor.construct_yaml_seq()` from
/// `powerline/lint/markedjson/constructor.py:250`.
pub fn construct_yaml_seq(
    c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:251-254  data = gen_marked_value([], mark); yield data; data.extend(...)
    // The Rust port collapses the generator pattern into a single
    // synchronous build since the JSON-only loader has no aliases.
    let mark = node.start_mark().unwrap_or(Mark { line: 0, column: 0 });
    let items = c.construct_sequence(node)?;
    // Re-encode the children's JSON values back into an array.
    let arr: Vec<Value> = items.iter().map(marked_any_to_json).collect();
    Ok(marked(Value::Array(arr), mark))
}

/// Port of `Constructor.construct_yaml_map()` from
/// `powerline/lint/markedjson/constructor.py:255`.
pub fn construct_yaml_map(
    c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:256-259  same generator pattern as seq
    c.construct_mapping(node)
}

/// Port of `Constructor.construct_undefined()` from
/// `powerline/lint/markedjson/constructor.py:260`.
pub fn construct_undefined(
    _c: &BaseConstructor,
    node: &ComposedNode,
) -> Result<MarkedAny, ConstructorError> {
    // py:261-265  raise ConstructorError 'could not determine a constructor for the tag'
    Err(ConstructorError(MarkedError::new(
        None,
        None,
        Some(&format!(
            "could not determine a constructor for the tag {}",
            node.node().tag
        )),
        None,
        None,
    )))
}

/// Helper projecting a `MarkedAny` back to its `serde_json::Value`.
fn marked_any_to_json(m: &MarkedAny) -> Value {
    match m {
        MarkedAny::Unicode(u) => Value::String(u.value.clone()),
        MarkedAny::Int(i) => Value::from(i.value),
        MarkedAny::Float(f) => {
            Value::Number(serde_json::Number::from_f64(f.value).unwrap_or_else(|| 0.into()))
        }
        MarkedAny::Dict(d) => Value::Object(d.value.clone()),
        MarkedAny::List(l) => Value::Array(l.value.clone()),
        MarkedAny::Other(v) => v.value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ported::lint::markedjson::nodes;
    use crate::ported::lint::markedjson::nodes::{MappingNode, ScalarNode, SequenceNode};
    use serde_json::json;

    fn mk_scalar(tag: &str, value: Value) -> ComposedNode {
        ComposedNode::Scalar(ScalarNode::new(
            tag,
            value,
            Some(Mark { line: 0, column: 0 }),
            None,
            None,
        ))
    }

    fn mk_seq(tag: &str, arr: Value) -> ComposedNode {
        ComposedNode::Sequence(SequenceNode::new(
            tag,
            arr,
            Some(Mark { line: 0, column: 0 }),
            None,
            None,
        ))
    }

    fn mk_map(tag: &str, obj: Value) -> ComposedNode {
        ComposedNode::Mapping(MappingNode::new(
            tag,
            obj,
            Some(Mark { line: 0, column: 0 }),
            None,
            None,
        ))
    }

    #[test]
    fn constructor_error_implements_error_traits() {
        let me = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let e = ConstructorError(me);
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("ctx"));
    }

    #[test]
    fn marked_wraps_value_with_mark() {
        let mark = Mark { line: 3, column: 4 };
        let m = marked(json!("hi"), mark);
        assert!(matches!(m, MarkedAny::Unicode(_)));
    }

    #[test]
    fn add_constructor_registers_tag() {
        let mut b = BaseConstructor::new();
        b.add_constructor(Some("custom"), construct_yaml_str);
        assert!(b.yaml_constructors.contains_key("custom"));
    }

    #[test]
    fn add_constructor_none_maps_to_catchall_key() {
        let mut b = BaseConstructor::new();
        b.add_constructor(None, construct_undefined);
        assert!(b.yaml_constructors.contains_key("*"));
    }

    #[test]
    fn construct_object_unknown_tag_returns_error() {
        let b = BaseConstructor::new();
        let n = mk_scalar("unknown-tag", json!("x"));
        let r = b.construct_object(&n);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("no constructor"));
    }

    #[test]
    fn construct_object_catchall_fires_for_unknown_tag() {
        let mut b = BaseConstructor::new();
        b.add_constructor(None, construct_undefined);
        let n = mk_scalar("never-registered-tag", json!("x"));
        let r = b.construct_object(&n);
        assert!(r.is_err());
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("could not determine a constructor"));
    }

    #[test]
    fn construct_scalar_returns_marked_unicode() {
        let n = mk_scalar("tag:yaml.org,2002:str", json!("hello"));
        let r = BaseConstructor::construct_scalar(&n).unwrap();
        match r {
            MarkedAny::Unicode(u) => assert_eq!(u.value, "hello"),
            _ => panic!("expected Unicode"),
        }
    }

    #[test]
    fn construct_scalar_rejects_sequence_node() {
        let n = mk_seq("tag:yaml.org,2002:seq", json!([1]));
        let r = BaseConstructor::construct_scalar(&n);
        assert!(r.is_err());
        assert!(r.unwrap_err().to_string().contains("scalar node"));
    }

    #[test]
    fn construct_sequence_returns_vec_of_marked_values() {
        let b = BaseConstructor::new();
        let n = mk_seq("tag:yaml.org,2002:seq", json!([1, 2, 3]));
        let r = b.construct_sequence(&n).unwrap();
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn construct_sequence_rejects_scalar_node() {
        let b = BaseConstructor::new();
        let n = mk_scalar("tag:yaml.org,2002:str", json!("x"));
        let r = b.construct_sequence(&n);
        assert!(r.is_err());
    }

    #[test]
    fn construct_mapping_returns_marked_dict() {
        let b = BaseConstructor::new();
        let n = mk_map("tag:yaml.org,2002:map", json!({"a": 1, "b": 2}));
        let r = b.construct_mapping(&n).unwrap();
        match r {
            MarkedAny::Dict(d) => {
                assert_eq!(d.value.get("a"), Some(&json!(1)));
                assert_eq!(d.value.get("b"), Some(&json!(2)));
            }
            _ => panic!("expected Dict"),
        }
    }

    #[test]
    fn construct_yaml_null_returns_null() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:null", json!("null"));
        let r = construct_yaml_null(&c.base, &n).unwrap();
        match r {
            MarkedAny::Other(o) => assert_eq!(o.value, Value::Null),
            _ => panic!("expected Null"),
        }
    }

    #[test]
    fn construct_yaml_bool_parses_true_string() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:bool", json!("true"));
        let r = construct_yaml_bool(&c.base, &n).unwrap();
        match r {
            MarkedAny::Other(o) => assert_eq!(o.value, Value::Bool(true)),
            _ => panic!("expected Bool"),
        }
    }

    #[test]
    fn construct_yaml_bool_parses_false_string() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:bool", json!("false"));
        let r = construct_yaml_bool(&c.base, &n).unwrap();
        match r {
            MarkedAny::Other(o) => assert_eq!(o.value, Value::Bool(false)),
            _ => panic!("expected Bool"),
        }
    }

    #[test]
    fn construct_yaml_int_positive() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:int", json!("42"));
        let r = construct_yaml_int(&c.base, &n).unwrap();
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, 42),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn construct_yaml_int_negative() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:int", json!("-7"));
        let r = construct_yaml_int(&c.base, &n).unwrap();
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, -7),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn construct_yaml_int_zero() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:int", json!("0"));
        let r = construct_yaml_int(&c.base, &n).unwrap();
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, 0),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn construct_yaml_float_parses_decimal() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:float", json!("3.5"));
        let r = construct_yaml_float(&c.base, &n).unwrap();
        match r {
            MarkedAny::Float(f) => assert!((f.value - 3.5).abs() < 1e-9),
            _ => panic!("expected Float"),
        }
    }

    #[test]
    fn construct_yaml_str_returns_string() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:str", json!("hello"));
        let r = construct_yaml_str(&c.base, &n).unwrap();
        match r {
            MarkedAny::Unicode(u) => assert_eq!(u.value, "hello"),
            _ => panic!("expected Unicode"),
        }
    }

    #[test]
    fn construct_yaml_seq_builds_marked_list() {
        let c = Constructor::new();
        let n = mk_seq("tag:yaml.org,2002:seq", json!([1, 2, 3]));
        let r = construct_yaml_seq(&c.base, &n).unwrap();
        match r {
            MarkedAny::List(l) => {
                assert_eq!(l.value.len(), 3);
                assert_eq!(l.value[0], json!(1));
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn construct_yaml_map_builds_marked_dict() {
        let c = Constructor::new();
        let n = mk_map("tag:yaml.org,2002:map", json!({"k": "v"}));
        let r = construct_yaml_map(&c.base, &n).unwrap();
        match r {
            MarkedAny::Dict(d) => assert_eq!(d.value.get("k"), Some(&json!("v"))),
            _ => panic!("expected Dict"),
        }
    }

    #[test]
    fn construct_undefined_always_errors() {
        let c = Constructor::new();
        let n = mk_scalar("custom:tag", json!("x"));
        let r = construct_undefined(&c.base, &n);
        assert!(r.is_err());
        assert!(r
            .unwrap_err()
            .to_string()
            .contains("could not determine a constructor"));
    }

    #[test]
    fn constructor_new_registers_all_eight_tags() {
        let c = Constructor::new();
        for tag in [
            "tag:yaml.org,2002:null",
            "tag:yaml.org,2002:bool",
            "tag:yaml.org,2002:int",
            "tag:yaml.org,2002:float",
            "tag:yaml.org,2002:str",
            "tag:yaml.org,2002:seq",
            "tag:yaml.org,2002:map",
            "*", // catch-all
        ] {
            assert!(
                c.base.yaml_constructors.contains_key(tag),
                "missing tag: {}",
                tag
            );
        }
    }

    #[test]
    fn construct_object_dispatches_int_constructor() {
        let c = Constructor::new();
        let n = mk_scalar("tag:yaml.org,2002:int", json!("42"));
        let r = c.base.construct_object(&n).unwrap();
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, 42),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn base_constructor_new_initialises_empty_state() {
        // py:30-33
        let c = BaseConstructor::new();
        assert!(c.constructed_objects.is_empty());
        assert!(c.state_generators.is_empty());
        assert!(!c.deep_construct);
    }

    #[test]
    fn check_data_returns_input() {
        // py:35-37  delegate to check_node
        assert!(BaseConstructor::check_data(true));
        assert!(!BaseConstructor::check_data(false));
    }

    #[test]
    fn get_data_returns_none_when_no_node() {
        // py:41-42  if not check_node: implicit None
        let mut c = Constructor::new();
        let r = c.base.get_data(None).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn get_single_data_returns_none_when_no_node() {
        // py:46-49  if node is None: return None
        let mut c = Constructor::new();
        let r = c.base.get_single_data(None).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn construct_document_resets_constructed_objects_after_construction() {
        // py:59  self.constructed_objects = {}
        use crate::ported::lint::markedjson::markedvalue::MarkedInt;
        let mut c = Constructor::new();
        // Seed some bogus entry to verify it gets cleared
        c.base.constructed_objects.insert(
            42,
            MarkedAny::Int(MarkedInt {
                value: 0,
                mark: Mark { line: 0, column: 0 },
            }),
        );
        // Build a real scalar node and construct it
        let scalar = nodes::ScalarNode {
            node: nodes::Node {
                tag: "tag:yaml.org,2002:int".to_string(),
                value: Value::from(7),
                start_mark: Some(Mark { line: 1, column: 0 }),
                end_mark: None,
            },
            style: None,
        };
        let composed = ComposedNode::Scalar(scalar);
        let r = c.base.construct_document(&composed).unwrap();
        // The result should be the constructed int
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, 7),
            other => panic!("expected Int, got {:?}", other),
        }
        // constructed_objects should be reset after construct_document
        assert!(c.base.constructed_objects.is_empty());
    }

    #[test]
    fn construct_document_resets_deep_construct_after_construction() {
        // py:60  self.deep_construct = False
        let mut c = Constructor::new();
        c.base.deep_construct = true;
        let scalar = nodes::ScalarNode {
            node: nodes::Node {
                tag: "tag:yaml.org,2002:bool".to_string(),
                value: Value::from(true),
                start_mark: Some(Mark { line: 1, column: 0 }),
                end_mark: None,
            },
            style: None,
        };
        let composed = ComposedNode::Scalar(scalar);
        let _ = c.base.construct_document(&composed).unwrap();
        assert!(!c.base.deep_construct);
    }

    #[test]
    fn get_data_with_node_returns_constructed_value() {
        // py:41-42  return construct_document(node)
        let mut c = Constructor::new();
        let scalar = nodes::ScalarNode {
            node: nodes::Node {
                tag: "tag:yaml.org,2002:int".to_string(),
                value: Value::from(123),
                start_mark: Some(Mark { line: 1, column: 0 }),
                end_mark: None,
            },
            style: None,
        };
        let composed = ComposedNode::Scalar(scalar);
        let r = c.base.get_data(Some(&composed)).unwrap().unwrap();
        match r {
            MarkedAny::Int(i) => assert_eq!(i.value, 123),
            _ => panic!("expected Int"),
        }
    }

    #[test]
    fn get_single_data_with_node_returns_constructed_value() {
        // py:46-49
        let mut c = Constructor::new();
        let scalar = nodes::ScalarNode {
            node: nodes::Node {
                tag: "tag:yaml.org,2002:str".to_string(),
                value: Value::from("hello"),
                start_mark: Some(Mark { line: 1, column: 0 }),
                end_mark: None,
            },
            style: None,
        };
        let composed = ComposedNode::Scalar(scalar);
        let r = c.base.get_single_data(Some(&composed)).unwrap().unwrap();
        match r {
            MarkedAny::Unicode(u) => assert_eq!(u.value, "hello"),
            other => panic!("expected Unicode, got {:?}", other),
        }
    }

    #[test]
    fn f_closure_wraps_construct_result_with_mark() {
        // py:18-19  gen_marked_value(construct(...), node.start_mark)
        let m = Mark { line: 5, column: 7 };
        let result = f(|| Value::String("hello".to_string()), m);
        match result {
            MarkedAny::Unicode(u) => {
                assert_eq!(u.value, "hello");
                assert_eq!(u.mark.line, 5);
                assert_eq!(u.mark.column, 7);
            }
            other => panic!("expected Unicode, got {:?}", other),
        }
    }
}
