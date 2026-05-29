// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/dict.py`.
//!
//! Recursive dict-merge primitives used by powerline-status's config
//! loader: each user / system / per-shell / per-theme JSON file is
//! merged into a single in-memory config tree via `mergedicts`. The
//! sentinel `REMOVE_THIS_KEY` lets a downstream config explicitly
//! delete a key set by an upstream layer.
//!
//! Dictionary type: powerline-status loads JSON/YAML config and works
//! against `dict[str, Any]`. The Rust port operates on
//! `serde_json::Map<String, serde_json::Value>` since powerliners's
//! config loader will produce these.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use serde_json::{Map, Value};

/// Port of module-level binding `REMOVE_THIS_KEY` from
/// `powerline/lib/dict.py:5`.
///
/// Python: `REMOVE_THIS_KEY = object()` — a sentinel object used to
/// signal that a downstream config layer should DELETE a key set by
/// an upstream layer (rather than overriding it with a new value).
///
/// Rust analog: a sentinel JSON value with a marker string that
/// `mergedicts` checks for via direct `Value` comparison. Using a
/// well-known marker string is the round-trippable shape for JSON
/// configs (a Python `object()` sentinel doesn't survive
/// JSON serialisation, but `{"__powerliners_remove_this_key__": true}`
/// does — and matches the upstream semantic of "I am the deletion
/// marker, not a real value").
#[allow(non_snake_case)]
pub fn REMOVE_THIS_KEY() -> Value {
    // py:5
    serde_json::json!({"__powerliners_remove_this_key__": true})
}

/// Port of `mergeargs()` from `powerline/lib/dict.py:8`.
///
/// Python:
/// ```python
/// def mergeargs(argvalue, remove=False):
///     if not argvalue:
///         return None
///     r = {}
///     for subval in argvalue:
///         mergedicts(r, dict([subval]), remove=remove)
///     return r
/// ```
///
/// Takes an iterable of `(key, value)` pairs and folds them into a
/// single dict via `mergedicts`. Used by the CLI argument parser to
/// merge `-t name=value` overrides into the config tree.
pub fn mergeargs<I>(argvalue: I, remove: bool) -> Option<Map<String, Value>>
where
    I: IntoIterator<Item = (String, Value)>,
{
    let mut iter = argvalue.into_iter().peekable(); // py:9
    if iter.peek().is_none() {
        return None; // py:10  if not argvalue: return None
    }
    let mut r: Map<String, Value> = Map::new(); // py:11
    for (k, v) in iter {
        // py:12-13
        let mut single = Map::new();
        single.insert(k, v);
        mergedicts(&mut r, single, remove);
    }
    Some(r) // py:14
}

/// Port of `_clear_special_values()` from `powerline/lib/dict.py:17`.
///
/// Remove REMOVE_THIS_KEY values from a (possibly nested) dictionary.
/// Iterative walk via an explicit stack (matching the Python code's
/// `l = [d]; while l: i = l.pop(); ...` shape).
pub fn _clear_special_values(d: &mut Map<String, Value>) {
    let mut l: Vec<*mut Map<String, Value>> = vec![d as *mut _]; // py:20
                                                                 // SAFETY: We only ever push pointers to live `Map`s reachable from
                                                                 // `d`. Each `Map<String, Value>` is owned by either `d` or a nested
                                                                 // `Value::Object` inside it; we hold no aliasing references during
                                                                 // the inner loop, and we never visit the same map twice.
    while let Some(p) = l.pop() {
        // py:21-22
        let i = unsafe { &mut *p };
        let mut pops: Vec<String> = Vec::new(); // py:23
        for (k, v) in i.iter_mut() {
            // py:24
            // py:25  isinstance check + sentinel identity — see REMOVE_THIS_KEY at py:5
            if matches!(
                v.get("__powerliners_remove_this_key__"),
                Some(Value::Bool(true))
            ) {
                // py:25
                pops.push(k.clone()); // py:26
            } else if let Value::Object(child) = v {
                // py:27  isinstance(v, dict)
                l.push(child as *mut _); // py:28
            }
        }
        for k in pops {
            // py:29
            i.remove(&k); // py:30  i.pop(k)
        }
    }
}

