// vim:fileencoding=utf-8:noet
//! Port of `powerline/lint/inspect.py`.
//!
//! Argument-spec introspection used by the linter to figure out which
//! kwargs a segment function accepts. Upstream `getconfigargspec`
//! relies on Python's `inspect.getfullargspec` — a runtime
//! reflection primitive Rust doesn't have. The lint pipeline will
//! need a static segment-registry pre-built at compile time when
//! the full lint port lands.
//!
//! This first chunk ports the data-transformation helper
//! `formatconfigargspec` (pure string assembly with no introspection).
//! `getconfigargspec` is stubbed and returns an empty FullArgSpec
//! until the static segment-registry is wired.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from inspect import FullArgSpec, getfullargspec                                          // py:4
// from itertools import zip_longest                                                         // py:5
// from powerline.segments import Segment                                                    // py:7

/// Port of `FullArgSpec` namedtuple (from `inspect`) used by
/// `getconfigargspec`.
///
/// Python carries `args`, `varargs`, `varkw`, `defaults`, `kwonlyargs`,
/// `kwonlydefaults`, `annotations`. The Rust port only carries the
/// fields `formatconfigargspec` reads.
#[derive(Debug, Clone, Default)]
pub struct FullArgSpec {
    pub args: Vec<String>,
    pub varargs: Option<String>,
    pub varkw: Option<String>,
    pub defaults: Vec<String>, // formatted via formatvalue
    pub kwonlyargs: Vec<String>,
}

/// Port of `getconfigargspec()` from `powerline/lint/inspect.py:10`.
///
/// **Status:** stub returning an empty `FullArgSpec`. Faithful port
/// requires either Python embedding (to call `inspect.getfullargspec`
/// at lint time) or a compile-time static registry of every segment
/// function and its kwargs. The static-registry path is the
/// powerliners endgame; see PORT_PLAN.md Phase 5.
pub fn getconfigargspec(_obj_name: &str) -> FullArgSpec {
    // py:11-65  introspect via inspect.getfullargspec — deferred
    FullArgSpec::default()
}

/// Port of `formatconfigargspec()` from
/// `powerline/lint/inspect.py:69`.
///
/// Format an argument spec from the values returned by
/// `getconfigargspec`.
///
/// This is a specialised replacement for `inspect.formatargspec`,
/// which has been deprecated since Python 3.5 and removed in 3.11.
/// It supports valid values for args, defaults and formatvalue; all
/// other parameters are expected to be either empty or None.
///
/// Python signature:
/// ```python
/// def formatconfigargspec(args, varargs=None, varkw=None, defaults=None,
///                         kwonlyargs=(), kwonlydefaults={}, annotations={},
///                         formatvalue=lambda value: '=' + repr(value)):
/// ```
///
/// Rust simplification: `varargs`, `varkw`, `kwonlyargs`,
/// `kwonlydefaults`, `annotations` are required-empty per Python's
/// own assertions at py:81-85 — collapsed into a single signature.
pub fn formatconfigargspec(args: &[String], defaults: &[String]) -> String {
    // py:81-85  assertion block — empty varargs/varkw/kwonlyargs/etc
    // py:87  specs = []
    let mut specs: Vec<String> = Vec::new();
    // py:88-89  firstdefault = len(args) - len(defaults) (only when defaults non-empty)
    let firstdefault = if !defaults.is_empty() {
        Some(args.len().saturating_sub(defaults.len()))
    } else {
        None
    };
    // py:90-93  iterate args, append '=value' to those at index ≥ firstdefault
    for (i, arg) in args.iter().enumerate() {
        let mut spec = arg.clone();
        if let Some(fd) = firstdefault {
            if i >= fd {
                // py:92  spec += formatvalue(defaults[i - firstdefault])
                spec.push('=');
                spec.push_str(&defaults[i - fd]);
            }
        }
        specs.push(spec);
    }
    // py:94  return '(' + ', '.join(specs) + ')'
    format!("({})", specs.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatconfigargspec_empty_args() {
        assert_eq!(formatconfigargspec(&[], &[]), "()");
    }

    #[test]
    fn formatconfigargspec_args_without_defaults() {
        let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(formatconfigargspec(&args, &[]), "(a, b, c)");
    }

    #[test]
    fn formatconfigargspec_args_with_trailing_defaults() {
        // 3 args, 2 defaults → b='1', c='2'
        let args = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let defaults = vec!["1".to_string(), "2".to_string()];
        assert_eq!(formatconfigargspec(&args, &defaults), "(a, b=1, c=2)");
    }

    #[test]
    fn formatconfigargspec_all_args_default() {
        let args = vec!["x".to_string(), "y".to_string()];
        let defaults = vec!["10".to_string(), "20".to_string()];
        assert_eq!(formatconfigargspec(&args, &defaults), "(x=10, y=20)");
    }

    #[test]
    fn formatconfigargspec_one_arg_one_default() {
        let args = vec!["only".to_string()];
        let defaults = vec!["'hello'".to_string()];
        assert_eq!(formatconfigargspec(&args, &defaults), "(only='hello')");
    }

    #[test]
    fn getconfigargspec_returns_empty_until_registry_lands() {
        let spec = getconfigargspec("dummy");
        assert!(spec.args.is_empty());
        assert!(spec.defaults.is_empty());
    }
}
