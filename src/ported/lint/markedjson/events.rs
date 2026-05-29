// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/events.py`.
//!
//! YAML parser event types emitted by `lint/markedjson/parser.py` and
//! consumed by `lint/markedjson/composer.py`. Each event carries
//! start/end Mark positions; specialised events add type-specific
//! fields (encoding, explicit, version, tags, implicit, value, style,
//! flow_style).

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use crate::ported::lint::markedjson::nodes::Mark;
use serde_json::Value;

/// Port of `class Event` from `powerline/lint/markedjson/events.py:6`.
///
/// Abstract base class for all parser events.
#[derive(Debug, Clone)]
pub struct Event {
    pub start_mark: Option<Mark>,
    pub end_mark: Option<Mark>,
}

impl Event {
    /// Port of `Event.__init__()` from
    /// `powerline/lint/markedjson/events.py:7`.
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            start_mark,
            end_mark,
        }
    }
}

/// Port of `class NodeEvent(Event)` from
/// `powerline/lint/markedjson/events.py:23`.
#[derive(Debug, Clone)]
pub struct NodeEvent {
    pub event: Event,
}

impl NodeEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
        }
    }
}

/// Port of `class CollectionStartEvent(NodeEvent)` from
/// `powerline/lint/markedjson/events.py:29`.
#[derive(Debug, Clone)]
pub struct CollectionStartEvent {
    pub node: NodeEvent,
    pub tag: Option<String>,
    pub implicit: bool,
    pub flow_style: Option<bool>,
}

impl CollectionStartEvent {
    /// Port of `CollectionStartEvent.__init__()` from
    /// `powerline/lint/markedjson/events.py:30`.
    pub fn new(
        implicit: bool,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            node: NodeEvent::new(start_mark, end_mark),
            tag: None, // py:31  self.tag = None
            implicit,
            flow_style,
        }
    }
}

/// Port of `class CollectionEndEvent(Event)` from
/// `powerline/lint/markedjson/events.py:37`.
#[derive(Debug, Clone)]
pub struct CollectionEndEvent {
    pub event: Event,
}

impl CollectionEndEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
        }
    }
}

/// Port of `class StreamStartEvent(Event)` from
/// `powerline/lint/markedjson/events.py:42`.
#[derive(Debug, Clone)]
pub struct StreamStartEvent {
    pub event: Event,
    pub encoding: Option<String>,
}

impl StreamStartEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>, encoding: Option<String>) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
            encoding,
        }
    }
}

/// Port of `class StreamEndEvent(Event)` from
/// `powerline/lint/markedjson/events.py:49`.
#[derive(Debug, Clone)]
pub struct StreamEndEvent {
    pub event: Event,
}

impl StreamEndEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
        }
    }
}

/// Port of `class DocumentStartEvent(Event)` from
/// `powerline/lint/markedjson/events.py:53`.
#[derive(Debug, Clone)]
pub struct DocumentStartEvent {
    pub event: Event,
    pub explicit: Option<bool>,
    pub version: Option<(i32, i32)>,
    pub tags: Option<Vec<(String, String)>>,
}

impl DocumentStartEvent {
    pub fn new(
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        explicit: Option<bool>,
        version: Option<(i32, i32)>,
        tags: Option<Vec<(String, String)>>,
    ) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
            explicit,
            version,
            tags,
        }
    }
}

/// Port of `class DocumentEndEvent(Event)` from
/// `powerline/lint/markedjson/events.py:61`.
#[derive(Debug, Clone)]
pub struct DocumentEndEvent {
    pub event: Event,
    pub explicit: Option<bool>,
}

impl DocumentEndEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>, explicit: Option<bool>) -> Self {
        Self {
            event: Event::new(start_mark, end_mark),
            explicit,
        }
    }
}

/// Port of `class AliasEvent(NodeEvent)` from
/// `powerline/lint/markedjson/events.py:68`.
#[derive(Debug, Clone)]
pub struct AliasEvent {
    pub node: NodeEvent,
}

impl AliasEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            node: NodeEvent::new(start_mark, end_mark),
        }
    }
}

