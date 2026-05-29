// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/context.py`.
//!
//! Context-tracking helpers used by the linter to attach
//! breadcrumb-style paths (`ext/shell/colorscheme/...`) to error
//! messages.
//!
//! Upstream `Context` subclasses `tuple` and overrides every mutator
//! to raise `TypeError`. Rust has no tuple-subclassing; the port
//! models it as an immutable `Vec<(String, Value)>` wrapper with the
//! same `enter` / `enter_key` / `enter_item` / `key` methods.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import itertools                                 // py:4
// from powerline.lib.unicode import unicode        // py:6
// from powerline.lint.markedjson.markedvalue import MarkedUnicode                          // py:7
// from powerline.lint.selfcheck import havemarks                                            // py:8

use serde_json::Value;

/// Port of `class JStr(unicode)` from `powerline/lint/context.py:11`.
///
/// Python: a `str` subclass that overrides `.join(iterable)` to call
/// `unicode(item)` on each element before joining.
///
/// Rust port: a thin newtype wrapping the separator string. The
/// `join` method auto-stringifies items via `Display`.
pub struct JStr(pub String);

impl JStr {
    /// Port of `JStr.join()` from `powerline/lint/context.py:12`.
    pub fn join<I, T>(&self, iterable: I) -> String
    where
        I: IntoIterator<Item = T>,
        T: std::fmt::Display,
    {
        // py:13  super().join((unicode(item) for item in iterable))
        let parts: Vec<String> = iterable.into_iter().map(|x| x.to_string()).collect();
        parts.join(&self.0)
    }
}

/// Port of module-level binding `key_sep` from
/// `powerline/lint/context.py:16`.
///
/// Python: `key_sep = JStr('/')` — the breadcrumb separator.
#[allow(non_upper_case_globals)]
pub fn key_sep() -> JStr {
    JStr("/".to_string())                            // py:16
}

