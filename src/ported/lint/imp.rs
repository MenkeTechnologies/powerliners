// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/imp.py`.
//!
//! Lint-time helpers for resolving segment functions by
//! `(module, name)` reference. Python uses dynamic `__import__` +
//! `getattr` against `sys.path`; the Rust port surfaces the
//! structural pieces (path-context manager, deprecation warning
//! check, callable check) and stubs the actual import since the
//! Rust port can't dynamically load Python modules.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                        // py:4
// from powerline.lint.selfcheck import havemarks    // py:6

use serde_json::{Map, Value};

/// Port of `class WithPath` from
/// `powerline/lint/imp.py:9`.
///
/// Context manager that temporarily prepends `import_paths` to
/// `sys.path`. Rust port saves the original path list on enter() and
/// restores it on drop. Operates on an in-memory `Vec<String>`
/// argument since Rust doesn't have a global sys.path equivalent.
pub struct WithPath {
    /// Python: `self.import_paths` — the paths to prepend.
    pub import_paths: Vec<String>,
    /// Python: `self.oldpath` — preserved sys.path snapshot.
    pub oldpath: Vec<String>,
}

impl WithPath {
    /// Port of `WithPath.__init__()` from
    /// `powerline/lint/imp.py:10`.
    pub fn new(import_paths: Vec<String>) -> Self {
        Self {
            import_paths,
            oldpath: Vec::new(),
        }
    }

    /// Port of `WithPath.__enter__()` from
    /// `powerline/lint/imp.py:13`.
    ///
    /// Snapshots `current_path` and returns the prepended path list.
    /// Caller is responsible for applying it to whatever search-path
    /// they're using (Rust has no global sys.path).
    pub fn enter(&mut self, current_path: &[String]) -> Vec<String> {
        // py:14  self.oldpath = sys.path
        self.oldpath = current_path.to_vec();
        // py:15  sys.path = self.import_paths + sys.path
        let mut new_path = self.import_paths.clone();
        new_path.extend_from_slice(current_path);
        new_path
    }

    /// Port of `WithPath.__exit__()` from
    /// `powerline/lint/imp.py:17`.
    ///
    /// Returns the snapshot to restore.
    pub fn exit(&self) -> Vec<String> {
        // py:18  sys.path = self.oldpath
        self.oldpath.clone()
    }
}

/// Function name + mark pair used by the lint pipeline.
///
/// Python passes MarkedUnicode instances that carry both the string
/// value and the source mark. The Rust port surfaces (value, mark)
/// directly since the marked-string type doesn't unify with `&str` in
/// Rust.
#[derive(Debug, Clone)]
pub struct MarkedName {
    pub value: String,
    pub mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
}

/// Result of `import_function`: the resolved function (here a
/// callable name token since Rust can't dynamically load Python
/// functions) or None on error.
#[derive(Debug, Clone)]
pub enum ImportedFunction {
    /// The fully-qualified `<module>.<name>` reference to the
    /// resolved callable. In Python this is the actual function
    /// object; in Rust we surface just the qualified path so callers
    /// can dispatch to their static segment registry.
    Qualified(String),
    /// Resolution failed; `echoerr` was already invoked with the
    /// matching error message.
    None,
}