/// Port of `mergedicts()` from `powerline/lib/dict.py:33`.
///
/// Recursively merge `d2` into `d1` in place.
///
/// - If both `d1[k]` and `d2[k]` are dicts → recurse.
/// - If `d2[k]` is REMOVE_THIS_KEY and `remove` is true → delete `d1[k]`.
/// - Otherwise → `d1[k] = d2[k]` (with REMOVE_THIS_KEY scrubbed from
///   nested dicts when `remove` is true).
pub fn mergedicts(d1: &mut Map<String, Value>, d2: Map<String, Value>, remove: bool) {
    _setmerged(d1, &d2); // py:38
    for (k, v) in d2 {
        // py:39
        let in_d1_as_dict = matches!(d1.get(&k), Some(Value::Object(_)));
        let v_is_dict = matches!(v, Value::Object(_));
        if in_d1_as_dict && v_is_dict {
            // py:40
            if let (Some(Value::Object(inner1)), Value::Object(inner2)) = (d1.get_mut(&k), v) {
                mergedicts(inner1, inner2, remove); // py:41
            }
        } else if remove
            && matches!(
                v.get("__powerliners_remove_this_key__"),
                Some(Value::Bool(true))
            )
        {
            // py:42
            d1.remove(&k); // py:43  d1.pop(k, None)
        } else {
            // py:44
            let mut owned = v;
            if remove {
                if let Value::Object(ref mut inner) = owned {
                    _clear_special_values(inner); // py:45-46
                }
            }
            d1.insert(k, owned); // py:47
        }
    }
}

/// Port of `mergedefaults()` from `powerline/lib/dict.py:50`.
///
/// Recursively merge `d2` into `d1`, keeping existing values in `d1`
/// (`d1` wins on every key collision; `d2` is only used to fill gaps).
pub fn mergedefaults(d1: &mut Map<String, Value>, d2: Map<String, Value>) {
    for (k, v) in d2 {
        // py:55
        let in_d1_as_dict = matches!(d1.get(&k), Some(Value::Object(_)));
        let v_is_dict = matches!(v, Value::Object(_));
        if in_d1_as_dict && v_is_dict {
            // py:56
            if let (Some(Value::Object(inner1)), Value::Object(inner2)) = (d1.get_mut(&k), v) {
                mergedefaults(inner1, inner2); // py:57
            }
        } else {
            // py:58
            d1.entry(k).or_insert(v); // py:59  d1.setdefault(k, d2[k])
        }
    }
}

/// Port of `_setmerged()` from `powerline/lib/dict.py:62`.
///
/// Python:
/// ```python
/// def _setmerged(d1, d2):
///     if hasattr(d1, 'setmerged'):
///         d1.setmerged(d2)
/// ```
///
/// Hooks into `MarkedValue` (from `lib/markedjson`) so the lint
/// subsystem can track which config file contributed which keys.
/// powerliners doesn't yet have `MarkedValue`; this is a no-op until
/// `lint/markedjson/markedvalue.rs` lands.
pub fn _setmerged(_d1: &mut Map<String, Value>, _d2: &Map<String, Value>) {
    // py:63-64  hasattr(d1, 'setmerged') check + d1.setmerged(d2) call
    // No-op until MarkedValue port lands.
}

/// Port of `mergedicts_copy()` from `powerline/lib/dict.py:67`.
///
/// Recursively merge two dictionaries without mutating either input.
/// `d2` wins on every collision.
pub fn mergedicts_copy(d1: &Map<String, Value>, d2: Map<String, Value>) -> Map<String, Value> {
    let mut ret = d1.clone(); // py:73
    _setmerged(&mut ret, &d2); // py:74
    for (k, v) in d2 {
        // py:75
        let in_d1_as_dict = matches!(d1.get(&k), Some(Value::Object(_)));
        let v_is_dict = matches!(v, Value::Object(_));
        if in_d1_as_dict && v_is_dict {
            // py:76
            if let (Some(Value::Object(inner1)), Value::Object(inner2)) = (d1.get(&k), v) {
                ret.insert(k, Value::Object(mergedicts_copy(inner1, inner2))); // py:77
            }
        } else {
            // py:78
            ret.insert(k, v); // py:79
        }
    }
    ret // py:80
}