/// Port of `list_themes()` from `powerline/lint/context.py:19`.
///
/// Returns the list of `(ext, theme_config)` pairs to walk for the
/// current lint cycle. Three modes per `theme_type`:
///   - `'top'`: every theme in every ext (py:24-28)
///   - `'main'` or `is_main_theme`: every theme in the current ext (py:29-30)
///   - other: just the parent context's theme (py:31-32)
pub fn list_themes(
    data: &serde_json::Map<String, Value>,
    context: &Context,
) -> Vec<(String, Value)> {
    // py:20  theme_type = data['theme_type']
    let theme_type = data
        .get("theme_type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // py:21  ext = data['ext']
    let ext = data.get("ext").and_then(|v| v.as_str()).unwrap_or("");
    // py:22  main_theme_name = data['main_config'].get('ext', {}).get(ext, {}).get('theme', None)
    let main_theme_name = data
        .get("main_config")
        .and_then(|v| v.get("ext"))
        .and_then(|v| v.get(ext))
        .and_then(|v| v.get("theme"))
        .and_then(|v| v.as_str());
    // py:23  is_main_theme = (data['theme'] == main_theme_name)
    let cur_theme = data.get("theme").and_then(|v| v.as_str());
    let is_main_theme = cur_theme.is_some() && cur_theme == main_theme_name;

    if theme_type == "top" {
        // py:24-28  every theme in every ext
        let mut out = Vec::new();
        if let Some(tc) = data.get("theme_configs").and_then(|v| v.as_object()) {
            for (theme_ext, theme_configs) in tc {
                if let Some(theme_obj) = theme_configs.as_object() {
                    for theme in theme_obj.values() {
                        out.push((theme_ext.clone(), theme.clone()));
                    }
                }
            }
        }
        out
    } else if theme_type == "main" || is_main_theme {
        // py:29-30  every theme in the current ext
        let mut out = Vec::new();
        if let Some(ext_themes) = data.get("ext_theme_configs").and_then(|v| v.as_object()) {
            for theme in ext_themes.values() {
                out.push((ext.to_string(), theme.clone()));
            }
        }
        out
    } else {
        // py:31-32  the parent context's theme only
        if context.0.is_empty() {
            Vec::new()
        } else {
            // The last element of the context tuple carries the (key, value).
            let (_k, v) = context.0.last().unwrap().clone();
            vec![(ext.to_string(), v)]
        }
    }
}

/// Port of `class Context` from `powerline/lint/context.py:35`.
///
/// Immutable breadcrumb tuple. Each entry is `(key, value)`. New
/// contexts are built by `enter` (returns a new `Context` with one
/// more entry); mutation via index-set / pop / etc. is forbidden
/// (mirrors Python's TypeError-raising overrides at py:36-43).
#[derive(Debug, Clone)]
pub struct Context(pub Vec<(String, Value)>);

impl Context {
    /// Port of `Context.__new__()` from `powerline/lint/context.py:47`.
    ///
    /// Two construction modes:
    ///   - `new(base)` with `base: &Context` — extends with the same
    ///     base (py:53-55)
    ///   - `new_with_kv(base, key, value)` — appends one (key, value)
    ///     to the existing context (py:48-52)
    pub fn new() -> Self {
        Context(Vec::new())
    }

    /// Append a `(key, value)` to a context, returning a new Context.
    pub fn enter(&self, context_key: String, context_value: Value) -> Self {
        // py:67-68  return Context.__new__(Context, self, context_key, context_value)
        let mut entries = self.0.clone();
        entries.push((context_key, context_value));
        Context(entries)
    }

    /// Port of `Context.key` property from
    /// `powerline/lint/context.py:57-59`.
    ///
    /// Joins the keys of every entry with `key_sep` ('/').
    pub fn key(&self) -> String {
        // py:59  key_sep.join((c[0] for c in self))
        key_sep().join(self.0.iter().map(|(k, _)| k.clone()))
    }

    /// Port of `Context.enter_key()` from
    /// `powerline/lint/context.py:61`.
    ///
    /// `value.keydict[key], value[key]` shape — Rust port operates on
    /// `serde_json::Value::Object` directly.
    pub fn enter_key(&self, value: &Value, key: &str) -> Self {
        // py:62  return self.enter(value.keydict[key], value[key])
        let inner = value
            .get(key)
            .cloned()
            .unwrap_or(Value::Null);
        self.enter(key.to_string(), inner)
    }

    /// Port of `Context.enter_item()` from
    /// `powerline/lint/context.py:64`.
    pub fn enter_item(&self, name: &str, item: &Value) -> Self {
        // py:65  return self.enter(MarkedUnicode(name, item.mark), item)
        self.enter(name.to_string(), item.clone())
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn jstr_joins_with_separator() {
        let sep = JStr("/".to_string());
        assert_eq!(sep.join(vec!["a", "b", "c"]), "a/b/c");
        assert_eq!(sep.join(vec![1, 2, 3]), "1/2/3");
    }

    #[test]
    fn key_sep_is_slash() {
        assert_eq!(key_sep().0, "/");
    }

    #[test]
    fn context_starts_empty() {
        let c = Context::new();
        assert!(c.0.is_empty());
        assert_eq!(c.key(), "");
    }

    #[test]
    fn context_enter_appends_one_entry() {
        let c = Context::new();
        let c1 = c.enter("ext".into(), json!("shell"));
        assert_eq!(c1.0.len(), 1);
        assert_eq!(c1.key(), "ext");
        let c2 = c1.enter("theme".into(), json!("default"));
        assert_eq!(c2.0.len(), 2);
        assert_eq!(c2.key(), "ext/theme");
    }

    #[test]
    fn context_enter_does_not_mutate_parent() {
        let c = Context::new();
        let _c1 = c.enter("ext".into(), json!("shell"));
        assert!(c.0.is_empty(), "parent context should be unchanged");
    }

    #[test]
    fn context_enter_key_pulls_value_from_obj() {
        let c = Context::new();
        let v = json!({"colorscheme": "default"});
        let c1 = c.enter_key(&v, "colorscheme");
        assert_eq!(c1.0.last().unwrap().0, "colorscheme");
        assert_eq!(c1.0.last().unwrap().1, json!("default"));
    }

    #[test]
    fn list_themes_top_returns_every_theme() {
        let mut data = serde_json::Map::new();
        data.insert("theme_type".into(), json!("top"));
        data.insert("ext".into(), json!("shell"));
        data.insert("theme".into(), json!("default"));
        data.insert("main_config".into(), json!({}));
        data.insert(
            "theme_configs".into(),
            json!({
                "shell": {"default": {"k": "v1"}, "alt": {"k": "v2"}},
                "tmux": {"default": {"k": "v3"}}
            }),
        );
        let c = Context::new();
        let r = list_themes(&data, &c);
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn list_themes_main_returns_ext_themes() {
        let mut data = serde_json::Map::new();
        data.insert("theme_type".into(), json!("main"));
        data.insert("ext".into(), json!("shell"));
        data.insert("theme".into(), json!("default"));
        data.insert("main_config".into(), json!({}));
        data.insert(
            "ext_theme_configs".into(),
            json!({"default": {"k": "v"}, "alt": {"k": "w"}}),
        );
        let c = Context::new();
        let r = list_themes(&data, &c);
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|(ext, _)| ext == "shell"));
    }
}
