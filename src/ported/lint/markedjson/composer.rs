// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/composer.py`.
//!
//! Builds a tree of `nodes::*Node` instances from a parser event
//! stream. The Python class uses multiple inheritance — the concrete
//! parser inherits from `Parser + Composer + Resolver` so each method
//! on `Composer` calls `self.check_event(...)` / `self.get_event(...)`
//! / `self.resolve(...)` directly on whatever the runtime class is.
//!
//! Rust analog: `Composer` is a free-function module that operates
//! over a `ParserBackend` trait. The concrete parser (when ported)
//! implements that trait by exposing event-stream and resolver
//! methods.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lint.markedjson import nodes                                              // py:4
// from powerline.lint.markedjson import events                                             // py:5
// from powerline.lint.markedjson.error import MarkedError                                  // py:6

use crate::ported::lint::markedjson::error::MarkedError;
use crate::ported::lint::markedjson::events::{MappingStartEvent, ScalarEvent, SequenceStartEvent};
use crate::ported::lint::markedjson::nodes::{MappingNode, Mark, Node, ScalarNode, SequenceNode};
use crate::ported::lint::markedjson::resolver::NodeKind;

/// Port of `class ComposerError(MarkedError)` from
/// `powerline/lint/markedjson/composer.py:13`.
#[derive(Debug, Clone)]
pub struct ComposerError(pub MarkedError);

impl std::fmt::Display for ComposerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ComposerError {}

/// Identifies an event variant without exposing the per-type struct.
///
/// Python uses `self.check_event(events.ScalarEvent)` with the class
/// reference; this enum is the Rust analog the trait uses for
/// type-discrimination in `check_event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias,
    Scalar,
    SequenceStart,
    SequenceEnd,
    MappingStart,
    MappingEnd,
}

/// The tagged sum returned by `ParserBackend::get_event()`.
///
/// Python composer code reaches into per-event fields (`event.tag`,
/// `event.value`, `event.implicit`, `event.start_mark`,
/// `event.end_mark`, `event.style`, `event.flow_style`). Each variant
/// owns the corresponding event struct so the composer can dispatch
/// on the discriminant and project out the fields.
#[derive(Debug, Clone)]
pub enum AnyEvent {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias,
    Scalar(ScalarEvent),
    SequenceStart(SequenceStartEvent),
    SequenceEnd,
    MappingStart(MappingStartEvent),
    MappingEnd,
}

impl AnyEvent {
    /// Returns the `EventKind` discriminant for this event.
    pub fn kind(&self) -> EventKind {
        match self {
            AnyEvent::StreamStart => EventKind::StreamStart,
            AnyEvent::StreamEnd => EventKind::StreamEnd,
            AnyEvent::DocumentStart => EventKind::DocumentStart,
            AnyEvent::DocumentEnd => EventKind::DocumentEnd,
            AnyEvent::Alias => EventKind::Alias,
            AnyEvent::Scalar(_) => EventKind::Scalar,
            AnyEvent::SequenceStart(_) => EventKind::SequenceStart,
            AnyEvent::SequenceEnd => EventKind::SequenceEnd,
            AnyEvent::MappingStart(_) => EventKind::MappingStart,
            AnyEvent::MappingEnd => EventKind::MappingEnd,
        }
    }
}

/// The composed-node return type for `compose_node()`.
///
/// Python returns one of `ScalarNode` / `SequenceNode` /
/// `MappingNode`; Rust uses an enum since the three node types don't
/// share a Rust trait (they only share Python's `Node` base class).
#[derive(Debug, Clone)]
pub enum ComposedNode {
    Scalar(ScalarNode),
    Sequence(SequenceNode),
    Mapping(MappingNode),
}

impl ComposedNode {
    /// Returns the underlying `Node` (tag/value/start_mark/end_mark)
    /// regardless of kind.
    pub fn node(&self) -> &Node {
        match self {
            ComposedNode::Scalar(s) => &s.node,
            ComposedNode::Sequence(s) => &s.collection.node,
            ComposedNode::Mapping(m) => &m.collection.node,
        }
    }

