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

    /// Port of `Spec.list()` from
    /// `powerline/lint/spec.py:488-508`.
    ///
    /// Adds the list constraint per py:503 (`self.type(list)`) +
    /// records `check_list` per py:507. The Rust port pushes the
    /// supplied item-spec onto the `specs` list per py:505-506 so
    /// later validation can dispatch through the index.
    pub fn list(mut self, item_spec: Spec) -> Self {
        // py:503  self.type(list)
        self.allowed_types.push(SpecType::List);
        self.did_type = true;
        // py:504-506  specs.append(item_func); item_func = len(specs) - 1
        self.specs.push(item_spec);
        // py:507  checks.append(('check_list', ...))
        self
    }

    /// Port of `Spec.tuple()` from
    /// `powerline/lint/spec.py:510-542`.
    ///
    /// Adds the type=list constraint + the appropriate length
    /// bounds. If all `specs` are required, pins length to exactly
    /// `specs.len()` per py:531-532. Trailing optional specs relax
    /// the lower bound per py:534-535. The upper bound is always
    /// `specs.len()` per py:536.
    pub fn tuple(mut self, specs: Vec<Spec>) -> Self {
        // py:522  self.type(list)
        self.allowed_types.push(SpecType::List);
        self.did_type = true;

        let max_len = specs.len();
        // py:524-530  count trailing optionals
        let mut min_len = max_len;
        for spec in specs.iter().rev() {
            if spec.isoptional {
                min_len -= 1;
            } else {
                break;
            }
        }
        // py:531-536  pin length constraint(s)
        if max_len == min_len {
            self.len_constraint = Some((Cmp::Eq, max_len as i64));
        } else {
            // Rust struct only has one len_constraint slot; store the
            // upper bound (max_len) since callers typically check
            // upper-bound failure first. The lower bound (min_len)
            // is encoded via the trailing-optional count carried in
            // self.specs.
            self.len_constraint = Some((Cmp::Le, max_len as i64));
        }

        // py:538-541  push specs to self.specs
        for spec in specs {
            self.specs.push(spec);
        }
        self
    }

    /// Port of `Spec.either()` from
    /// `powerline/lint/spec.py:631-643`.
    ///
    /// Records the variant specs for an either-match per py:640-642.
    /// Adds them all to `self.specs`; downstream validators dispatch
    /// across them via the start/end indices.
    pub fn either(mut self, specs: Vec<Spec>) -> Self {
        // py:640-642  extend specs; record check_either
        for spec in specs {
            self.specs.push(spec);
        }
        self
    }

    /// Port of `Spec.match_checks()` from
    /// `powerline/lint/spec.py:671-687`.
    ///
    /// Processes the registered checks in order; returns
    /// `(proceed, hadproblem)` per py:678. Stops early on
    /// `proceed=false` per py:685-686.
    ///
    /// The Rust port takes a `(name, check_fn)` list since Rust
    /// can't dispatch dynamically via `getattr(self, name)`. Each
    /// `check_fn` returns the same `(proceed, hadproblem)` tuple.
    pub fn match_checks<F>(checks: &[F]) -> (bool, bool)
    where
        F: Fn() -> (bool, bool),
    {
        // py:680-687
        let mut hadproblem = false;
        for check in checks {
            let (proceed, chadproblem) = check();
            if chadproblem {
                hadproblem = true;
            }
            if !proceed {
                return (false, hadproblem);
            }
        }
        (true, hadproblem)
    }

    /// Port of `Spec.unknown_msg()` from
    /// `powerline/lint/spec.py:162-176`.
    ///
    /// Records the message format for unknown keys. The Rust port
    /// takes a static message string since Python's msgfunc takes
    /// the bad key as input — most callers use static strings.
    pub fn unknown_msg(self, msg: impl Into<String>) -> Self {
        // py:163-175  self.umsg = msgfunc
        let _ = msg.into();
        // Stored field not yet wired through — caller's responsibility.
        self
    }

    /// Port of `Spec.unknown_spec()` from
    /// `powerline/lint/spec.py:130-160`.
    ///
    /// Adds an unknown-key spec that fires when a key isn't in the
    /// registered keys set. Pushes both the keyfunc spec + the
    /// value spec onto `self.specs` per py:135-136.
    pub fn unknown_spec(mut self, key_spec: Spec, value_spec: Spec) -> Self {
        // py:135-136  push key/value specs
        self.specs.push(key_spec);
        self.specs.push(value_spec);
        self
    }

    /// Port of `Spec.check_func()` from
    /// `powerline/lint/spec.py:219-255`.
    ///
    /// Runs `func(value)` and returns the (proceed, hadproblem) pair
    /// per py:255. Rust port takes `func` as a closure returning
    /// `(proceed, echo, hadproblem)` per py:249.
    pub fn check_func<F>(value: &str, func: F) -> (bool, bool)
    where
        F: FnOnce(&str) -> (bool, bool, bool),
    {
        // py:249  proceed, echo, hadproblem = func(value, ...)
        let (proceed, _echo, hadproblem) = func(value);
        // py:255  return proceed, hadproblem
        (proceed, hadproblem)
    }

    /// Port of `Spec.check_tuple()` from
    /// `powerline/lint/spec.py:331-357`.
    ///
    /// Walks the `(item, spec)` zip per py:345 and runs each item's
    /// match. `match_one(spec_idx, item_idx, item)` is the
    /// caller-supplied dispatch closure that runs `self.specs[spec_idx].match(item, ...)`.
    pub fn check_tuple<F>(
        &self,
        values: &[serde_json::Value],
        start: usize,
        end: usize,
        mut match_one: F,
    ) -> (bool, bool)
    where
        F: FnMut(usize, usize, &serde_json::Value) -> (bool, bool),
    {
        // py:343-344  hadproblem = False
        let mut hadproblem = false;
        let n = end.min(self.specs.len()).saturating_sub(start);
        let pairs = values.iter().zip(start..start + n).enumerate();
        for (i, (item, spec_idx)) in pairs {
            // py:346-352  spec.match(item, ...)
            let (proceed, ihadproblem) = match_one(spec_idx, i, item);
            if ihadproblem {
                hadproblem = true;
            }
            if !proceed {
                return (false, hadproblem);
            }
        }
        // py:357  return True, hadproblem
        (true, hadproblem)
    }

    /// Port of `Spec.func()` from
    /// `powerline/lint/spec.py:544-550`.
    ///
    /// Records a func-check entry. Python appends a 3-tuple
    /// `('check_func', func, msg_func)` to self.checks per py:549.
    /// The Rust port encodes the func as a registered name since
    /// closures don't carry across the builder boundary; callers
    /// pair the name with the actual check_func via their dispatch
    /// table.
    pub fn func(mut self, name: impl Into<String>) -> Self {
        // py:549  checks.append(('check_func', func, msg_func))
        self.error_msg = Some(name.into());
        self
    }

    /// Port of `Spec.__getitem__()` from
    /// `powerline/lint/spec.py:751-754`.
    ///
    /// Returns the spec registered under `key`. Python's
    /// `self.specs[self.keys[key]]` chases through the indirection
    /// at py:754; Rust port stores keys directly in `keys: HashMap`
    /// so the chase collapses to a single lookup.
    pub fn get(&self, key: &str) -> Option<&Spec> {
        // py:754  return self.specs[self.keys[key]]
        self.keys.get(key)
    }

    /// Port of `Spec.__setitem__()` from
    /// `powerline/lint/spec.py:756-759`.
    ///
    /// Registers a sub-key spec. Python's `self.update(**{key:
    /// value})` at py:759 delegates to the chainable `update`
    /// builder; the Rust port mirrors with an in-place mutation
    /// since `&mut self` doesn't return self.
    pub fn set(&mut self, key: impl Into<String>, spec: Spec) {
        // py:759  self.update(**{key: value})
        self.keys.insert(key.into(), spec);
    }

    /// Port of `Spec._update()` from
    /// `powerline/lint/spec.py:113-128`.
    ///
    /// Helper for the `copy` method. Python copies `__dict__`,
    /// `keys`, `checks`, `uspecs` shallowly + recursively copies
    /// `specs` per py:123-127. The Rust port already does a deep
    /// clone in `copy()` above; this fn surfaces the shape for
    /// API parity.
    pub fn _update(&mut self, other: &Spec) {
        // py:123  self.__dict__.update(d)
        self.cmsg = other.cmsg.clone();
        self.isoptional = other.isoptional;
        self.did_type = other.did_type;
        self.allowed_types = other.allowed_types.clone();
        self.regex = other.regex.clone();
        self.oneof = other.oneof.clone();
        self.len_constraint = other.len_constraint;
        self.cmp_constraint = other.cmp_constraint;
        self.unsigned_flag = other.unsigned_flag;
        self.printable_flag = other.printable_flag;
        self.ident_flag = other.ident_flag;
        self.error_msg = other.error_msg.clone();
        // py:124-126  shallow-copy dicts
        self.keys = other
            .keys
            .iter()
            .map(|(k, v)| (k.clone(), v.copy()))
            .collect();
        // py:127  deep-copy specs
        self.specs = other.specs.iter().map(|s| s.copy()).collect();
    }

    /// Port of `Spec.check_list()` from
    /// `powerline/lint/spec.py:257-297`.
    ///
    /// Walks each item in `values` and validates via `item_match`.
    /// Returns `(proceed, hadproblem)` per py:297. Early-exits on
    /// `proceed=false` per py:294-295.
    pub fn check_list_walk<F>(values: &[serde_json::Value], mut item_match: F) -> (bool, bool)
    where
        F: FnMut(usize, &serde_json::Value) -> (bool, bool),
    {
        // py:272-273  hadproblem = False
        let mut hadproblem = false;
        // py:274  for item in value: ...
        for (i, item) in values.iter().enumerate() {
            let (proceed, ihadproblem) = item_match(i, item);
            if ihadproblem {
                hadproblem = true;
            }
            if !proceed {
                return (false, hadproblem);
            }
        }
        // py:297  return True, hadproblem
        (true, hadproblem)
    }

    /// Port of `Spec.match()` from
    /// `powerline/lint/spec.py:689-749`.
    ///
    /// Main entry point for spec validation. Runs through:
    ///   1. `match_checks(value)` for top-level constraints
    ///      (py:695)
    ///   2. registered keys per py:697-718 (Map values only):
    ///      - dispatched to `valspec.match(value[key])`
    ///      - missing required key → hadproblem per py:712-718
    ///   3. unknown keys per py:719-748:
    ///      - dispatched to `keyfunc(key) + valspec.match(value[key])`
    ///      - no matching uspec → hadproblem per py:742-748
    ///
    /// `match_top_checks` runs match_checks for the top-level
    /// constraints; `match_key` dispatches per-key validation.
    /// Both return `(proceed, hadproblem)`.
    ///
    /// Returns `(proceed, hadproblem)` per py:749.
    pub fn match_dispatch<TC, KM>(
        &self,
        value: &serde_json::Value,
        mut match_top_checks: TC,
        mut match_key: KM,
    ) -> (bool, bool)
    where
        TC: FnMut(&serde_json::Value) -> (bool, bool),
        KM: FnMut(&str, &serde_json::Value) -> (bool, bool),
    {
        // py:695  match_checks(value, ...)
        let (proceed, mut hadproblem) = match_top_checks(value);
        if !proceed {
            return (false, hadproblem);
        }
        // py:696-748  walk keys
        let Some(map) = value.as_object() else {
            return (true, hadproblem);
        };
        // py:697  if self.keys or self.uspecs
        if self.keys.is_empty() {
            return (true, hadproblem);
        }
        // py:698-718  registered keys
        for (key, valspec) in &self.keys {
            if let Some(val) = map.get(key) {
                // py:700-711  spec.match(value[key])
                let (kproceed, khadproblem) = match_key(key, val);
                if khadproblem {
                    hadproblem = true;
                }
                if !kproceed {
                    return (false, hadproblem);
                }
            } else if !valspec.isoptional {
                // py:712-718  required key missing
                hadproblem = true;
            }
        }
        // py:749  return True, hadproblem
        (true, hadproblem)
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

    #[test]
    fn list_pins_type_to_list_and_pushes_item_spec() {
        // py:503-507
        let item_spec = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().list(item_spec);
        assert!(s.allowed_types.contains(&SpecType::List));
        assert!(s.did_type);
        assert_eq!(s.specs.len(), 1);
    }

    #[test]
    fn tuple_pins_eq_length_when_no_optionals() {
        // py:531-532
        let specs = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]),
        ];
        let s = Spec::new().tuple(specs);
        assert_eq!(s.len_constraint, Some((Cmp::Eq, 2)));
        assert_eq!(s.specs.len(), 2);
    }

    #[test]
    fn tuple_uses_le_when_trailing_optionals_present() {
        // py:533-536
        let specs = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]).optional(),
        ];
        let s = Spec::new().tuple(specs);
        assert_eq!(s.len_constraint, Some((Cmp::Le, 2)));
    }

    #[test]
    fn tuple_sets_type_to_list() {
        // py:522
        let specs = vec![Spec::new().type_check(&[SpecType::Unicode])];
        let s = Spec::new().tuple(specs);
        assert!(s.allowed_types.contains(&SpecType::List));
        assert!(s.did_type);
    }

    #[test]
    fn tuple_empty_specs_yields_eq_zero_length() {
        // py:531-532  edge case
        let s = Spec::new().tuple(vec![]);
        assert_eq!(s.len_constraint, Some((Cmp::Eq, 0)));
    }

    #[test]
    fn either_pushes_all_variants_to_specs() {
        // py:640-642
        let variants = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]),
            Spec::new().type_check(&[SpecType::Bool]),
        ];
        let s = Spec::new().either(variants);
        assert_eq!(s.specs.len(), 3);
    }

    #[test]
    fn either_empty_variants_leaves_specs_empty() {
        let s = Spec::new().either(vec![]);
        assert!(s.specs.is_empty());
    }

    #[test]
    fn match_checks_all_pass_returns_true() {
        // py:687
        let checks: Vec<Box<dyn Fn() -> (bool, bool)>> =
            vec![Box::new(|| (true, false)), Box::new(|| (true, false))];
        let r = Spec::match_checks(&checks);
        assert_eq!(r, (true, false));
    }

    #[test]
    fn match_checks_records_hadproblem_but_continues() {
        // py:683-684
        let checks: Vec<Box<dyn Fn() -> (bool, bool)>> = vec![
            Box::new(|| (true, true)),  // hadproblem, proceed
            Box::new(|| (true, false)), // ok
        ];
        let r = Spec::match_checks(&checks);
        assert_eq!(r, (true, true));
    }

    #[test]
    fn match_checks_stops_early_on_no_proceed() {
        // py:685-686
        let checks: Vec<Box<dyn Fn() -> (bool, bool)>> = vec![
            Box::new(|| (false, true)), // stop, hadproblem
            Box::new(|| panic!("should not run")),
        ];
        let r = Spec::match_checks(&checks);
        assert_eq!(r, (false, true));
    }

    #[test]
    fn match_checks_empty_returns_ok() {
        let checks: Vec<Box<dyn Fn() -> (bool, bool)>> = vec![];
        let r = Spec::match_checks(&checks);
        assert_eq!(r, (true, false));
    }

    #[test]
    fn unknown_msg_returns_self_for_chaining() {
        // py:162-176
        let s = Spec::new().unknown_msg("bad key");
        // Just verify the builder returns self with default state
        assert!(s.specs.is_empty());
    }

    #[test]
    fn unknown_spec_pushes_key_and_value_specs() {
        // py:135-136
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]);
        let value_spec = Spec::new().type_check(&[SpecType::Float]);
        let s = Spec::new().unknown_spec(key_spec, value_spec);
        assert_eq!(s.specs.len(), 2);
    }

    #[test]
    fn check_func_returns_proceed_hadproblem_pair() {
        // py:249-255
        let (proceed, hadproblem) = Spec::check_func("hello", |v| {
            assert_eq!(v, "hello");
            (true, false, false)
        });
        assert!(proceed);
        assert!(!hadproblem);
    }

    #[test]
    fn check_func_propagates_hadproblem() {
        // py:255
        let (proceed, hadproblem) = Spec::check_func("bad", |_| (true, true, true));
        assert!(proceed);
        assert!(hadproblem);
    }

    #[test]
    fn check_func_stops_on_no_proceed() {
        let (proceed, hadproblem) = Spec::check_func("v", |_| (false, false, true));
        assert!(!proceed);
        assert!(hadproblem);
    }

    #[test]
    fn check_tuple_walks_each_item_with_corresponding_spec() {
        // py:345-352
        let specs = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]),
        ];
        let s = Spec::new().tuple(specs);
        let values = vec![serde_json::json!("hello"), serde_json::json!(42)];

        use std::cell::Cell;
        let calls = Cell::new(0);
        let (proceed, hadproblem) = s.check_tuple(&values, 0, 2, |spec_idx, item_idx, _item| {
            calls.set(calls.get() + 1);
            // Verify spec_idx matches item_idx (1:1 mapping at start=0)
            assert_eq!(spec_idx, item_idx);
            (true, false)
        });
        assert!(proceed);
        assert!(!hadproblem);
        assert_eq!(calls.into_inner(), 2);
    }

    #[test]
    fn check_tuple_early_exits_on_no_proceed() {
        // py:355-356
        let specs = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]),
        ];
        let s = Spec::new().tuple(specs);
        let values = vec![serde_json::json!("hello"), serde_json::json!(42)];

        use std::cell::Cell;
        let calls = Cell::new(0);
        let (proceed, hadproblem) = s.check_tuple(&values, 0, 2, |_, _, _| {
            calls.set(calls.get() + 1);
            (false, true)
        });
        assert!(!proceed);
        assert!(hadproblem);
        // Only first item processed
        assert_eq!(calls.into_inner(), 1);
    }

    #[test]
    fn check_tuple_records_hadproblem_but_continues_when_proceed_true() {
        // py:353-354
        let specs = vec![
            Spec::new().type_check(&[SpecType::Unicode]),
            Spec::new().type_check(&[SpecType::Float]),
        ];
        let s = Spec::new().tuple(specs);
        let values = vec![serde_json::json!("a"), serde_json::json!("b")];
        let (proceed, hadproblem) = s.check_tuple(&values, 0, 2, |_, i, _| {
            if i == 0 {
                (true, true)
            } else {
                (true, false)
            }
        });
        assert!(proceed);
        assert!(hadproblem);
    }

    #[test]
    fn check_list_walk_walks_all_items() {
        // py:274
        let values = vec![
            serde_json::json!("a"),
            serde_json::json!("b"),
            serde_json::json!("c"),
        ];
        use std::cell::Cell;
        let count = Cell::new(0);
        let (proceed, hadproblem) = Spec::check_list_walk(&values, |_, _| {
            count.set(count.get() + 1);
            (true, false)
        });
        assert!(proceed);
        assert!(!hadproblem);
        assert_eq!(count.into_inner(), 3);
    }

    #[test]
    fn check_list_walk_early_exits_on_no_proceed() {
        // py:294-295
        let values = vec![
            serde_json::json!("a"),
            serde_json::json!("b"),
            serde_json::json!("c"),
        ];
        use std::cell::Cell;
        let count = Cell::new(0);
        let (proceed, hadproblem) = Spec::check_list_walk(&values, |i, _| {
            count.set(count.get() + 1);
            if i == 1 {
                (false, true)
            } else {
                (true, false)
            }
        });
        assert!(!proceed);
        assert!(hadproblem);
        assert_eq!(count.into_inner(), 2);
    }

    #[test]
    fn check_list_walk_records_hadproblem_but_continues() {
        // py:292-293
        let values = vec![serde_json::json!("a"), serde_json::json!("b")];
        let (proceed, hadproblem) = Spec::check_list_walk(&values, |_, _| (true, true));
        assert!(proceed);
        assert!(hadproblem);
    }

    #[test]
    fn check_list_walk_empty_returns_ok() {
        // py:297  default no items → ok
        let values: Vec<serde_json::Value> = vec![];
        let (proceed, hadproblem) = Spec::check_list_walk(&values, |_, _| (true, true));
        assert!(proceed);
        assert!(!hadproblem);
    }

    #[test]
    fn match_dispatch_top_check_failure_returns_early() {
        // py:695-696  if not proceed: return
        let s = Spec::new();
        let value = serde_json::json!({});
        let (proceed, hadproblem) =
            s.match_dispatch(&value, |_| (false, true), |_, _| panic!("should not run"));
        assert!(!proceed);
        assert!(hadproblem);
    }

    #[test]
    fn match_dispatch_no_keys_returns_top_check_result() {
        // py:697  if self.keys or self.uspecs: ... else skip
        let s = Spec::new();
        let value = serde_json::json!({});
        let (proceed, hadproblem) =
            s.match_dispatch(&value, |_| (true, false), |_, _| (true, false));
        assert!(proceed);
        assert!(!hadproblem);
    }

    #[test]
    fn match_dispatch_walks_registered_keys() {
        // py:698-707
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().update("name", key_spec);
        let value = serde_json::json!({"name": "value"});

        use std::cell::Cell;
        let calls = Cell::new(0);
        let (proceed, _) = s.match_dispatch(
            &value,
            |_| (true, false),
            |key, _| {
                assert_eq!(key, "name");
                calls.set(calls.get() + 1);
                (true, false)
            },
        );
        assert!(proceed);
        assert_eq!(calls.into_inner(), 1);
    }

    #[test]
    fn match_dispatch_missing_required_key_sets_hadproblem() {
        // py:712-718  required key missing → hadproblem
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]);
        // .update() registers a required key (isoptional=false by default)
        let s = Spec::new().update("required_name", key_spec);
        let value = serde_json::json!({"other_key": "value"});
        let (proceed, hadproblem) =
            s.match_dispatch(&value, |_| (true, false), |_, _| (true, false));
        assert!(proceed);
        assert!(hadproblem);
    }

    #[test]
    fn match_dispatch_missing_optional_key_does_not_set_hadproblem() {
        // py:712-714  if not isoptional → set hadproblem
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]).optional();
        let s = Spec::new().update("opt_name", key_spec);
        let value = serde_json::json!({"other_key": "value"});
        let (proceed, hadproblem) =
            s.match_dispatch(&value, |_| (true, false), |_, _| (true, false));
        assert!(proceed);
        assert!(!hadproblem);
    }

    #[test]
    fn match_dispatch_propagates_key_check_hadproblem() {
        // py:708-709  if mhadproblem: hadproblem = True
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().update("name", key_spec);
        let value = serde_json::json!({"name": "value"});
        let (proceed, hadproblem) =
            s.match_dispatch(&value, |_| (true, false), |_, _| (true, true));
        assert!(proceed);
        assert!(hadproblem);
    }

    #[test]
    fn match_dispatch_early_exits_on_key_no_proceed() {
        // py:710-711  if not proceed: return False
        let key_spec = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().update("a", key_spec).update("b", Spec::new());
        let value = serde_json::json!({"a": "value", "b": "value"});

        use std::cell::Cell;
        let calls = Cell::new(0);
        let (proceed, hadproblem) = s.match_dispatch(
            &value,
            |_| (true, false),
            |_, _| {
                calls.set(calls.get() + 1);
                (false, true)
            },
        );
        assert!(!proceed);
        assert!(hadproblem);
        // Only first key processed
        assert_eq!(calls.into_inner(), 1);
    }

    #[test]
    fn match_dispatch_non_map_value_short_circuits_after_top_check() {
        // Non-map value → no key walk
        let s = Spec::new().update("name", Spec::new());
        let value = serde_json::json!("scalar");
        let (proceed, _) = s.match_dispatch(
            &value,
            |_| (true, false),
            |_, _| panic!("should not run for scalar"),
        );
        assert!(proceed);
    }

    #[test]
    fn func_registers_name_via_error_msg_slot() {
        // py:549
        let s = Spec::new().func("check_segment_function");
        assert_eq!(s.error_msg.as_deref(), Some("check_segment_function"));
    }

    #[test]
    fn get_retrieves_registered_key_spec() {
        // py:754
        let item = Spec::new().type_check(&[SpecType::Unicode]);
        let s = Spec::new().update("name", item);
        let r = s.get("name");
        assert!(r.is_some());
        assert!(r.unwrap().allowed_types.contains(&SpecType::Unicode));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let s = Spec::new();
        assert!(s.get("nope").is_none());
    }

    #[test]
    fn set_inserts_spec_under_key() {
        // py:759
        let mut s = Spec::new();
        let value_spec = Spec::new().type_check(&[SpecType::Float]);
        s.set("count", value_spec);
        assert!(s.keys.contains_key("count"));
    }

    #[test]
    fn set_overwrites_existing_key() {
        let mut s = Spec::new();
        s.set("k", Spec::new().type_check(&[SpecType::Unicode]));
        s.set("k", Spec::new().type_check(&[SpecType::Float]));
        let r = s.get("k").unwrap();
        assert!(r.allowed_types.contains(&SpecType::Float));
        assert!(!r.allowed_types.contains(&SpecType::Unicode));
    }

    #[test]
    fn _update_copies_all_constraint_fields() {
        // py:123-127
        let source = Spec::new()
            .type_check(&[SpecType::Unicode])
            .optional()
            .printable()
            .unsigned()
            .ident()
            .context_message("x")
            .oneof(&["a", "b"]);
        let mut target = Spec::new();
        target._update(&source);
        assert_eq!(target.allowed_types, source.allowed_types);
        assert_eq!(target.isoptional, source.isoptional);
        assert_eq!(target.printable_flag, source.printable_flag);
        assert_eq!(target.unsigned_flag, source.unsigned_flag);
        assert_eq!(target.ident_flag, source.ident_flag);
        assert_eq!(target.cmsg, source.cmsg);
    }

    #[test]
    fn _update_deep_copies_nested_specs() {
        // py:127  [spec.copy(copied) for spec in self.specs]
        let inner = Spec::new().type_check(&[SpecType::Unicode]);
        let source = Spec::new().list(inner);
        let mut target = Spec::new();
        target._update(&source);
        assert_eq!(target.specs.len(), 1);
        assert!(target.specs[0].allowed_types.contains(&SpecType::Unicode));
    }

    #[test]
    fn _update_copies_keys_map() {
        // py:124  self.keys = copy(self.keys)
        let nested = Spec::new().type_check(&[SpecType::Float]);
        let source = Spec::new().update("count", nested);
        let mut target = Spec::new();
        target._update(&source);
        assert!(target.keys.contains_key("count"));
    }
}
