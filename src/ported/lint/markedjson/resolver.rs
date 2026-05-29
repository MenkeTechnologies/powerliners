// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/markedjson/resolver.py`.
//!
//! YAML-style implicit-tag resolver used by the lint-time JSON
//! loader. Maps scalar values to one of four YAML tags
//! (`bool`/`float`/`int`/`null`) via a per-first-character registry
//! of (tag, regex) pairs, with fallback to the default scalar tag.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import re                                        // py:4
// from powerline.lint.markedjson.error import MarkedError                                 // py:6
// from powerline.lint.markedjson import nodes      // py:7

use crate::ported::lint::markedjson::error::MarkedError;
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Port of `class ResolverError(MarkedError)` from
/// `powerline/lint/markedjson/resolver.py:10`.
#[derive(Debug, Clone)]
pub struct ResolverError(pub MarkedError);

impl std::fmt::Display for ResolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::error::Error for ResolverError {}

/// Port of `class BaseResolver.DEFAULT_SCALAR_TAG`.
pub const DEFAULT_SCALAR_TAG: &str = "tag:yaml.org,2002:str";
/// Port of `class BaseResolver.DEFAULT_SEQUENCE_TAG`.
pub const DEFAULT_SEQUENCE_TAG: &str = "tag:yaml.org,2002:seq";
/// Port of `class BaseResolver.DEFAULT_MAPPING_TAG`.
pub const DEFAULT_MAPPING_TAG: &str = "tag:yaml.org,2002:map";

/// Which kind of node is being resolved.
///
/// Port of the type-arg `kind` in `BaseResolver.resolve()` (py:81-)
/// which receives one of `nodes.ScalarNode` / `nodes.SequenceNode` /
/// `nodes.MappingNode`. Rust represents the discrimination as an enum
/// since the original passes class references.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Scalar,
    Sequence,
    Mapping,
}

/// Port of `class BaseResolver` from
/// `powerline/lint/markedjson/resolver.py:14`.
///
/// The Python class tracks `yaml_implicit_resolvers` as a class-level
/// dict + `resolver_exact_paths` / `resolver_prefix_paths` on the
/// instance. Rust port stores the resolver registry as a per-instance
/// table populated via `register_default_resolvers()` (the Python
/// module-level `add_implicit_resolver(...)` calls at py:108-127).
pub struct BaseResolver {
    /// Python: `cls.yaml_implicit_resolvers` — keyed by first char
    /// (or '\0' for the empty-match bucket). Each entry is the list of
    /// (tag, regex) pairs to try.
    implicit_resolvers: HashMap<char, Vec<(String, Regex)>>,
}

impl Default for BaseResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseResolver {
    /// Port of `BaseResolver.__init__()` from
    /// `powerline/lint/markedjson/resolver.py:23`.
    ///
    /// Also registers the four module-level `add_implicit_resolver()`
    /// calls (py:108-127) on construction since Rust doesn't have a
    /// module-execution hook.
    pub fn new() -> Self {
        // py:14  class BaseResolver:
        // py:15  DEFAULT_SCALAR_TAG = 'tag:yaml.org,2002:str'
        // py:16  DEFAULT_SEQUENCE_TAG = 'tag:yaml.org,2002:seq'
        // py:17  DEFAULT_MAPPING_TAG = 'tag:yaml.org,2002:map'
        // py:19  yaml_implicit_resolvers = {}
        // py:20  yaml_path_resolvers = {}
        // py:22  def __init__(self):
        // py:23  self.resolver_exact_paths = []
        // py:24  self.resolver_prefix_paths = []
        let mut r = Self {
            implicit_resolvers: HashMap::new(),
        };
        r.register_defaults();
        r
    }

    /// Port of `BaseResolver.add_implicit_resolver()` from
    /// `powerline/lint/markedjson/resolver.py:28`.
    ///
    /// Python: registers `(tag, regexp)` under each char in `first`
    /// (or under `None` if first is None — bucket '\0' here).
    pub fn add_implicit_resolver(&mut self, tag: &str, regexp: Regex, first: Option<&[char]>) {
        // py:26  @classmethod
        // py:27  def add_implicit_resolver(cls, tag, regexp, first):
        // py:28  if 'yaml_implicit_resolvers' not in cls.__dict__:
        // py:29  cls.yaml_implicit_resolvers = cls.yaml_implicit_resolvers.copy()
        // py:30  if first is None:
        // py:31  first = [None]
        let chars: Vec<char> = match first {
            Some(s) => s.to_vec(),
            None => vec!['\0'],
        };
        // py:32  for ch in first:
        // py:33  cls.yaml_implicit_resolvers.setdefault(ch, []).append((tag, regexp))
        for ch in chars {
            self.implicit_resolvers
                .entry(ch)
                .or_default()
                .push((tag.to_string(), regexp.clone()));
        }
    }