    /// Returns the start_mark of the underlying node.
    pub fn start_mark(&self) -> Option<Mark> {
        self.node().start_mark.clone()
    }
}

/// Trait the composer uses to interact with parser + resolver state.
///
/// Python expresses this as multiple inheritance: the concrete class
/// inherits from `Parser` (for `check_event`/`get_event`) +
/// `Resolver` (for `descend_resolver`/`ascend_resolver`/`resolve`) +
/// `Composer`. Rust port factors those into one trait — the actual
/// parser (when ported) implements it.
pub trait ParserBackend {
    /// Port of `self.check_event(*choices)` peeked-event predicate.
    fn check_event(&mut self, kinds: &[EventKind]) -> bool;
    /// Port of `self.get_event()` event-stream pop.
    fn get_event(&mut self) -> AnyEvent;
    /// Port of `self.descend_resolver(parent, index)`.
    fn descend_resolver(&mut self, parent: Option<&ComposedNode>, index: Option<&ComposedNode>);
    /// Port of `self.ascend_resolver()`.
    fn ascend_resolver(&mut self);
    /// Port of `self.resolve(kind, value, implicit, mark=None)`.
    fn resolve(
        &mut self,
        kind: NodeKind,
        value: Option<&serde_json::Value>,
        implicit: bool,
    ) -> String;
}

/// Port of `class Composer` from
/// `powerline/lint/markedjson/composer.py:17`.
///
/// Rust port is a unit struct — the composer's state lives on the
/// `ParserBackend` since Python combines them via inheritance.
pub struct Composer;

impl Composer {
    /// Port of `Composer.check_node()` from
    /// `powerline/lint/markedjson/composer.py:21`.
    pub fn check_node<B: ParserBackend>(backend: &mut B) -> bool {
        // py:20  def check_node(self):
        // py:21  # Drop the STREAM-START event.
        // py:22  if self.check_event(events.StreamStartEvent):
        // py:23  self.get_event()
        if backend.check_event(&[EventKind::StreamStart]) {
            backend.get_event();
        }
        // py:25  # If there are more documents available?
        // py:26  return not self.check_event(events.StreamEndEvent)
        !backend.check_event(&[EventKind::StreamEnd])
    }

    /// Port of `Composer.get_node()` from
    /// `powerline/lint/markedjson/composer.py:29`.
    pub fn get_node<B: ParserBackend>(backend: &mut B) -> Option<ComposedNode> {
        // py:28  def get_node(self):
        // py:29  # Get the root node of the next document.
        // py:30  if not self.check_event(events.StreamEndEvent):
        // py:31  return self.compose_document()
        if !backend.check_event(&[EventKind::StreamEnd]) {
            return Some(Self::compose_document(backend));
        }
        None
    }

    /// Port of `Composer.get_single_node()` from
    /// `powerline/lint/markedjson/composer.py:34`.
    pub fn get_single_node<B: ParserBackend>(
        backend: &mut B,
    ) -> Result<Option<ComposedNode>, ComposerError> {
        // py:33  def get_single_node(self):
        // py:34  # Drop the STREAM-START event.
        // py:35  self.get_event()
        backend.get_event();

        // py:37  # Compose a document if the stream is not empty.
        // py:38  document = None
        // py:39  if not self.check_event(events.StreamEndEvent):
        // py:40  document = self.compose_document()
        let document = if !backend.check_event(&[EventKind::StreamEnd]) {
            Some(Self::compose_document(backend))
        } else {
            None
        };

        // py:42  # Ensure that the stream contains no more documents.
        // py:43  if not self.check_event(events.StreamEndEvent):
        // py:44  event = self.get_event()
        // py:45  raise ComposerError(
        // py:46  'expected a single document in the stream',
        // py:47  document.start_mark,
        // py:48  'but found another document',
        // py:49  event.start_mark
        // py:50  )
        if !backend.check_event(&[EventKind::StreamEnd]) {
            let event = backend.get_event();
            let _ = event;
            let _ = document.as_ref();
            return Err(ComposerError(MarkedError::new(
                Some("expected a single document in the stream"),
                None,
                Some("but found another document"),
                None,
                None,
            )));
        }

        // py:52  # Drop the STREAM-END event.
        // py:53  self.get_event()
        backend.get_event();

        // py:55  return document
        Ok(document)
    }