/// Port of `updated()` from `powerline/lib/dict.py:83`.
///
/// Copy `d` and merge in `args` and `kwargs`.
///
/// Python's `dict.update()` accepts both positional iterables and
/// keyword args; the Rust port collapses both into one explicit
/// `updates` iterable since Rust has no `**kwargs`.
pub fn updated<I>(d: &Map<String, Value>, updates: I) -> Map<String, Value>
where
    I: IntoIterator<Item = (String, Value)>,
{
    let mut d = d.clone(); // py:86
    for (k, v) in updates {
        // py:87  d.update(*args, **kwargs)
        d.insert(k, v);
    }
    d // py:88
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: Value) -> Map<String, Value> {
        match v {
            Value::Object(m) => m,
            _ => panic!("not an object: {:?}", v),
        }
    }

    #[test]
    fn mergedicts_overwrites_scalar() {
        let mut d1 = obj(json!({"a": 1, "b": 2}));
        let d2 = obj(json!({"b": 3, "c": 4}));
        mergedicts(&mut d1, d2, true);
        assert_eq!(Value::Object(d1), json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn mergedicts_recurses_into_nested() {
        let mut d1 = obj(json!({"a": {"x": 1, "y": 2}}));
        let d2 = obj(json!({"a": {"y": 20, "z": 30}}));
        mergedicts(&mut d1, d2, true);
        assert_eq!(Value::Object(d1), json!({"a": {"x": 1, "y": 20, "z": 30}}));
    }

    #[test]
    fn mergedicts_remove_this_key_deletes() {
        let mut d1 = obj(json!({"a": 1, "b": 2}));
        let mut d2 = Map::new();
        d2.insert("b".to_string(), REMOVE_THIS_KEY());
        mergedicts(&mut d1, d2, true);
        assert_eq!(Value::Object(d1), json!({"a": 1}));
    }

    #[test]
    fn mergedefaults_keeps_existing() {
        let mut d1 = obj(json!({"a": 1}));
        let d2 = obj(json!({"a": 2, "b": 3}));
        mergedefaults(&mut d1, d2);
        assert_eq!(Value::Object(d1), json!({"a": 1, "b": 3}));
    }

    #[test]
    fn mergedicts_copy_does_not_mutate_inputs() {
        let d1 = obj(json!({"a": 1, "nested": {"x": 1}}));
        let d2 = obj(json!({"a": 2, "nested": {"y": 2}}));
        let merged = mergedicts_copy(&d1, d2.clone());
        // Originals unchanged.
        assert_eq!(d1.get("a"), Some(&json!(1)));
        assert_eq!(d2.get("a"), Some(&json!(2)));
        // Merge result.
        assert_eq!(
            Value::Object(merged),
            json!({"a": 2, "nested": {"x": 1, "y": 2}})
        );
    }

    #[test]
    fn mergeargs_empty_returns_none() {
        let r: Option<Map<String, Value>> = mergeargs(std::iter::empty(), false);
        assert!(r.is_none());
    }

    #[test]
    fn mergeargs_folds_pairs() {
        let r = mergeargs(
            vec![("a".to_string(), json!(1)), ("b".to_string(), json!(2))],
            false,
        )
        .unwrap();
        assert_eq!(Value::Object(r), json!({"a": 1, "b": 2}));
    }

    #[test]
    fn updated_does_not_mutate_input() {
        let d = obj(json!({"a": 1}));
        let r = updated(&d, vec![("b".to_string(), json!(2))]);
        assert_eq!(d.get("a"), Some(&json!(1)));
        assert!(d.get("b").is_none(), "original was mutated");
        assert_eq!(Value::Object(r), json!({"a": 1, "b": 2}));
    }
}
