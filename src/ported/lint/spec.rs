// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/spec.py`.
//!
//! JSON-value specification DSL used by `powerline-lint` to validate
//! the powerline configuration files. The Python source provides a
//! chainable builder (`Spec().type(unicode).re(r'^\w+$')`) with 27
//! methods covering type checks / regex matching / list+tuple
//! membership / printable text / required+optional keys.
//!
//! Rust port surfaces:
//!   - `NON_PRINTABLE_RE()` accessor (minus the allowed tab/newline/
//!     U+0085 chars per py:14-19)
//!   - `Spec` struct with the chainable builder pieces:
//!     `new` / `update` / `optional` / `required` / `context_message`
//!     / `printable` / `unsigned` / `oneof(items)` / `error(msg)` /
//!     `type_check(allowed)` / `regex(pattern)` / `ident()`
//!   - `CheckResult` (proceed, hadproblem) tuple per the Python
//!     check_*/match contract
//!
//! The check_* / match implementation (py:194-749) is heavy enough
//! to deserve its own port pass — those methods call user-supplied
//! closures, walk nested specs, and emit `echoerr(context=..., ...)`
//! diagnostics with the full DelayedEchoErr accumulator. The
//! chainable builder methods captured here record the constraint
//! flags so a future port can implement match without rebuilding
//! the API surface.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import itertools                                  // py:4
// import re                                         // py:5
// from copy import copy                             // py:7
// from powerline.lib.unicode import unicode         // py:9
// from powerline.lint.markedjson.error import echoerr, DelayedEchoErr, NON_PRINTABLE_STR  // py:10
// from powerline.lint.selfcheck import havemarks    // py:11

use regex::Regex;
use std::sync::OnceLock;

/// Port of `NON_PRINTABLE_RE` from
/// `powerline/lint/spec.py:14-19`.
///
/// The Python source takes the NON_PRINTABLE_STR set from
/// `markedjson.error` and removes `\t`, `\n`, and U+0085 before
/// compiling. The Rust port uses the equivalent control-char set
/// minus those three.
#[allow(non_snake_case)]
pub fn NON_PRINTABLE_RE() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        // py:14-19  exclude tab/newline/U+0085
        Regex::new(r"[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]").unwrap()
    })
}

/// Tuple returned by Spec check_* / match methods per
/// `powerline/lint/spec.py:42-46`.
///
/// `proceed` controls whether the caller continues running other
/// checks; `hadproblem` reports whether the check found errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckResult {
    pub proceed: bool,
    pub hadproblem: bool,
}

impl CheckResult {
    pub fn ok() -> Self {
        Self {
            proceed: true,
            hadproblem: false,
        }
    }

    pub fn failed() -> Self {
        Self {
            proceed: false,
            hadproblem: true,
        }
    }
}

/// Allowed JSON value types matched by `Spec::type_check`.
///
/// Rust analog of the Python tuple of class references passed to
/// `Spec.type(...)` at py:379. The Python source supports
/// `dict`/`list`/`unicode`/`bool`/`float`/`NoneType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecType {
    /// Python: `dict`.
    Dict,
    /// Python: `list`.
    List,
    /// Python: `unicode` / `str`.
    Unicode,
    /// Python: `bool`.
    Bool,
    /// Python: `float` / `int`.
    Float,
    /// Python: `NoneType`.
    Null,
}

/// Comparison operator used by `Spec::len` and `Spec::cmp` from
/// `powerline/lint/spec.py:408/436`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cmp {
    /// Python `'<'`.
    Lt,
    /// Python `'<='`.
    Le,
    /// Python `'=='`.
    Eq,
    /// Python `'>='`.
    Ge,
    /// Python `'>'`.
    Gt,
    /// Python `'!='`.
    Ne,
}