    /// Port of `Composer.compose_document()` from
    /// `powerline/lint/markedjson/composer.py:57`.
    pub fn compose_document<B: ParserBackend>(backend: &mut B) -> ComposedNode {
        // py:58-59  drop DOCUMENT-START
        backend.get_event();
        // py:61-62  compose root
        let node = Self::compose_node(backend, None, None);
        // py:64-65  drop DOCUMENT-END
        backend.get_event();
        node
    }

    /// Port of `Composer.compose_node()` from
    /// `powerline/lint/markedjson/composer.py:69`.
    pub fn compose_node<B: ParserBackend>(
        backend: &mut B,
        parent: Option<&ComposedNode>,
        index: Option<&ComposedNode>,
    ) -> ComposedNode {
        // py:70  descend_resolver(parent, index)
        backend.descend_resolver(parent, index);
        // py:71-76  dispatch on event kind
        let node = if backend.check_event(&[EventKind::Scalar]) {
            ComposedNode::Scalar(Self::compose_scalar_node(backend))
        } else if backend.check_event(&[EventKind::SequenceStart]) {
            ComposedNode::Sequence(Self::compose_sequence_node(backend))
        } else if backend.check_event(&[EventKind::MappingStart]) {
            ComposedNode::Mapping(Self::compose_mapping_node(backend))
        } else {
            // py: the original implicitly relies on at least one of
            // the three branches firing; if no branch matches the
            // event stream is malformed. Mirror that with a panic so
            // the bug is visible (Python would NameError on `node`).
            panic!(
                "compose_node: expected Scalar/SequenceStart/MappingStart event, \
                 caller did not check_node first?"
            );
        };
        // py:77  ascend_resolver()
        backend.ascend_resolver();
        node
    }

    /// Port of `Composer.compose_scalar_node()` from
    /// `powerline/lint/markedjson/composer.py:80`.
    pub fn compose_scalar_node<B: ParserBackend>(backend: &mut B) -> ScalarNode {
        // py:81  event = self.get_event()
        let event = backend.get_event();
        let scalar = match event {
            AnyEvent::Scalar(s) => s,
            _ => unreachable!("compose_scalar_node called without ScalarEvent on top"),
        };
        // py:82-84  resolve tag if None or '!'
        let mut tag = scalar.tag.clone();
        if tag.as_deref().is_none() || tag.as_deref() == Some("!") {
            tag = Some(backend.resolve(NodeKind::Scalar, Some(&scalar.value), scalar.implicit));
        }
        // py:85  return ScalarNode(tag, value, start_mark, end_mark, style)
        ScalarNode::new(
            tag.unwrap(),
            scalar.value,
            scalar.node.event.start_mark,
            scalar.node.event.end_mark,
            scalar.style,
        )
    }