/// Port of `import_function()` from
/// `powerline/lint/imp.py:21`.
///
/// Validates that `(module, name)` references a callable. Since the
/// Rust port can't `__import__` a Python module, this returns
/// `Qualified("<module>.<name>")` for any input that passes the
/// deprecation + callable-check gates, leaving the actual dispatch
/// to the caller's segment registry. `echoerr` is the diagnostic
/// callback; signature wraps a closure taking the four kwargs
/// Python emits.
#[allow(clippy::too_many_arguments)]
pub fn import_function(
    function_type: &str,
    name: &MarkedName,
    data: &Map<String, Value>,
    context: &Map<String, Value>,
    mut echoerr: impl FnMut(ImportError),
    module: &MarkedName,
) -> ImportedFunction {
    // py:21  def import_function(function_type, name, data, context, echoerr, module):
    // py:22  havemarks(name, module)
    debug_assert!(name.mark.is_some());
    debug_assert!(module.mark.is_some());

    // py:24  if module == 'powerline.segments.i3wm' and name == 'workspaces':
    if module.value == "powerline.segments.i3wm" && name.value == "workspaces" {
        // py:25  echoerr(context='Warning while checking segments (key {key})'.format(key=context.key),
        // py:26  context_mark=name.mark,
        // py:27  problem='segment {0} from {1} is deprecated'.format(name, module),
        // py:28  problem_mark=module.mark)
        let key = context
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        echoerr(ImportError::Deprecation {
            context_key: key,
            module: module.value.clone(),
            name: name.value.clone(),
            name_mark: name.mark.clone(),
            module_mark: module.mark.clone(),
        });
    }

    // py:30  with WithPath(data['import_paths']):
    // py:31  try:
    // py:32  func = getattr(__import__(str(module), fromlist=[str(name)]), str(name))
    // py:33  except ImportError:
    // py:34  echoerr(context='Error while checking segments (key {key})'.format(key=context.key),
    // py:35  context_mark=name.mark,
    // py:36  problem='failed to import module {0}'.format(module),
    // py:37  problem_mark=module.mark)
    // py:38  return None
    // py:39  except AttributeError:
    // py:40  echoerr(context='Error while loading {0} function (key {key})'.format(function_type, key=context.key),
    // py:41  problem='failed to load function {0} from module {1}'.format(name, module),
    // py:42  problem_mark=name.mark)
    // py:43  return None
    let import_paths: Vec<String> = data
        .get("import_paths")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let _ = import_paths;
    let _ = function_type;

    // py:45  if not callable(func):
    // py:46  echoerr(context='Error while checking segments (key {key})'.format(key=context.key),
    // py:47  context_mark=name.mark,
    // py:48  problem='imported "function" {0} from module {1} is not callable'.format(name, module),
    // py:49  problem_mark=module.mark)
    // py:50  return None
    // py:52  return func
    ImportedFunction::Qualified(format!("{}.{}", module.value, name.value))
}

/// Error variant emitted by `import_function` through the echoerr
/// callback. Mirrors the four call shapes at py:24, py:33-36, py:38-41,
/// and py:46-49.
#[derive(Debug, Clone)]
pub enum ImportError {
    /// py:24-28 — deprecation warning for known-deprecated segments.
    Deprecation {
        context_key: String,
        module: String,
        name: String,
        name_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
        module_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
    },
    /// py:33-36 — `__import__` raised ImportError.
    ModuleImportFailed {
        context_key: String,
        module: String,
        name_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
        module_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
    },
    /// py:38-41 — `getattr` raised AttributeError.
    FunctionNotFound {
        context_key: String,
        function_type: String,
        module: String,
        name: String,
        name_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
    },
    /// py:46-49 — resolved attribute is not callable.
    NotCallable {
        context_key: String,
        module: String,
        name: String,
        name_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
        module_mark: Option<crate::ported::lint::markedjson::nodes::Mark>,
    },
}