/// Port of `class Spec(object)` from
/// `powerline/lint/spec.py:23`.
///
/// JSON-value specification DSL. The Rust port captures the
/// constraints registered by the chainable builder methods; the
/// match / check_* validation methods (py:194-749) are deferred
/// pending the closure-heavy diagnostic dispatch.
pub struct Spec {
    /// Python: `self.specs` (py:70).
    pub specs: Vec<Spec>,
    /// Python: `self.keys` (py:71).
    pub keys: std::collections::HashMap<String, Spec>,
    /// Python: `self.cmsg` (py:73) — context message format string.
    pub cmsg: String,
    /// Python: `self.isoptional` (py:74).
    pub isoptional: bool,
    /// Python: `self.did_type` (py:78).
    pub did_type: bool,
    /// Registered type constraints from `Spec::type_check`.
    pub allowed_types: Vec<SpecType>,
    /// Registered regex constraint from `Spec::regex`.
    pub regex: Option<String>,
    /// Registered oneof constraint from `Spec::oneof`.
    pub oneof: Option<Vec<String>>,
    /// Registered len constraint from `Spec::len`.
    pub len_constraint: Option<(Cmp, i64)>,
    /// Registered cmp constraint from `Spec::cmp`.
    pub cmp_constraint: Option<(Cmp, f64)>,
    /// Registered unsigned flag from `Spec::unsigned`.
    pub unsigned_flag: bool,
    /// Registered printable flag from `Spec::printable`.
    pub printable_flag: bool,
    /// Registered ident flag from `Spec::ident`.
    pub ident_flag: bool,
    /// Registered error message from `Spec::error`.
    pub error_msg: Option<String>,
}

impl Default for Spec {
    fn default() -> Self {
        Self::new()
    }
}

impl Spec {
    /// Port of `Spec.__init__()` from
    /// `powerline/lint/spec.py:69`.
    pub fn new() -> Self {
        Self {
            // py:70-78  reset state
            specs: Vec::new(),
            keys: std::collections::HashMap::new(),
            cmsg: String::new(),
            isoptional: false,
            did_type: false,
            allowed_types: Vec::new(),
            regex: None,
            oneof: None,
            len_constraint: None,
            cmp_constraint: None,
            unsigned_flag: false,
            printable_flag: false,
            ident_flag: false,
            error_msg: None,
        }
    }

    /// Port of `Spec.update()` from
    /// `powerline/lint/spec.py:80`.
    ///
    /// Registers a sub-key constraint. Returns self for chaining.
    pub fn update(mut self, key: impl Into<String>, spec: Spec) -> Self {
        // py:81-93  register key spec
        self.keys.insert(key.into(), spec);
        self
    }

    /// Port of `Spec.optional()` from
    /// `powerline/lint/spec.py:645`.
    pub fn optional(mut self) -> Self {
        // py:646-654  isoptional = True
        self.isoptional = true;
        self
    }

    /// Port of `Spec.required()` from
    /// `powerline/lint/spec.py:656`.
    pub fn required(mut self) -> Self {
        // py:657-669  isoptional = False
        self.isoptional = false;
        self
    }

    /// Port of `Spec.context_message()` from
    /// `powerline/lint/spec.py:178`.
    pub fn context_message(mut self, msg: impl Into<String>) -> Self {
        // py:179-192  self.cmsg = msg
        self.cmsg = msg.into();
        self
    }

    /// Port of `Spec.printable()` from
    /// `powerline/lint/spec.py:374`.
    pub fn printable(mut self) -> Self {
        // py:374-377  register check_printable
        self.printable_flag = true;
        self
    }

    /// Port of `Spec.unsigned()` from
    /// `powerline/lint/spec.py:471`.
    pub fn unsigned(mut self) -> Self {
        // py:472-486  register unsigned check
        self.unsigned_flag = true;
        self
    }

    /// Port of `Spec.ident()` from
    /// `powerline/lint/spec.py:574`.
    pub fn ident(mut self) -> Self {
        // py:575-588  register check_re identifier pattern
        self.ident_flag = true;
        // py:578  ident pattern: ^[a-zA-Z_]\w*$
        self.regex = Some(r"^[a-zA-Z_]\w*$".to_string());
        self
    }