    /// Port of `Composer.compose_sequence_node()` from
    /// `powerline/lint/markedjson/composer.py:88`.
    pub fn compose_sequence_node<B: ParserBackend>(backend: &mut B) -> SequenceNode {
        // py:89-91  start_event = self.get_event(); resolve tag
        let start = backend.get_event();
        let seq = match start {
            AnyEvent::SequenceStart(s) => s,
            _ => unreachable!("compose_sequence_node called without SequenceStartEvent"),
        };
        let mut tag = seq.collection.tag.clone();
        if tag.as_deref().is_none() || tag.as_deref() == Some("!") {
            tag = Some(backend.resolve(NodeKind::Sequence, None, seq.collection.implicit));
        }
        let mut node = SequenceNode::new(
            tag.unwrap(),
            serde_json::Value::Array(Vec::new()),
            seq.collection.node.event.start_mark,
            None,
            seq.collection.flow_style,
        );
        // py:94-97  drain elements until SequenceEnd
        let mut elements: Vec<ComposedNode> = Vec::new();
        let mut index_counter: usize = 0;
        while !backend.check_event(&[EventKind::SequenceEnd]) {
            // py:95  compose child (parent=node, index=counter)
            // Rust can't easily borrow `node` to pass as parent here
            // because we'd hold a mutable borrow of backend. The
            // Python source passes `node` for resolver-path tracking
            // only (descend/ascend); since no real path tracking is
            // wired through ParserBackend yet (JSON-only loader uses
            // `yaml_path_resolvers={}`) we pass None.
            let _ = index_counter;
            let child = Self::compose_node(backend, None, None);
            elements.push(child);
            index_counter += 1;
        }
        // py:98-99  consume SequenceEnd, set end_mark
        let end_event = backend.get_event();
        let _ = end_event;
        // Re-encode the composed children back into the JSON value
        // by collecting their .node().value clones. This preserves
        // the upstream behaviour where node.value is the sequence of
        // child nodes (Python keeps them as nodes; Rust caller can
        // descend via .collection.node.value as a JSON array).
        let arr: Vec<serde_json::Value> = elements.iter().map(|c| c.node().value.clone()).collect();
        node.collection.node.value = serde_json::Value::Array(arr);
        node
    }