    /// Registers the four module-level `Resolver.add_implicit_resolver`
    /// calls from `powerline/lint/markedjson/resolver.py:108-127`.
    fn register_defaults(&mut self) {
        // py:108-111  bool resolver — first chars yYnNtTfFoO
        self.add_implicit_resolver(
            "tag:yaml.org,2002:bool",
            Regex::new(r"^(?:true|false)$").unwrap(),
            Some(&['y', 'Y', 'n', 'N', 't', 'T', 'f', 'F', 'o', 'O']),
        );
        // py:113-116  float resolver. Python uses a lookahead
        // `(?=[.eE])` to require a decimal point or exponent; the
        // Rust `regex` crate doesn't support lookahead, so the
        // equivalent without it requires at least one of the
        // optional groups to be present: either `.\d+(?:[eE]...)?`
        // or `[eE][-+]?\d+`.
        self.add_implicit_resolver(
            "tag:yaml.org,2002:float",
            Regex::new(r"^-?(?:0|[1-9]\d*)(?:\.\d+(?:[eE][-+]?\d+)?|[eE][-+]?\d+)$").unwrap(),
            Some(&['-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9']),
        );
        // py:118-121  int resolver
        self.add_implicit_resolver(
            "tag:yaml.org,2002:int",
            Regex::new(r"^(?:0|-?[1-9]\d*)$").unwrap(),
            Some(&['-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9']),
        );
        // py:123-126  null resolver
        self.add_implicit_resolver(
            "tag:yaml.org,2002:null",
            Regex::new(r"^null$").unwrap(),
            Some(&['n']),
        );
    }

    /// Port of `BaseResolver.resolve()` from
    /// `powerline/lint/markedjson/resolver.py:81`.
    ///
    /// Returns the implicit YAML tag for the given `(kind, value)`
    /// pair. `implicit_first` corresponds to the `implicit[0]` flag
    /// from Python (whether implicit resolution is enabled for plain
    /// scalars). On no-match, returns the default scalar tag and emits
    /// nothing (the Python source's `self.echoerr(...)` call is
    /// stubbed since the EchoErr plumbing isn't wired here).
    pub fn resolve(&self, kind: NodeKind, value: &str, implicit_first: bool) -> String {
        // py:84  def resolve(self, kind, value, implicit, mark=None):
        // py:85  if kind is nodes.ScalarNode and implicit[0]:
        if matches!(kind, NodeKind::Scalar) && implicit_first {
            // py:86  if value == '':
            // py:87  resolvers = self.yaml_implicit_resolvers.get('', [])
            // py:88  else:
            // py:89  resolvers = self.yaml_implicit_resolvers.get(value[0], [])
            let bucket_key: char = if value.is_empty() {
                '\0'
            } else {
                value.chars().next().unwrap()
            };
            // py:90  resolvers += self.yaml_implicit_resolvers.get(None, [])
            let mut resolvers: Vec<&(String, Regex)> = Vec::new();
            if let Some(rs) = self.implicit_resolvers.get(&bucket_key) {
                resolvers.extend(rs.iter());
            }
            if let Some(rs) = self.implicit_resolvers.get(&'\0') {
                if bucket_key != '\0' {
                    resolvers.extend(rs.iter());
                }
            }
            // py:91  for tag, regexp in resolvers:
            // py:92  if regexp.match(value):
            // py:93  return tag
            for (tag, regex) in resolvers {
                if regex.is_match(value) {
                    return tag.clone();
                }
            }
            // py:94  else:
            // py:95  self.echoerr(
            // py:96  'While resolving plain scalar', None,
            // py:97  'expected floating-point value, integer, null or boolean, but got %r' % value,
            // py:98  mark
            // py:99  )
            // py:100  return self.DEFAULT_SCALAR_TAG
            return DEFAULT_SCALAR_TAG.to_string();
        }
        // py:101  if kind is nodes.ScalarNode:
        // py:102  return self.DEFAULT_SCALAR_TAG
        // py:103  elif kind is nodes.SequenceNode:
        // py:104  return self.DEFAULT_SEQUENCE_TAG
        // py:105  elif kind is nodes.MappingNode:
        // py:106  return self.DEFAULT_MAPPING_TAG
        match kind {
            NodeKind::Scalar => DEFAULT_SCALAR_TAG.to_string(),
            NodeKind::Sequence => DEFAULT_SEQUENCE_TAG.to_string(),
            NodeKind::Mapping => DEFAULT_MAPPING_TAG.to_string(),
        }
    }