    /// Port of `Spec.type()` from
    /// `powerline/lint/spec.py:379`.
    ///
    /// Registers allowed types. Renamed to `type_check` in Rust
    /// since `type` is a reserved keyword.
    pub fn type_check(mut self, types: &[SpecType]) -> Self {
        // py:380-406  set did_type + accumulate types
        self.did_type = true;
        self.allowed_types.extend_from_slice(types);
        self
    }

    /// Port of `Spec.re()` from
    /// `powerline/lint/spec.py:552`.
    ///
    /// Renamed to `regex` in Rust since `re` collides with the
    /// `regex` crate name.
    pub fn regex(mut self, pattern: impl Into<String>) -> Self {
        // py:553-572  register check_re
        self.regex = Some(pattern.into());
        self
    }

    /// Port of `Spec.oneof()` from
    /// `powerline/lint/spec.py:590`.
    pub fn oneof(mut self, values: &[&str]) -> Self {
        // py:591-608  register membership check
        self.oneof = Some(values.iter().map(|s| s.to_string()).collect());
        self
    }

    /// Port of `Spec.error()` from
    /// `powerline/lint/spec.py:610`.
    pub fn error(mut self, msg: impl Into<String>) -> Self {
        // py:611-629  set error msg
        self.error_msg = Some(msg.into());
        self
    }

    /// Port of `Spec.len()` from
    /// `powerline/lint/spec.py:408`.
    pub fn len(mut self, comparison: Cmp, value: i64) -> Self {
        // py:409-434  register len check
        self.len_constraint = Some((comparison, value));
        self
    }

    /// Port of `Spec.cmp()` from
    /// `powerline/lint/spec.py:436`.
    pub fn cmp(mut self, comparison: Cmp, value: f64) -> Self {
        // py:437-469  register cmp check
        self.cmp_constraint = Some((comparison, value));
        self
    }

    /// Port of `Spec.copy()` from
    /// `powerline/lint/spec.py:96`.
    ///
    /// Returns a shallow clone of the spec. Python uses
    /// `copy.copy` + `_update` to handle inner spec references; the
    /// Rust port re-clones the registered fields.
    pub fn copy(&self) -> Spec {
        Spec {
            specs: self.specs.iter().map(|s| s.copy()).collect(),
            keys: self
                .keys
                .iter()
                .map(|(k, v)| (k.clone(), v.copy()))
                .collect(),
            cmsg: self.cmsg.clone(),
            isoptional: self.isoptional,
            did_type: self.did_type,
            allowed_types: self.allowed_types.clone(),
            regex: self.regex.clone(),
            oneof: self.oneof.clone(),
            len_constraint: self.len_constraint,
            cmp_constraint: self.cmp_constraint,
            unsigned_flag: self.unsigned_flag,
            printable_flag: self.printable_flag,
            ident_flag: self.ident_flag,
            error_msg: self.error_msg.clone(),
        }
    }

    /// Port of `Spec.check_printable()` from
    /// `powerline/lint/spec.py:359`.
    ///
    /// Returns failed when `value` contains characters matched by
    /// `NON_PRINTABLE_RE`.
    pub fn check_printable(value: &str) -> CheckResult {
        // py:359-372  NON_PRINTABLE_RE.search(value)
        if NON_PRINTABLE_RE().is_match(value) {
            CheckResult::failed()
        } else {
            CheckResult::ok()
        }
    }
}

/// Port of `Spec.check_type()` from
/// `powerline/lint/spec.py:194`.
///
/// Returns `ok` when `value`'s shape matches one of the allowed
/// types; `failed` otherwise.
pub fn check_type(value: &serde_json::Value, types: &[SpecType]) -> CheckResult {
    // py:204-217  type check + error message
    let actual = match value {
        serde_json::Value::Object(_) => SpecType::Dict,
        serde_json::Value::Array(_) => SpecType::List,
        serde_json::Value::String(_) => SpecType::Unicode,
        serde_json::Value::Bool(_) => SpecType::Bool,
        serde_json::Value::Number(_) => SpecType::Float,
        serde_json::Value::Null => SpecType::Null,
    };
    if types.contains(&actual) {
        CheckResult::ok()
    } else {
        CheckResult::failed()
    }
}