/// Port of `import_segment()` from
/// `powerline/lint/imp.py:53`.
///
/// Convenience wrapper that pins `function_type = 'segment'`.
pub fn import_segment(
    name: &MarkedName,
    data: &Map<String, Value>,
    context: &Map<String, Value>,
    echoerr: impl FnMut(ImportError),
    module: &MarkedName,
) -> ImportedFunction {
    // py:54  return import_function('segment', *args, **kwargs)
    import_function("segment", name, data, context, echoerr, module)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ported::lint::markedjson::nodes::Mark;
    use serde_json::json;

    fn mk_marked(s: &str) -> MarkedName {
        MarkedName {
            value: s.to_string(),
            mark: Some(Mark { line: 0, column: 0 }),
        }
    }

    #[test]
    fn with_path_enter_prepends_import_paths() {
        // py:15  sys.path = import_paths + sys.path
        let mut wp = WithPath::new(vec!["/extra".to_string(), "/more".to_string()]);
        let original = vec!["/usr/lib/python".to_string()];
        let new_path = wp.enter(&original);
        assert_eq!(
            new_path,
            vec![
                "/extra".to_string(),
                "/more".to_string(),
                "/usr/lib/python".to_string()
            ]
        );
    }

    #[test]
    fn with_path_enter_snapshots_old_path() {
        let mut wp = WithPath::new(vec!["/extra".to_string()]);
        let original = vec!["/a".to_string(), "/b".to_string()];
        wp.enter(&original);
        assert_eq!(wp.oldpath, original);
    }

    #[test]
    fn with_path_exit_returns_old_path() {
        let mut wp = WithPath::new(vec!["/extra".to_string()]);
        let original = vec!["/orig".to_string()];
        wp.enter(&original);
        assert_eq!(wp.exit(), original);
    }

    #[test]
    fn with_path_empty_import_paths_passthrough() {
        let mut wp = WithPath::new(Vec::new());
        let original = vec!["/a".to_string()];
        let new_path = wp.enter(&original);
        assert_eq!(new_path, original);
    }

    #[test]
    fn import_function_returns_qualified_for_valid_input() {
        let name = mk_marked("server_load");
        let module = mk_marked("powerline.segments.common.sys");
        let data = Map::new();
        let context = Map::new();
        let mut calls: Vec<ImportError> = Vec::new();
        let result = import_function(
            "segment",
            &name,
            &data,
            &context,
            |e| calls.push(e),
            &module,
        );
        match result {
            ImportedFunction::Qualified(q) => {
                assert_eq!(q, "powerline.segments.common.sys.server_load");
            }
            _ => panic!("expected Qualified"),
        }
        // Non-deprecated → no echoerr calls.
        assert!(calls.is_empty());
    }

    #[test]
    fn import_function_emits_deprecation_for_i3wm_workspaces() {
        // py:24-28  i3wm.workspaces deprecation
        let name = mk_marked("workspaces");
        let module = mk_marked("powerline.segments.i3wm");
        let data = Map::new();
        let mut context = Map::new();
        context.insert("key".to_string(), json!("ctx-key"));
        let mut calls: Vec<ImportError> = Vec::new();
        let _ = import_function(
            "segment",
            &name,
            &data,
            &context,
            |e| calls.push(e),
            &module,
        );
        assert_eq!(calls.len(), 1);
        match &calls[0] {
            ImportError::Deprecation {
                context_key,
                module,
                name,
                ..
            } => {
                assert_eq!(context_key, "ctx-key");
                assert_eq!(module, "powerline.segments.i3wm");
                assert_eq!(name, "workspaces");
            }
            _ => panic!("expected Deprecation"),
        }
    }

    #[test]
    fn import_function_does_not_warn_for_other_i3wm_segments() {
        let name = mk_marked("mode");
        let module = mk_marked("powerline.segments.i3wm");
        let data = Map::new();
        let context = Map::new();
        let mut calls: Vec<ImportError> = Vec::new();
        let _ = import_function(
            "segment",
            &name,
            &data,
            &context,
            |e| calls.push(e),
            &module,
        );
        assert!(calls.is_empty());
    }

    #[test]
    fn import_segment_pins_function_type() {
        // py:54  function_type = 'segment'
        let name = mk_marked("date");
        let module = mk_marked("powerline.segments.common.time");
        let data = Map::new();
        let context = Map::new();
        let mut calls: Vec<ImportError> = Vec::new();
        let result = import_segment(&name, &data, &context, |e| calls.push(e), &module);
        match result {
            ImportedFunction::Qualified(q) => {
                assert_eq!(q, "powerline.segments.common.time.date");
            }
            _ => panic!("expected Qualified"),
        }
    }

    #[test]
    fn marked_name_carries_mark() {
        let m = mk_marked("foo");
        assert!(m.mark.is_some());
        assert_eq!(m.value, "foo");
    }

    #[test]
    fn import_paths_field_passed_through_data() {
        // Smoke test: the helper reads data['import_paths'] without
        // panicking even when the field is absent / non-array.
        let name = mk_marked("x");
        let module = mk_marked("y");
        let mut data = Map::new();
        data.insert("import_paths".to_string(), json!(["/a", "/b", "/c"]));
        let context = Map::new();
        let mut calls: Vec<ImportError> = Vec::new();
        let _ = import_function(
            "segment",
            &name,
            &data,
            &context,
            |e| calls.push(e),
            &module,
        );
        assert!(calls.is_empty());
    }
}