    /// Port of `BaseResolver.descend_resolver()` from
    /// `powerline/lint/markedjson/resolver.py:35`.
    ///
    /// **Status:** stub. The Rust port surfaces the call shape;
    /// `yaml_path_resolvers` is empty in this codebase so the body
    /// short-circuits per py:36-37.
    pub fn descend_resolver(&self) {
        // py:35  def descend_resolver(self, current_node, current_index):
        // py:36  if not self.yaml_path_resolvers:
        // py:37  return
        // py:38  exact_paths = {}
        // py:39  prefix_paths = []
        // py:40  if current_node:
        // py:41  depth = len(self.resolver_prefix_paths)
        // py:42  for path, kind in self.resolver_prefix_paths[-1]:
        // py:43  if self.check_resolver_prefix(depth, path, kind, current_node, current_index):
        // py:44  if len(path) > depth:
        // py:45  prefix_paths.append((path, kind))
        // py:46  else:
        // py:47  exact_paths[kind] = self.yaml_path_resolvers[path, kind]
        // py:48  else:
        // py:49  for path, kind in self.yaml_path_resolvers:
        // py:50  if not path:
        // py:51  exact_paths[kind] = self.yaml_path_resolvers[path, kind]
        // py:52  else:
        // py:53  prefix_paths.append((path, kind))
        // py:54  self.resolver_exact_paths.append(exact_paths)
        // py:55  self.resolver_prefix_paths.append(prefix_paths)
    }

    /// Port of `BaseResolver.ascend_resolver()` from
    /// `powerline/lint/markedjson/resolver.py:57`.
    pub fn ascend_resolver(&self) {
        // py:57  def ascend_resolver(self):
        // py:58  if not self.yaml_path_resolvers:
        // py:59  return
        // py:60  self.resolver_exact_paths.pop()
        // py:61  self.resolver_prefix_paths.pop()
    }

    /// Port of `BaseResolver.check_resolver_prefix()` from
    /// `powerline/lint/markedjson/resolver.py:63`.
    pub fn check_resolver_prefix(&self) -> bool {
        // py:63  def check_resolver_prefix(self, depth, path, kind, current_node, current_index):
        // py:64  node_check, index_check = path[depth - 1]
        // py:65  if isinstance(node_check, str):
        // py:66  if current_node.tag != node_check:
        // py:67  return
        // py:68  elif node_check is not None:
        // py:69  if not isinstance(current_node, node_check):
        // py:70  return
        // py:71  if index_check is True and current_index is not None:
        // py:72  return
        // py:73  if ((index_check is False or index_check is None)
        // py:74  and current_index is None):
        // py:75  return
        // py:76  if isinstance(index_check, str):
        // py:77  if not (isinstance(current_index, nodes.ScalarNode) and index_check == current_index.value):
        // py:78  return
        // py:79  elif isinstance(index_check, int) and not isinstance(index_check, bool):
        // py:80  if index_check != current_index:
        // py:81  return
        // py:82  return True
        true
    }
}

// `descend_resolver` / `ascend_resolver` / `check_resolver_prefix`
// (py:38-79) port separately — they implement the path-resolver
// feature for YAML !tag overrides, which the JSON-only lint loader
// never populates (`yaml_path_resolvers = {}` is always empty in this
// codebase). Deferred.

