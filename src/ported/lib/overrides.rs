// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/overrides.py`.
//!
//! CLI override-value parsing. Used by the `--theme-option k=v` and
//! `--config-override k=v` flags on `powerline` and `powerline-render`
//! to inject config tweaks at runtime.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import json                                      // py:4
// from powerline.lib.dict import REMOVE_THIS_KEY   // py:6

use crate::ported::lib::dict::REMOVE_THIS_KEY;
use serde_json::Value;

/// Port of `parse_value()` from `powerline/lib/overrides.py:9`.
///
/// Convert string to Python object.
///
/// Rules:
///
/// * Empty string means that corresponding key should be removed from
///   the dictionary.
/// * Strings that start with a minus, digit or with some character
///   that starts JSON collection or string object are parsed as JSON.
/// * JSON special values `null`, `true`, `false` (case matters) are
///   parsed as JSON.
/// * All other values are considered to be raw strings.
///
/// :param str s: Parsed string.
///
/// :return: Python object.
pub fn parse_value(s: &str) -> Value {
    // py:9  def parse_value(s):
    // py:26  if not s:
    if s.is_empty() {
        // py:27  return REMOVE_THIS_KEY
        return REMOVE_THIS_KEY();
    }
    // py:28  elif s[0] in '"{[0123456789-' or s in ('null', 'true', 'false'):
    let first = s.chars().next().unwrap();
    let starts_json = matches!(
        first,
        '"' | '{' | '[' | '0' | '1' | '2' | '3' | '4' | '5' | '6' | '7' | '8' | '9' | '-'
    );
    let is_special = s == "null" || s == "true" || s == "false";
    if starts_json || is_special {
        // py:29  return json.loads(s)
        serde_json::from_str(s).unwrap_or_else(|_| Value::String(s.to_string()))
    } else {
        // py:30  else:
        // py:31  return s
        Value::String(s.to_string())
    }
}

/// Port of `keyvaluesplit()` from `powerline/lib/overrides.py:34`.
///
/// Split `K1.K2=VAL` into `K1.K2` and parsed VAL.
pub fn keyvaluesplit(s: &str) -> Result<(String, Value), String> {
    // py:34  def keyvaluesplit(s):
    // py:37  if '=' not in s:
    if !s.contains('=') {
        // py:38  raise TypeError('Option must look like option=json_value')
        return Err("Option must look like option=json_value".to_string());
    }
    // py:39  if s[0] == '_':
    if s.starts_with('_') {
        // py:40  raise ValueError('Option names must not start with `_\'')
        return Err("Option names must not start with `_'".to_string());
    }
    // py:41  idx = s.index('=')
    let idx = s.find('=').unwrap();
    // py:42  o = s[:idx]
    let o = s[..idx].to_string();
    // py:43  val = parse_value(s[idx + 1:])
    let val = parse_value(&s[idx + 1..]);
    // py:44  return (o, val)
    Ok((o, val))
}

/// Port of `parsedotval()` from `powerline/lib/overrides.py:47`.
///
/// Parse `K1.K2=VAL` into `{"K1": {"K2": VAL}}`.
///
/// `VAL` is processed according to rules defined in `parse_value`.
///
/// Python accepts either a `str` or a pre-split `(o, val)` tuple as
/// input. The Rust port splits this into two functions to match the
/// two call shapes: `parsedotval_str` for the str case and
/// `parsedotval_tuple` for the (o, val) case. Both write to the same
/// output shape.
pub fn parsedotval_str(s: &str) -> Result<(String, Value), String> {
    // py:47  def parsedotval(s):
    // py:52  if type(s) is tuple:
    // py:55  else:
    // py:56  o, val = keyvaluesplit(s)
    let (o, val) = keyvaluesplit(s)?;
    Ok(build_nested(&o, val))
}

/// Tuple variant of `parsedotval` — matches the Python branch at
/// `powerline/lib/overrides.py:53-54` where `s` is already `(o, val_str)`
/// and `val_str` is re-parsed via `parse_value`.
pub fn parsedotval_tuple(o: &str, val: &str) -> (String, Value) {
    // py:52  if type(s) is tuple:
    // py:53  o, val = s
    // py:54  val = parse_value(val)
    let parsed_val = parse_value(val);
    build_nested(o, parsed_val)
}

/// Builds the nested-dict shape from a dotted key + already-parsed value.
/// Inlined from `parsedotval` body at `powerline/lib/overrides.py:58-68`.
fn build_nested(o: &str, val: Value) -> (String, Value) {
    // py:58  keys = o.split('.')
    let keys: Vec<&str> = o.split('.').collect();
    // py:59  if len(keys) > 1:
    if keys.len() > 1 {
        // py:60  r = (keys[0], {})
        // py:61  rcur = r[1]
        // py:62  for key in keys[1:-1]:
        // py:63  rcur[key] = {}
        // py:64  rcur = rcur[key]
        // py:65  rcur[keys[-1]] = val
        let mut current = val;
        for k in keys[1..].iter().rev() {
            let mut m = serde_json::Map::new();
            m.insert(k.to_string(), current);
            current = Value::Object(m);
        }
        // py:66  return r
        (keys[0].to_string(), current)
    } else {
        // py:67  else:
        // py:68  return (o, val)
        (o.to_string(), val)
    }
}

