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
    // py:18  return gen_marked_value(func(self, node), node.start_mark)
    gen_marked_value(value, mark)
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
        Self {
            yaml_constructors: std::collections::HashMap::new(),
        }
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
        let tag = &node.node().tag;
        // py:71-75  yaml_constructors[node.tag] or error
        if let Some(constructor) = self.yaml_constructors.get(tag) {
            return constructor(self, node);
        }
        // Fallback to the catch-all None key (py:288 add_constructor(None, ...))
        if let Some(constructor) = self.yaml_constructors.get("*") {
            return constructor(self, node);
        }
        // py:74-75  raise ConstructorError
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
        // py:89-94  isinstance(node, ScalarNode)?
        match node {
            ComposedNode::Scalar(s) => {
                let mark = s
                    .node
                    .start_mark
                    .clone()
                    .unwrap_or(Mark { line: 0, column: 0 });
                // py:95  return node.value  (with @marked wrapping)
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
        // py:98-103  isinstance(node, SequenceNode)?
        match node {
            ComposedNode::Sequence(s) => {
                // py:104-107  [construct_object(child) for child in node.value]
                // Note: the composer collapses node.value into a JSON
                // array of child values rather than child nodes
                // (composer.rs compose_sequence_node line 312). For
                // construct_sequence to recurse, we'd need child
                // nodes — which the JSON-only loader doesn't preserve.
                // Iterate over the JSON values directly and wrap each
                // as a scalar.
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
        // py:111-116  isinstance(node, MappingNode)?
        match node {
            ComposedNode::Mapping(m) => {
                // py:117  mapping = {}
                let mut mapping = Map::new();
                if let Some(obj) = m.collection.node.value.as_object() {
                    // py:118-138  iterate (key_node, value_node) pairs
                    for (k, v) in obj {
                        // py:127-130  isinstance(key.value, unicode)?
                        // (string-key check; JSON map keys are always
                        // strings, so this is always satisfied.)
                        if mapping.contains_key(k) {
                            // py:132-136  duplicate key — emit and skip
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
}