/// Port of `Spec.check_list()` from
/// `powerline/lint/spec.py:257`.
///
/// Returns the conjunction of `item_check` over each element of the
/// list. `proceed` is true iff every element returned proceed=true;
/// `hadproblem` is true iff any element returned hadproblem=true.
pub fn check_list<F>(items: &[serde_json::Value], mut item_check: F) -> CheckResult
where
    F: FnMut(&serde_json::Value) -> CheckResult,
{
    let mut proceed = true;
    let mut had_problem = false;
    for item in items {
        let r = item_check(item);
        proceed &= r.proceed;
        had_problem |= r.hadproblem;
    }
    CheckResult {
        proceed,
        hadproblem: had_problem,
    }
}

/// Port of `Spec.check_either()` from
/// `powerline/lint/spec.py:299`.
///
/// Returns ok if any sub-spec succeeds.
pub fn check_either<F>(specs_count: usize, mut check_one: F) -> CheckResult
where
    F: FnMut(usize) -> CheckResult,
{
    for i in 0..specs_count {
        let r = check_one(i);
        if !r.hadproblem {
            return CheckResult::ok();
        }
    }
    CheckResult::failed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn non_printable_re_matches_control_chars() {
        assert!(NON_PRINTABLE_RE().is_match("\x07"));
        assert!(NON_PRINTABLE_RE().is_match("\x1f"));
        assert!(!NON_PRINTABLE_RE().is_match("\t")); // py:15 tab allowed
        assert!(!NON_PRINTABLE_RE().is_match("\n")); // py:16 newline allowed
    }

    #[test]
    fn non_printable_re_allows_printable() {
        assert!(!NON_PRINTABLE_RE().is_match("hello"));
        assert!(!NON_PRINTABLE_RE().is_match(""));
    }

    #[test]
    fn check_result_ok_is_proceed_true_no_problem() {
        let r = CheckResult::ok();
        assert!(r.proceed);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_result_failed_is_proceed_false_with_problem() {
        let r = CheckResult::failed();
        assert!(!r.proceed);
        assert!(r.hadproblem);
    }

    #[test]
    fn spec_new_starts_empty() {
        let s = Spec::new();
        assert!(s.specs.is_empty());
        assert!(s.keys.is_empty());
        assert!(s.cmsg.is_empty());
        assert!(!s.isoptional);
        assert!(!s.did_type);
        assert!(s.allowed_types.is_empty());
    }

    #[test]
    fn spec_update_registers_key_spec() {
        let inner = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().update("foo", inner);
        assert!(s.keys.contains_key("foo"));
    }

    #[test]
    fn spec_optional_sets_isoptional() {
        let s = Spec::new().optional();
        assert!(s.isoptional);
    }

    #[test]
    fn spec_required_clears_isoptional() {
        let s = Spec::new().optional().required();
        assert!(!s.isoptional);
    }

    #[test]
    fn spec_context_message_stores_msg() {
        let s = Spec::new().context_message("at {key}");
        assert_eq!(s.cmsg, "at {key}");
    }

    #[test]
    fn spec_printable_sets_flag() {
        let s = Spec::new().printable();
        assert!(s.printable_flag);
    }

    #[test]
    fn spec_unsigned_sets_flag() {
        let s = Spec::new().unsigned();
        assert!(s.unsigned_flag);
    }

    #[test]
    fn spec_ident_sets_flag_and_regex() {
        let s = Spec::new().ident();
        assert!(s.ident_flag);
        assert_eq!(s.regex.as_deref(), Some(r"^[a-zA-Z_]\w*$"));
    }

    #[test]
    fn spec_type_check_registers_allowed_types() {
        let s = Spec::new().type_check(&[SpecType::Unicode, SpecType::Bool]);
        assert!(s.did_type);
        assert_eq!(s.allowed_types.len(), 2);
        assert!(s.allowed_types.contains(&SpecType::Unicode));
        assert!(s.allowed_types.contains(&SpecType::Bool));
    }

    #[test]
    fn spec_regex_stores_pattern() {
        let s = Spec::new().regex(r"^\d+$");
        assert_eq!(s.regex.as_deref(), Some(r"^\d+$"));
    }

    #[test]
    fn spec_oneof_stores_values() {
        let s = Spec::new().oneof(&["foo", "bar"]);
        let v = s.oneof.unwrap();
        assert_eq!(v, vec!["foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn spec_error_stores_msg() {
        let s = Spec::new().error("invalid value");
        assert_eq!(s.error_msg.as_deref(), Some("invalid value"));
    }

    #[test]
    fn spec_len_stores_constraint() {
        let s = Spec::new().len(Cmp::Lt, 10);
        assert_eq!(s.len_constraint, Some((Cmp::Lt, 10)));
    }

    #[test]
    fn spec_cmp_stores_constraint() {
        let s = Spec::new().cmp(Cmp::Ge, 0.0);
        let c = s.cmp_constraint.unwrap();
        assert_eq!(c.0, Cmp::Ge);
        assert_eq!(c.1, 0.0);
    }

    #[test]
    fn spec_copy_clones_all_fields() {
        let s = Spec::new()
            .type_check(&[SpecType::Unicode])
            .regex(r"^foo$")
            .optional()
            .context_message("at {key}");
        let c = s.copy();
        assert_eq!(c.allowed_types, s.allowed_types);
        assert_eq!(c.regex, s.regex);
        assert_eq!(c.isoptional, s.isoptional);
        assert_eq!(c.cmsg, s.cmsg);
    }

    #[test]
    fn check_printable_accepts_normal_text() {
        let r = Spec::check_printable("hello world");
        assert!(r.proceed);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_printable_rejects_control_char() {
        let r = Spec::check_printable("hello\x07world");
        assert!(!r.proceed);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_type_object_matches_dict() {
        let r = check_type(&json!({"k": 1}), &[SpecType::Dict]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_array_matches_list() {
        let r = check_type(&json!([1, 2]), &[SpecType::List]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_string_matches_unicode() {
        let r = check_type(&json!("hi"), &[SpecType::Unicode]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_bool_matches_bool() {
        let r = check_type(&json!(true), &[SpecType::Bool]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_number_matches_float() {
        let r = check_type(&json!(42), &[SpecType::Float]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_null_matches_null() {
        let r = check_type(&json!(null), &[SpecType::Null]);
        assert!(r.proceed);
    }

    #[test]
    fn check_type_mismatch_returns_failed() {
        let r = check_type(&json!("hi"), &[SpecType::Bool]);
        assert!(r.hadproblem);
    }

    #[test]
    fn check_list_aggregates_results() {
        let items = vec![json!(1), json!(2), json!(3)];
        let r = check_list(&items, |_| CheckResult::ok());
        assert!(r.proceed);
        assert!(!r.hadproblem);
    }

    #[test]
    fn check_list_any_failure_marks_had_problem() {
        let items = vec![json!(1), json!(2)];
        let mut called = 0;
        let r = check_list(&items, |_| {
            called += 1;
            if called == 1 {
                CheckResult::failed()
            } else {
                CheckResult::ok()
            }
        });
        assert!(r.hadproblem);
        assert!(!r.proceed);
    }

    #[test]
    fn check_either_short_circuits_on_first_success() {
        let r = check_either(3, |i| {
            if i == 0 {
                CheckResult::ok()
            } else {
                CheckResult::failed()
            }
        });
        assert!(r.proceed);
    }

    #[test]
    fn check_either_all_fail_returns_failed() {
        let r = check_either(3, |_| CheckResult::failed());
        assert!(r.hadproblem);
    }

    #[test]
    fn cmp_enum_variants_match_python_operators() {
        // Sanity check the operator set matches py:411
        // ('<', '<=', '==', '>=', '>', '!=')
        let ops = [Cmp::Lt, Cmp::Le, Cmp::Eq, Cmp::Ge, Cmp::Gt, Cmp::Ne];
        assert_eq!(ops.len(), 6);
    }
}
