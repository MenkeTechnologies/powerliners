// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/ipython.py`.
//!
//! Upstream source in full:
//!
//! ```python
//! # vim:fileencoding=utf-8:noet
//! from __future__ import (unicode_literals, division, absolute_import, print_function)
//!
//! from powerline.theme import requires_segment_info
//!
//! @requires_segment_info
//! def prompt_count(pl, segment_info):
//!     return str(segment_info['ipython'].prompt_count)
//! ```
//!
//! Renders the IPython input-cell counter (`In [N]:`) as a powerline
//! segment. The `@requires_segment_info` decorator marks the segment
//! so the renderer knows to pass it the live `segment_info` payload
//! (which under IPython holds the active prompt manager).

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

// py:4  from powerline.theme import requires_segment_info
// `requires_segment_info` lives in `powerline/theme.py` and is currently
// unported (theme.rs is a scaffold stub). When ported, it will be a
// marker attribute / trait that the renderer reads via reflection.
// For now, the decoration is implicit: the function below ports the
// undecorated body, and the requires_segment_info marker will be
// reattached when theme.rs lands its real port.

use std::collections::HashMap;

/// Port of `prompt_count()` from `powerline/segments/ipython.py:8`.
///
/// Python:
/// ```python
/// @requires_segment_info
/// def prompt_count(pl, segment_info):
///     return str(segment_info['ipython'].prompt_count)
/// ```
///
/// `pl` is the powerline-status logger (unused here but kept per the
/// Python signature). `segment_info` is the per-render payload dict;
/// under IPython it contains an `'ipython'` key whose value is an
/// object exposing `.prompt_count` (the current input cell number).
///
/// The Rust port currently takes `segment_info` as a `&HashMap` keyed
/// by string. The `ipython` entry's value type is the IPython
/// integration shape; here we model the prompt-count read as a u64
/// lookup in a parallel `prompt_counts` map for stand-alone testing.
/// A faithful port will land alongside the IPython binding ports.
pub fn prompt_count(_pl: &(), segment_info: &HashMap<String, IpythonPromptInfo>) -> Option<String> {
    // py:9  return str(segment_info['ipython'].prompt_count)
    segment_info
        .get("ipython")
        .map(|info| info.prompt_count.to_string())
}

/// Minimal mirror of the IPython prompt-info shape used by `prompt_count`.
///
/// Python's segment_info['ipython'] is the live IPython
/// `PromptManager` instance, exposing `.prompt_count`. The Rust port
/// models the read surface with a small struct so call sites can
/// construct a test payload without depending on the IPython binding.
///
/// # WARNING: NOT IN segments/ipython.py — Rust-only test shim
///
/// Python uses duck-typing: `segment_info['ipython'].prompt_count`
/// works on any object with a `prompt_count` attribute. Rust needs an
/// explicit type. This struct is the minimum required to compile
/// `prompt_count`; the real IPython binding will provide the full
/// `PromptManager` mirror when `bindings/ipython/*` is ported.
pub struct IpythonPromptInfo {
    pub prompt_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// prompt_count reads from segment_info['ipython'].prompt_count.
    #[test]
    fn prompt_count_returns_str_of_count() {
        let mut info = HashMap::new();
        info.insert(
            "ipython".to_string(),
            IpythonPromptInfo { prompt_count: 42 },
        );
        assert_eq!(prompt_count(&(), &info).as_deref(), Some("42"));
    }

    /// Missing 'ipython' key returns None (Python KeyError → Option).
    #[test]
    fn prompt_count_missing_key_returns_none() {
        let info: HashMap<String, IpythonPromptInfo> = HashMap::new();
        assert_eq!(prompt_count(&(), &info), None);
    }
}
