// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/vim/__init__.py`.
//!
//! Vim integration bindings. Upstream is a 482-LOC Python module that
//! talks to vim's embedded Python interpreter (`import vim`) and
//! exposes helpers for matchers/segments to query buffer state.
//!
//! Rust analog: powerliners has no equivalent to vim's embedded
//! Python; the matching pieces would need to talk to nvim via its
//! MessagePack RPC (the `neovim` Rust crate) or to vim via its
//! channel protocol. Until that integration lands, this module
//! exposes the data-shape callable stubs that matchers/segments need
//! so the dependency graph compiles.
//!
//! Matcher info shape: powerline passes a `dict` to matchers carrying
//! `bufnr`, `window`, `winnr`, etc. The Rust port models it as
//! `MatcherInfo` — a typed struct that callers populate.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

/// Per-buffer info passed to matchers.
///
/// Mirrors the `segment_info` / `matcher_info` dict shape powerline
/// builds in its vim binding. The Rust port carries the subset of
/// fields the ported matchers / segments need.
#[derive(Debug, Clone, Default)]
pub struct MatcherInfo {
    /// Buffer number (Python: `matcher_info['bufnr']`).
    pub bufnr: i32,
    /// Buffer name (Python: `matcher_info['buffer'].name`).
    /// Bytes shape because Python `buffer.name` is `bytes` on vim ≥ 8.
    pub buffer_name: Option<Vec<u8>>,
    /// Per-buffer option cache (Python:
    /// `vim.eval('getbufoption(...)')`).
    pub buffer_options: std::collections::HashMap<String, String>,
}

/// Port of `buffer_name()` from
/// `powerline/bindings/vim/__init__.py:415` / `:420`.
///
/// Returns the current buffer's name as bytes, or `None` if no name
/// is set. Python's two-version dispatch (vim ≥ 8 vs old) collapses
/// to one Rust fn since the Rust port doesn't model the vim plugin
/// version split.
pub fn buffer_name(matcher_info: &MatcherInfo) -> Option<Vec<u8>> {
    // py:417 / :422  return matcher_info['buffer'].name
    matcher_info.buffer_name.clone()
}

/// Port of `vim_getbufoption()` from
/// `powerline/bindings/vim/__init__.py:275` / `:284`.
///
/// Returns the value of `option` on `matcher_info`'s buffer. Python's
/// two-version dispatch (try `info['buffer'].options[option]`,
/// fall back to `vim.eval('getbufvar(...)')`) collapses to one Rust
/// fn over the cached option dict.
pub fn vim_getbufoption(matcher_info: &MatcherInfo, option: &str) -> String {
    // py:276 / :285  return info['buffer'].options[option]
    matcher_info
        .buffer_options
        .get(option)
        .cloned()
        .unwrap_or_default()
}

/// Port of `list_tabpages()` from
/// `powerline/bindings/vim/__init__.py:370`.
///
/// Returns the list of vim tabpages. Without a live vim connection
/// the Rust port returns an empty Vec; the selector below treats that
/// as "no tabs" which is the safe default.
pub fn list_tabpages() -> Vec<()> {
    // py:371  return vim.tabpages — no equivalent in Rust without RPC
    Vec::new()
}

/// Port of `bufvar_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists(...)') wrapper).
///
/// Returns true if buffer-local variable `var` is defined on
/// `matcher_info`'s buffer. Stub returns false (no vim connection).
pub fn bufvar_exists(_matcher_info: Option<&MatcherInfo>, _var: &str) -> bool {
    false
}

/// Port of `vim_func_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists(":func")') wrapper).
///
/// Returns true if vim function `name` is defined. Stub returns false.
pub fn vim_func_exists(_name: &str) -> bool {
    false
}

/// Port of `vim_global_exists()` from
/// `powerline/bindings/vim/__init__.py` (vim.eval('exists("g:var")') wrapper).
///
/// Returns true if vim global variable `name` is defined. Stub returns false.
pub fn vim_global_exists(_name: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_name_returns_set_value() {
        let info = MatcherInfo {
            bufnr: 1,
            buffer_name: Some(b"/tmp/test.txt".to_vec()),
            ..Default::default()
        };
        assert_eq!(buffer_name(&info), Some(b"/tmp/test.txt".to_vec()));
    }

    #[test]
    fn vim_getbufoption_returns_value_if_set() {
        let mut opts = std::collections::HashMap::new();
        opts.insert("filetype".into(), "rust".into());
        let info = MatcherInfo {
            buffer_options: opts,
            ..Default::default()
        };
        assert_eq!(vim_getbufoption(&info, "filetype"), "rust");
        assert_eq!(vim_getbufoption(&info, "missing"), "");
    }

    #[test]
    fn list_tabpages_empty_when_no_vim() {
        assert!(list_tabpages().is_empty());
    }
}
