// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/__init__.py`.
//!
//! Exports the `Segment` base class + `with_docstring` decorator used
//! by class-based segments (`segments/common/vcs.py`,
//! `segments/common/players.py`, etc.). Function-based segments don't
//! inherit from `Segment` — they're plain `def`s decorated with
//! `@requires_segment_info` / `@requires_filesystem_watcher`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// from pkgutil import extend_path                  // py:6
// from types import MethodType                     // py:7

// py:10  __path__ = extend_path(__path__, __name__)
// (Namespace-package mechanism — handled statically in Rust via
// the `pub mod` declarations at the bottom of this file.)

pub mod common;
pub mod i3wm;
pub mod ipython;
pub mod pdb;
pub mod shell;
pub mod tmux;
pub mod vim;

/// Port of `class Segment` from `powerline/segments/__init__.py:13`.
///
/// Base class for any segment that is not a function.
///
/// Required for `powerline.lint.inspect` to work properly: it defines
/// methods for omitting existing or adding new arguments.
///
/// The Python implementation has three methods:
/// - `argspecobjs()` — yields `('__call__', self.__call__)`
/// - `omitted_args(name, method)` — list args to drop from inspection
/// - `additional_args()` (static) — extra args to inject
///
/// All three are introspection helpers used by the linter to figure
/// out what arguments a class-based segment accepts. powerliners's
/// linter is unported (Phase 5), so the Rust port carries the trait
/// shape with default no-op implementations; class-based segments
/// override as needed.
pub trait Segment {
    // py:13  class Segment(object):
    // py:14-24  docstring
    // py:25  if sys.version_info < (3, 4):
    // py:26  def argspecobjs(self):
    // py:27  yield '__call__', self.__call__
    // py:28  else:
    // py:29  def argspecobjs(self):
    // py:30  yield '__call__', self

    /// Port of `Segment.omitted_args()` from
    /// `powerline/segments/__init__.py:40`.
    ///
    /// Returns a tuple with indexes of omitted arguments.
    fn omitted_args(&self, _name: &str) -> Vec<usize> {
        // py:39  def omitted_args(self, name, method):
        // py:40-48  docstring
        // py:49  if isinstance(self.__call__, MethodType):
        // py:50  return (0,)
        // py:51  else:
        // py:52  return ()
        Vec::new()
    }

    /// Port of `Segment.additional_args()` from
    /// `powerline/segments/__init__.py:53`.
    ///
    /// Returns a list of `(additional argument name[, default value])` tuples.
    fn additional_args(&self) -> Vec<(String, Option<serde_json::Value>)> {
        // py:54  @staticmethod
        // py:55  def additional_args():
        // py:56-57  docstring
        // py:58  return ()
        Vec::new()
    }
}

/// Port of `with_docstring()` from `powerline/segments/__init__.py:60`.
///
/// Python: `instance.__doc__ = doc; return instance`
///
/// Used by `segments/common/env.py` etc. to replace the docstring of a
/// class-based segment instance (since the class docstring would
/// otherwise apply to every instance). Rust has no runtime
/// `__doc__` attribute; doc-strings are baked into the binary at
/// compile time. The Rust port is therefore an identity passthrough
/// preserved for upstream call-site shape.
pub fn with_docstring<T>(instance: T, _doc: &str) -> T {
    // py:61  instance.__doc__ = doc
    // py:62  return instance
    instance
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default `Segment` impl returns empty omitted_args + additional_args.
    #[test]
    fn segment_defaults_are_empty() {
        struct DummySegment;
        impl Segment for DummySegment {}

        let s = DummySegment;
        assert!(s.omitted_args("__call__").is_empty());
        assert!(s.additional_args().is_empty());
    }

    /// `with_docstring` is an identity passthrough.
    #[test]
    fn with_docstring_passes_through() {
        let x = 42;
        let y = with_docstring(x, "any docstring");
        assert_eq!(x, y);
    }
}
