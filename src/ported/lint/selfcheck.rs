// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/selfcheck.py`.
//!
//! Recursive assertion that every value in a config tree carries a
//! `mark` attribute (from `lib/markedjson`) so the linter can attach
//! file:line information to error messages.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.lib.unicode import unicode                                                // py:4

use serde_json::Value;

/// Port of `havemarks()` from `powerline/lint/selfcheck.py:7`.
///
/// Python signature: `havemarks(*args, **kwargs)` with `origin` kwarg.
/// Walks each arg recursively (dicts and lists) and asserts via a
/// raised `AssertionError` that every encountered value has a `mark`
/// attribute.
///
/// The Rust port operates on `serde_json::Value` since that is the
/// in-memory shape of powerliners's loaded config; `mark` tracking
/// lives in a parallel mark-map carried alongside the value tree
/// (populated by `lib/markedjson` when ported). Until `markedjson` is
/// ported, this fn is a structural walker that records the *paths*
/// at which `mark` would need to be checked.
pub fn havemarks(args: &[&Value], origin: &str) -> Result<(), String> {
    for (i, v) in args.iter().enumerate() {
        // py:9
        // py:10  if not hasattr(v, 'mark'):
        // (No mark tracking yet — the mark check is a no-op until
        // MarkedValue lands. See PORT_PLAN.md Phase 5 for the lint port.)

        // py:12  if isinstance(v, dict):
        if let Value::Object(d) = v {
            for (key, val) in d {
                // py:13
                // py:14  havemarks(key, val, origin=(origin + '[' + unicode(i) + ']/' + unicode(key)))
                let new_origin = format!("{}[{}]/{}", origin, i, key);
                // Python passes `key` (a `str`) and `val` separately;
                // since Rust strings have no `mark` either, the key
                // walk is skipped — only `val` is recursed into.
                havemarks(&[val], &new_origin)?;
                let _ = key;
            }
        // py:15  elif isinstance(v, list):
        } else if let Value::Array(l) = v {
            // py:16  havemarks(*v, origin=(origin + '[' + unicode(i) + ']'))
            let new_origin = format!("{}[{}]", origin, i);
            let refs: Vec<&Value> = l.iter().collect();
            havemarks(&refs, &new_origin)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Walks a nested structure without error (mark-presence assertion
    /// is currently a no-op pending markedjson port).
    #[test]
    fn havemarks_walks_nested_structure() {
        let v = json!({
            "a": [1, 2, {"b": [3, 4]}],
            "c": "leaf"
        });
        assert!(havemarks(&[&v], "").is_ok());
    }

    /// Empty input is a trivial pass.
    #[test]
    fn havemarks_empty_input_ok() {
        assert!(havemarks(&[], "").is_ok());
    }
}