/// Port of `class ScalarEvent(NodeEvent)` from
/// `powerline/lint/markedjson/events.py:72`.
#[derive(Debug, Clone)]
pub struct ScalarEvent {
    pub node: NodeEvent,
    pub tag: Option<String>,
    pub implicit: bool,
    pub value: Value,
    pub style: Option<char>,
}

impl ScalarEvent {
    pub fn new(
        implicit: bool,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        style: Option<char>,
    ) -> Self {
        Self {
            node: NodeEvent::new(start_mark, end_mark),
            tag: None, // py:74  self.tag = None
            implicit,
            value,
            style,
        }
    }
}

/// Port of `class SequenceStartEvent(CollectionStartEvent)` from
/// `powerline/lint/markedjson/events.py:81`.
#[derive(Debug, Clone)]
pub struct SequenceStartEvent {
    pub collection: CollectionStartEvent,
}

impl SequenceStartEvent {
    pub fn new(
        implicit: bool,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            collection: CollectionStartEvent::new(implicit, start_mark, end_mark, flow_style),
        }
    }
}

/// Port of `class SequenceEndEvent(CollectionEndEvent)` from
/// `powerline/lint/markedjson/events.py:85`.
#[derive(Debug, Clone)]
pub struct SequenceEndEvent {
    pub collection: CollectionEndEvent,
}

impl SequenceEndEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            collection: CollectionEndEvent::new(start_mark, end_mark),
        }
    }
}

/// Port of `class MappingStartEvent(CollectionStartEvent)` from
/// `powerline/lint/markedjson/events.py:89`.
#[derive(Debug, Clone)]
pub struct MappingStartEvent {
    pub collection: CollectionStartEvent,
}

impl MappingStartEvent {
    pub fn new(
        implicit: bool,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            collection: CollectionStartEvent::new(implicit, start_mark, end_mark, flow_style),
        }
    }
}

/// Port of `class MappingEndEvent(CollectionEndEvent)` from
/// `powerline/lint/markedjson/events.py:93`.
#[derive(Debug, Clone)]
pub struct MappingEndEvent {
    pub collection: CollectionEndEvent,
}

impl MappingEndEvent {
    pub fn new(start_mark: Option<Mark>, end_mark: Option<Mark>) -> Self {
        Self {
            collection: CollectionEndEvent::new(start_mark, end_mark),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn event_carries_marks() {
        let s = Mark { line: 1, column: 0 };
        let e = Mark { line: 1, column: 5 };
        let ev = Event::new(Some(s.clone()), Some(e.clone()));
        assert_eq!(ev.start_mark, Some(s));
        assert_eq!(ev.end_mark, Some(e));
    }

    #[test]
    fn stream_start_event_carries_encoding() {
        let e = StreamStartEvent::new(None, None, Some("utf-8".to_string()));
        assert_eq!(e.encoding.as_deref(), Some("utf-8"));
    }

    #[test]
    fn document_start_event_carries_version_and_tags() {
        let e = DocumentStartEvent::new(
            None,
            None,
            Some(true),
            Some((1, 2)),
            Some(vec![("!".into(), "tag:yaml.org,2002:".into())]),
        );
        assert_eq!(e.explicit, Some(true));
        assert_eq!(e.version, Some((1, 2)));
        assert_eq!(e.tags.unwrap().len(), 1);
    }

    #[test]
    fn collection_start_event_default_tag_is_none() {
        let e = CollectionStartEvent::new(true, None, None, Some(true));
        assert!(e.tag.is_none());
        assert!(e.implicit);
        assert_eq!(e.flow_style, Some(true));
    }

    #[test]
    fn scalar_event_default_tag_is_none() {
        let e = ScalarEvent::new(true, json!("x"), None, None, Some('"'));
        assert!(e.tag.is_none());
        assert!(e.implicit);
        assert_eq!(e.value, "x");
        assert_eq!(e.style, Some('"'));
    }

    #[test]
    fn sequence_mapping_inherit_collection_shape() {
        let s = SequenceStartEvent::new(true, None, None, Some(false));
        assert!(s.collection.implicit);
        assert_eq!(s.collection.flow_style, Some(false));

        let m = MappingStartEvent::new(false, None, None, None);
        assert!(!m.collection.implicit);
    }
}
