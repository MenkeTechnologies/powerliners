// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/nodes.py`.
//!
//! YAML-style node tree used by the markedjson parser. Each node
//! carries a tag (`!str`, `!seq`, `!map`), a value, and start/end
//! Mark positions for error reporting.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use serde_json::Value;

/// Mark — a `(line, column, name, buffer, pointer)` shape used by the
/// markedjson parser. Until the full parser ports, only the
/// human-visible `(line, column)` are modelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mark {
    pub line: usize,
    pub column: usize,
}

/// Port of `class Node` from `powerline/lint/markedjson/nodes.py:5`.
///
/// Base class for all marked-JSON nodes. The Rust port wraps `value`
/// in `serde_json::Value` since downstream consumers operate on JSON
/// shapes; the `tag` field carries the type-tag string.
#[derive(Debug, Clone)]
pub struct Node {
    pub tag: String,
    pub value: Value,
    pub start_mark: Option<Mark>,
    pub end_mark: Option<Mark>,
}

impl Node {
    /// Port of `Node.__init__()` from
    /// `powerline/lint/markedjson/nodes.py:6`.
    pub fn new(
        tag: impl Into<String>,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
    ) -> Self {
        Self {
            tag: tag.into(),
            value,
            start_mark,
            end_mark,
        }
    }
}

impl std::fmt::Display for Node {
    /// Port of `Node.__repr__()` from
    /// `powerline/lint/markedjson/nodes.py:12`.
    ///
    /// Python: `'%s(tag=%r, value=%s)' % (cls_name, tag, repr(value))`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node(tag={:?}, value={:?})", self.tag, self.value)
    }
}

/// Port of `class ScalarNode(Node)` from
/// `powerline/lint/markedjson/nodes.py:31`.
///
/// Scalar-valued marked node (string/int/float/bool/null). Carries
/// the original literal `style` character (`'` / `"` / `>` / `|`) so
/// the linter can echo the right quote convention.
#[derive(Debug, Clone)]
pub struct ScalarNode {
    pub node: Node,
    pub style: Option<char>,
}

impl ScalarNode {
    /// Python class attribute: `id = 'scalar'` — py:32
    pub const ID: &'static str = "scalar";

    /// Port of `ScalarNode.__init__()` from
    /// `powerline/lint/markedjson/nodes.py:34`.
    pub fn new(
        tag: impl Into<String>,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        style: Option<char>,
    ) -> Self {
        Self {
            node: Node::new(tag, value, start_mark, end_mark),
            style,
        }
    }
}

/// Port of `class CollectionNode(Node)` from
/// `powerline/lint/markedjson/nodes.py:41`.
///
/// Base for sequence / mapping nodes. Carries the `flow_style` flag
/// (`true` for `[...]` / `{...}` inline syntax, `false` for the
/// block form, `None` for unspecified).
#[derive(Debug, Clone)]
pub struct CollectionNode {
    pub node: Node,
    pub flow_style: Option<bool>,
}

impl CollectionNode {
    /// Port of `CollectionNode.__init__()` from
    /// `powerline/lint/markedjson/nodes.py:42`.
    pub fn new(
        tag: impl Into<String>,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            node: Node::new(tag, value, start_mark, end_mark),
            flow_style,
        }
    }
}

/// Port of `class SequenceNode(CollectionNode)` from
/// `powerline/lint/markedjson/nodes.py:49`.
///
/// `id = 'sequence'`.
#[derive(Debug, Clone)]
pub struct SequenceNode {
    pub collection: CollectionNode,
}

impl SequenceNode {
    pub const ID: &'static str = "sequence";

    pub fn new(
        tag: impl Into<String>,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            collection: CollectionNode::new(tag, value, start_mark, end_mark, flow_style),
        }
    }
}

/// Port of `class MappingNode(CollectionNode)` from
/// `powerline/lint/markedjson/nodes.py:53`.
///
/// `id = 'mapping'`.
#[derive(Debug, Clone)]
pub struct MappingNode {
    pub collection: CollectionNode,
}

impl MappingNode {
    pub const ID: &'static str = "mapping";

    pub fn new(
        tag: impl Into<String>,
        value: Value,
        start_mark: Option<Mark>,
        end_mark: Option<Mark>,
        flow_style: Option<bool>,
    ) -> Self {
        Self {
            collection: CollectionNode::new(tag, value, start_mark, end_mark, flow_style),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn node_carries_tag_value_and_marks() {
        let start = Mark { line: 1, column: 0 };
        let end = Mark { line: 1, column: 5 };
        let n = Node::new(
            "!str",
            json!("hello"),
            Some(start.clone()),
            Some(end.clone()),
        );
        assert_eq!(n.tag, "!str");
        assert_eq!(n.value, "hello");
        assert_eq!(n.start_mark, Some(start));
        assert_eq!(n.end_mark, Some(end));
    }

    #[test]
    fn scalar_node_id_matches_upstream() {
        assert_eq!(ScalarNode::ID, "scalar");
    }

    #[test]
    fn scalar_node_carries_style() {
        let s = ScalarNode::new("!str", json!("x"), None, None, Some('"'));
        assert_eq!(s.style, Some('"'));
    }

    #[test]
    fn sequence_node_id_matches_upstream() {
        assert_eq!(SequenceNode::ID, "sequence");
    }

    #[test]
    fn mapping_node_id_matches_upstream() {
        assert_eq!(MappingNode::ID, "mapping");
    }

    #[test]
    fn collection_node_carries_flow_style() {
        let c = CollectionNode::new("!seq", json!([]), None, None, Some(true));
        assert_eq!(c.flow_style, Some(true));
    }

    #[test]
    fn node_display_matches_upstream_repr_shape() {
        let n = Node::new("!str", json!("hello"), None, None);
        let s = format!("{}", n);
        assert!(s.starts_with("Node(tag="));
        assert!(s.contains("value="));
    }
}