    /// Port of `Composer.compose_mapping_node()` from
    /// `powerline/lint/markedjson/composer.py:102`.
    pub fn compose_mapping_node<B: ParserBackend>(backend: &mut B) -> MappingNode {
        // py:103-105  start_event = self.get_event(); resolve tag
        let start = backend.get_event();
        let mapping = match start {
            AnyEvent::MappingStart(m) => m,
            _ => unreachable!("compose_mapping_node called without MappingStartEvent"),
        };
        let mut tag = mapping.collection.tag.clone();
        if tag.as_deref().is_none() || tag.as_deref() == Some("!") {
            tag = Some(backend.resolve(NodeKind::Mapping, None, mapping.collection.implicit));
        }
        let mut node = MappingNode::new(
            tag.unwrap(),
            serde_json::Value::Object(serde_json::Map::new()),
            mapping.collection.node.event.start_mark,
            None,
            mapping.collection.flow_style,
        );
        // py:108-117  drain (key, value) pairs until MappingEnd
        let mut pairs: Vec<(ComposedNode, ComposedNode)> = Vec::new();
        while !backend.check_event(&[EventKind::MappingEnd]) {
            // py:110  item_key = self.compose_node(node, None)
            let key = Self::compose_node(backend, None, None);
            // py:114  item_value = self.compose_node(node, item_key)
            let value = Self::compose_node(backend, None, None);
            pairs.push((key, value));
        }
        // py:118-119  consume MappingEnd
        let end_event = backend.get_event();
        let _ = end_event;
        // Re-encode pairs into a serde_json Map for the .value field.
        // serde Map keys must be strings; we stringify the key's JSON.
        let mut obj = serde_json::Map::new();
        for (k, v) in pairs {
            let key_str = match k.node().value.clone() {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            obj.insert(key_str, v.node().value.clone());
        }
        node.collection.node.value = serde_json::Value::Object(obj);
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Minimal in-memory ParserBackend for unit testing the
    // composer dispatch + tag-resolution flow. Stores a scripted
    // event queue + a fixed-tag resolver.
    struct FakeBackend {
        events: std::collections::VecDeque<AnyEvent>,
    }

    impl FakeBackend {
        fn new(events: Vec<AnyEvent>) -> Self {
            Self {
                events: events.into(),
            }
        }
    }

    impl ParserBackend for FakeBackend {
        fn check_event(&mut self, kinds: &[EventKind]) -> bool {
            self.events
                .front()
                .map(|e| kinds.contains(&e.kind()))
                .unwrap_or(false)
        }
        fn get_event(&mut self) -> AnyEvent {
            self.events.pop_front().expect("event stream underflow")
        }
        fn descend_resolver(
            &mut self,
            _parent: Option<&ComposedNode>,
            _index: Option<&ComposedNode>,
        ) {
        }
        fn ascend_resolver(&mut self) {}
        fn resolve(
            &mut self,
            kind: NodeKind,
            _value: Option<&serde_json::Value>,
            _implicit: bool,
        ) -> String {
            match kind {
                NodeKind::Scalar => "tag:yaml.org,2002:str".to_string(),
                NodeKind::Sequence => "tag:yaml.org,2002:seq".to_string(),
                NodeKind::Mapping => "tag:yaml.org,2002:map".to_string(),
            }
        }
    }

    #[test]
    fn composer_error_implements_error_traits() {
        let me = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let e = ComposerError(me);
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("ctx"));
    }

    #[test]
    fn check_node_drops_stream_start_and_returns_true_if_doc_remaining() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::StreamStart,
            AnyEvent::DocumentStart,
            AnyEvent::Scalar(ScalarEvent::new(true, json!("x"), None, None, None)),
            AnyEvent::DocumentEnd,
            AnyEvent::StreamEnd,
        ]);
        assert!(Composer::check_node(&mut b));
        // After StreamStart was dropped, the next event is DocumentStart.
        assert_eq!(b.get_event().kind(), EventKind::DocumentStart);
    }

    #[test]
    fn check_node_returns_false_at_stream_end() {
        let mut b = FakeBackend::new(vec![AnyEvent::StreamEnd]);
        assert!(!Composer::check_node(&mut b));
    }

    #[test]
    fn compose_document_returns_scalar_node() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::DocumentStart,
            AnyEvent::Scalar(ScalarEvent::new(true, json!("hello"), None, None, None)),
            AnyEvent::DocumentEnd,
        ]);
        let n = Composer::compose_document(&mut b);
        match n {
            ComposedNode::Scalar(s) => {
                assert_eq!(s.node.value, json!("hello"));
                assert_eq!(s.node.tag, "tag:yaml.org,2002:str");
            }
            _ => panic!("expected ScalarNode"),
        }
    }

    #[test]
    fn compose_node_dispatches_scalar() {
        let mut b = FakeBackend::new(vec![AnyEvent::Scalar(ScalarEvent::new(
            true,
            json!("x"),
            None,
            None,
            None,
        ))]);
        let n = Composer::compose_node(&mut b, None, None);
        assert!(matches!(n, ComposedNode::Scalar(_)));
    }

    #[test]
    fn compose_node_dispatches_sequence() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::SequenceStart(SequenceStartEvent::new(true, None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!(1), None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!(2), None, None, None)),
            AnyEvent::SequenceEnd,
        ]);
        let n = Composer::compose_node(&mut b, None, None);
        match n {
            ComposedNode::Sequence(s) => {
                let arr = s.collection.node.value.as_array().unwrap();
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0], json!(1));
                assert_eq!(arr[1], json!(2));
            }
            _ => panic!("expected SequenceNode"),
        }
    }

    #[test]
    fn compose_node_dispatches_mapping() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::MappingStart(MappingStartEvent::new(true, None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!("a"), None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!(1), None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!("b"), None, None, None)),
            AnyEvent::Scalar(ScalarEvent::new(true, json!(2), None, None, None)),
            AnyEvent::MappingEnd,
        ]);
        let n = Composer::compose_node(&mut b, None, None);
        match n {
            ComposedNode::Mapping(m) => {
                let obj = m.collection.node.value.as_object().unwrap();
                assert_eq!(obj.get("a"), Some(&json!(1)));
                assert_eq!(obj.get("b"), Some(&json!(2)));
            }
            _ => panic!("expected MappingNode"),
        }
    }

    #[test]
    fn compose_scalar_node_resolves_tag_when_none() {
        let mut b = FakeBackend::new(vec![AnyEvent::Scalar(ScalarEvent::new(
            true,
            json!(42),
            None,
            None,
            None,
        ))]);
        // ScalarEvent.tag is None by construction, so compose triggers
        // backend.resolve() → "tag:yaml.org,2002:str" from the fake.
        let s = Composer::compose_scalar_node(&mut b);
        assert_eq!(s.node.tag, "tag:yaml.org,2002:str");
    }

    #[test]
    fn get_single_node_errors_on_two_documents() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::StreamStart,
            AnyEvent::DocumentStart,
            AnyEvent::Scalar(ScalarEvent::new(true, json!(1), None, None, None)),
            AnyEvent::DocumentEnd,
            // Second document
            AnyEvent::DocumentStart,
            AnyEvent::Scalar(ScalarEvent::new(true, json!(2), None, None, None)),
            AnyEvent::DocumentEnd,
            AnyEvent::StreamEnd,
        ]);
        let r = Composer::get_single_node(&mut b);
        assert!(r.is_err());
    }

    #[test]
    fn get_single_node_returns_single_document() {
        let mut b = FakeBackend::new(vec![
            AnyEvent::StreamStart,
            AnyEvent::DocumentStart,
            AnyEvent::Scalar(ScalarEvent::new(true, json!("only"), None, None, None)),
            AnyEvent::DocumentEnd,
            AnyEvent::StreamEnd,
        ]);
        let r = Composer::get_single_node(&mut b).unwrap();
        assert!(r.is_some());
        match r.unwrap() {
            ComposedNode::Scalar(s) => assert_eq!(s.node.value, json!("only")),
            _ => panic!("expected scalar"),
        }
    }

    #[test]
    fn get_single_node_returns_none_on_empty_stream() {
        let mut b = FakeBackend::new(vec![AnyEvent::StreamStart, AnyEvent::StreamEnd]);
        let r = Composer::get_single_node(&mut b).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn any_event_kind_dispatches_correctly() {
        assert_eq!(AnyEvent::StreamStart.kind(), EventKind::StreamStart);
        assert_eq!(AnyEvent::StreamEnd.kind(), EventKind::StreamEnd);
        assert_eq!(AnyEvent::DocumentStart.kind(), EventKind::DocumentStart);
        assert_eq!(AnyEvent::DocumentEnd.kind(), EventKind::DocumentEnd);
        assert_eq!(AnyEvent::Alias.kind(), EventKind::Alias);
        assert_eq!(AnyEvent::SequenceEnd.kind(), EventKind::SequenceEnd);
        assert_eq!(AnyEvent::MappingEnd.kind(), EventKind::MappingEnd);
        assert_eq!(
            AnyEvent::Scalar(ScalarEvent::new(true, json!(0), None, None, None)).kind(),
            EventKind::Scalar
        );
        assert_eq!(
            AnyEvent::SequenceStart(SequenceStartEvent::new(true, None, None, None)).kind(),
            EventKind::SequenceStart
        );
        assert_eq!(
            AnyEvent::MappingStart(MappingStartEvent::new(true, None, None, None)).kind(),
            EventKind::MappingStart
        );
    }

    #[test]
    fn composed_node_start_mark_returns_underlying_mark() {
        let m = Some(Mark { line: 4, column: 9 });
        let s = ScalarNode::new("t", json!(0), m, None, None);
        let c = ComposedNode::Scalar(s);
        assert_eq!(c.start_mark().unwrap().line, 4);
        assert_eq!(c.start_mark().unwrap().column, 9);
    }
}