/// Port of `class Resolver(BaseResolver)` from
/// `powerline/lint/markedjson/resolver.py:106`.
///
/// Python defines `Resolver` as a subclass with no overrides; the four
/// `add_implicit_resolver` calls at py:108-127 mutate the class-level
/// `yaml_implicit_resolvers` dict. Rust port returns a `BaseResolver`
/// with those registrations applied by `register_defaults()`.
pub fn resolver_singleton() -> &'static BaseResolver {
    static R: OnceLock<BaseResolver> = OnceLock::new();
    R.get_or_init(BaseResolver::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tags_match_upstream() {
        assert_eq!(DEFAULT_SCALAR_TAG, "tag:yaml.org,2002:str");
        assert_eq!(DEFAULT_SEQUENCE_TAG, "tag:yaml.org,2002:seq");
        assert_eq!(DEFAULT_MAPPING_TAG, "tag:yaml.org,2002:map");
    }

    #[test]
    fn resolve_bool_true() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "true", true),
            "tag:yaml.org,2002:bool"
        );
    }

    #[test]
    fn resolve_bool_false() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "false", true),
            "tag:yaml.org,2002:bool"
        );
    }

    #[test]
    fn resolve_int() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "42", true),
            "tag:yaml.org,2002:int"
        );
        assert_eq!(
            r.resolve(NodeKind::Scalar, "-7", true),
            "tag:yaml.org,2002:int"
        );
        assert_eq!(
            r.resolve(NodeKind::Scalar, "0", true),
            "tag:yaml.org,2002:int"
        );
    }

    #[test]
    fn resolve_float() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "3.14", true),
            "tag:yaml.org,2002:float"
        );
        assert_eq!(
            r.resolve(NodeKind::Scalar, "1e10", true),
            "tag:yaml.org,2002:float"
        );
        assert_eq!(
            r.resolve(NodeKind::Scalar, "-2.5e-3", true),
            "tag:yaml.org,2002:float"
        );
    }

    #[test]
    fn resolve_null() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "null", true),
            "tag:yaml.org,2002:null"
        );
    }

    #[test]
    fn resolve_unknown_scalar_falls_back_to_string() {
        let r = BaseResolver::new();
        // py:93-99  no implicit match → DEFAULT_SCALAR_TAG
        assert_eq!(
            r.resolve(NodeKind::Scalar, "hello", true),
            DEFAULT_SCALAR_TAG
        );
        // 'h' isn't in any first-char bucket, hits the no-match path.
    }

    #[test]
    fn resolve_non_implicit_scalar_returns_str() {
        let r = BaseResolver::new();
        // py:100-101  implicit_first=false path
        assert_eq!(
            r.resolve(NodeKind::Scalar, "true", false),
            DEFAULT_SCALAR_TAG
        );
    }

    #[test]
    fn resolve_sequence_returns_seq_tag() {
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Sequence, "", false),
            DEFAULT_SEQUENCE_TAG
        );
    }

    #[test]
    fn resolve_mapping_returns_map_tag() {
        let r = BaseResolver::new();
        assert_eq!(r.resolve(NodeKind::Mapping, "", false), DEFAULT_MAPPING_TAG);
    }

    #[test]
    fn add_implicit_resolver_appends_to_first_chars() {
        let mut r = BaseResolver::new();
        r.add_implicit_resolver("tag:custom", Regex::new(r"^X$").unwrap(), Some(&['X']));
        assert_eq!(r.resolve(NodeKind::Scalar, "X", true), "tag:custom");
    }

    #[test]
    fn add_implicit_resolver_with_none_first_uses_null_bucket() {
        let mut r = BaseResolver::new();
        r.add_implicit_resolver("tag:catchall", Regex::new(r"^ZZZ$").unwrap(), None);
        // py:88  resolvers += yaml_implicit_resolvers.get(None, [])
        assert_eq!(r.resolve(NodeKind::Scalar, "ZZZ", true), "tag:catchall");
    }

    #[test]
    fn resolve_bool_with_lowercase_first_char_t() {
        // 't' is in the bool resolver first-char list; "true" matches.
        let r = BaseResolver::new();
        assert_eq!(
            r.resolve(NodeKind::Scalar, "true", true),
            "tag:yaml.org,2002:bool"
        );
    }

    #[test]
    fn resolver_singleton_caches_instance() {
        let r1 = resolver_singleton();
        let r2 = resolver_singleton();
        assert!(std::ptr::eq(r1, r2));
    }

    #[test]
    fn resolver_error_implements_error_traits() {
        let me = MarkedError::new(Some("ctx"), None, Some("prob"), None, None);
        let e = ResolverError(me);
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("ctx"));
    }

    #[test]
    fn float_regex_rejects_plain_int() {
        // py float regex requires lookahead (?=[.eE]) so plain "42" must
        // NOT resolve to float.
        let r = BaseResolver::new();
        let tag = r.resolve(NodeKind::Scalar, "42", true);
        assert_eq!(tag, "tag:yaml.org,2002:int");
        assert_ne!(tag, "tag:yaml.org,2002:float");
    }

    #[test]
    fn int_regex_rejects_leading_zero_multi_digit() {
        // py int regex: ^(?:0|-?[1-9]\d*)$ — "00" is not a valid int.
        let r = BaseResolver::new();
        assert_eq!(r.resolve(NodeKind::Scalar, "00", true), DEFAULT_SCALAR_TAG);
    }
}