/// Input dispatch for [`parsedotval`] — matches Python's runtime
/// `type(s) is tuple` discrimination at `powerline/lib/overrides.py:52`.
///
/// Python's `parsedotval(s)` takes either a `str` or a pre-split
/// `(o, val_str)` tuple. Rust can't dispatch on parameter type at
/// runtime without a sum type, so the input is wrapped here.
pub enum ParseDotValInput<'a> {
    /// `s` is a `"K1.K2=VAL"`-shaped string — parsed via
    /// [`keyvaluesplit`].
    Str(&'a str),
    /// `s` is the pre-split `(o, val_str)` tuple — `val_str` is
    /// re-parsed via [`parse_value`].
    Tuple(&'a str, &'a str),
}

/// Port of `parsedotval()` from `powerline/lib/overrides.py:47`.
///
/// Dispatches to [`parsedotval_str`] or [`parsedotval_tuple`] based
/// on the input variant — mirrors Python's runtime
/// `if type(s) is tuple` branch at py:52.
///
/// Returns the nested-shape `(key, value)` per py:60-68 (or the flat
/// `(o, val)` per py:67-68 when no dots are present).
pub fn parsedotval(input: ParseDotValInput<'_>) -> Result<(String, Value), String> {
    match input {
        ParseDotValInput::Str(s) => parsedotval_str(s),
        ParseDotValInput::Tuple(o, val) => Ok(parsedotval_tuple(o, val)),
    }
}

/// Port of `parse_override_var()` from `powerline/lib/overrides.py:71`.
///
/// Parse a semicolon-separated list of strings into a sequence of values.
///
/// Emits the same items in sequence as `parsedotval` does.
pub fn parse_override_var(s: &str) -> Vec<(String, Value)> {
    // py:71  def parse_override_var(s):
    // py:76  return (
    // py:77  parsedotval(item)
    // py:78  for item in s.split(';')
    // py:79  if item
    // py:80  )
    s.split(';')
        .filter(|item| !item.is_empty())
        .filter_map(|item| parsedotval_str(item).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_value_empty_returns_remove_marker() {
        let v = parse_value("");
        assert_eq!(v, REMOVE_THIS_KEY());
    }

    #[test]
    fn parse_value_number() {
        assert_eq!(parse_value("42"), json!(42));
        assert_eq!(parse_value("-3"), json!(-3));
        assert_eq!(parse_value("2.5"), json!(2.5_f64));
    }

    #[test]
    fn parse_value_string_in_quotes() {
        assert_eq!(parse_value(r#""hello""#), json!("hello"));
    }

    #[test]
    fn parse_value_specials() {
        assert_eq!(parse_value("null"), json!(null));
        assert_eq!(parse_value("true"), json!(true));
        assert_eq!(parse_value("false"), json!(false));
    }

    #[test]
    fn parse_value_array_and_object() {
        assert_eq!(parse_value("[1,2,3]"), json!([1, 2, 3]));
        assert_eq!(parse_value(r#"{"a":1}"#), json!({"a": 1}));
    }

    #[test]
    fn parse_value_raw_string() {
        assert_eq!(parse_value("hello"), json!("hello"));
        // Note: "TRUE" is not "true" (case matters per py:18)
        assert_eq!(parse_value("TRUE"), json!("TRUE"));
    }

    #[test]
    fn keyvaluesplit_basic() {
        let (k, v) = keyvaluesplit("ext.tmux.theme=default").unwrap();
        assert_eq!(k, "ext.tmux.theme");
        assert_eq!(v, json!("default"));
    }

    #[test]
    fn keyvaluesplit_no_equals_errors() {
        assert!(keyvaluesplit("no-equals-here").is_err());
    }

    #[test]
    fn keyvaluesplit_leading_underscore_errors() {
        assert!(keyvaluesplit("_private=42").is_err());
    }

    #[test]
    fn parsedotval_str_builds_nested() {
        let (k, v) = parsedotval_str("ext.tmux.theme=default").unwrap();
        assert_eq!(k, "ext");
        assert_eq!(v, json!({"tmux": {"theme": "default"}}));
    }

    #[test]
    fn parsedotval_str_no_dot_is_flat() {
        let (k, v) = parsedotval_str("foo=42").unwrap();
        assert_eq!(k, "foo");
        assert_eq!(v, json!(42));
    }

    #[test]
    fn parse_override_var_splits_on_semicolon() {
        let items = parse_override_var("a=1;b=2;c.d=3");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], ("a".to_string(), json!(1)));
        assert_eq!(items[1], ("b".to_string(), json!(2)));
        assert_eq!(items[2], ("c".to_string(), json!({"d": 3})));
    }

    #[test]
    fn parse_override_var_skips_empty() {
        let items = parse_override_var("a=1;;b=2;");
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parsedotval_dispatches_str_to_keyvaluesplit() {
        // py:55-56  type(s) is not tuple → keyvaluesplit branch
        let (key, val) = parsedotval(ParseDotValInput::Str("foo.bar=42")).unwrap();
        assert_eq!(key, "foo");
        assert_eq!(val, json!({"bar": 42}));
    }

    #[test]
    fn parsedotval_dispatches_tuple_to_parse_value() {
        // py:52-54  type(s) is tuple → parse_value(val) branch
        let (key, val) = parsedotval(ParseDotValInput::Tuple("foo.bar", "42")).unwrap();
        assert_eq!(key, "foo");
        assert_eq!(val, json!({"bar": 42}));
    }

    #[test]
    fn parsedotval_str_no_dot_returns_flat_pair() {
        // py:67-68  else branch — no dots, return (o, val)
        let (key, val) = parsedotval(ParseDotValInput::Str("flag=true")).unwrap();
        assert_eq!(key, "flag");
        assert_eq!(val, json!(true));
    }
}
