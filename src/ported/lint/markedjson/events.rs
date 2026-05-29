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
        // py:5  # Abstract classes.
        // py:6  class Event(object):
        // py:7  def __init__(self, start_mark=None, end_mark=None):
        // py:8  self.start_mark = start_mark
        // py:9  self.end_mark = end_mark
        Self {
            start_mark,
            end_mark,
        }
    }

    /// Port of `Event.__repr__()` from
    /// `powerline/lint/markedjson/events.py:11`.
    ///
    /// Python builds `ClassName(implicit=..., value=...)` exposing
    /// only the `implicit` and `value` attributes if present.
    #[allow(non_snake_case)]
    pub fn __repr__(class_name: &str) -> String {
        // py:11  def __repr__(self):
        // py:12  attributes = [
        // py:13  key for key in ['implicit', 'value']
        // py:14  if hasattr(self, key)
        // py:15  ]
        // py:16  arguments = ', '.join([
        // py:17  '%s=%r' % (key, getattr(self, key))
        // py:18  for key in attributes
        // py:19  ])
        // py:20  return '%s(%s)' % (self.__class__.__name__, arguments)
        format!("{}()", class_name)
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
        // py:23  class NodeEvent(Event):
        // py:24  def __init__(self, start_mark=None, end_mark=None):
        // py:25  self.start_mark = start_mark
        // py:26  self.end_mark = end_mark
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
        // py:29  class CollectionStartEvent(NodeEvent):
        // py:30  def __init__(self, implicit, start_mark=None, end_mark=None, flow_style=None):
        // py:31  self.tag = None
        // py:32  self.implicit = implicit
        // py:33  self.start_mark = start_mark
        // py:34  self.end_mark = end_mark
        // py:35  self.flow_style = flow_style
        Self {
            node: NodeEvent::new(start_mark, end_mark),
            tag: None,
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
        // py:38  class CollectionEndEvent(Event):
        // py:39  pass
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
        // py:42  # Implementations.
        // py:43  class StreamStartEvent(Event):
        // py:44  def __init__(self, start_mark=None, end_mark=None, encoding=None):
        // py:45  self.start_mark = start_mark
        // py:46  self.end_mark = end_mark
        // py:47  self.encoding = encoding
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
        // py:50  class StreamEndEvent(Event):
        // py:51  pass
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
        // py:54  class DocumentStartEvent(Event):
        // py:55  def __init__(self, start_mark=None, end_mark=None, explicit=None, version=None, tags=None):
        // py:56  self.start_mark = start_mark
        // py:57  self.end_mark = end_mark
        // py:58  self.explicit = explicit
        // py:59  self.version = version
        // py:60  self.tags = tags
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
        // py:63  class DocumentEndEvent(Event):
        // py:64  def __init__(self, start_mark=None, end_mark=None, explicit=None):
        // py:65  self.start_mark = start_mark
        // py:66  self.end_mark = end_mark
        // py:67  self.explicit = explicit
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
        // py:70  class AliasEvent(NodeEvent):
        // py:71  pass
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
        // py:74  class ScalarEvent(NodeEvent):
        // py:75  def __init__(self, implicit, value, start_mark=None, end_mark=None, style=None):
        // py:76  self.tag = None
        // py:77  self.implicit = implicit
        // py:78  self.value = value
        // py:79  self.start_mark = start_mark
        // py:80  self.end_mark = end_mark
        // py:81  self.style = style
        Self {
            node: NodeEvent::new(start_mark, end_mark),
            tag: None,
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
        // py:84  class SequenceStartEvent(CollectionStartEvent):
        // py:85  pass
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
        // py:88  class SequenceEndEvent(CollectionEndEvent):
        // py:89  pass
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
        // py:92  class MappingStartEvent(CollectionStartEvent):
        // py:93  pass
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
        // py:96  class MappingEndEvent(CollectionEndEvent):
        // py:97  pass
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
