// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/tmux.py`.
//!
//! Tmux-specific powerline segments. Currently a single segment:
//! `attached_clients`, which renders the count of clients attached to
//! the current tmux session.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2

use crate::ported::bindings::tmux::get_tmux_output;
// py:4  from powerline.bindings.tmux import get_tmux_output

/// Port of `attached_clients()` from `powerline/segments/tmux.py:7`.
///
/// Return the number of tmux clients attached to the currently active
/// session.
///
/// :param int minimum:
///     The minimum number of attached clients that must be present
///     for this segment to be visible.
pub fn attached_clients(pl: &(), minimum: i32) -> Option<String> {
    // py:14  session_output = get_tmux_output(pl, 'list-panes', '-F', '#{session_name}')
    let session_output = get_tmux_output(pl, &["list-panes", "-F", "#{session_name}"])?;
    // py:15  if not session_output: return None
    if session_output.is_empty() {
        return None;                                   // py:16
    }
    // py:17  session_name = session_output.rstrip().split('\n')[0]
    let session_name = session_output
        .trim_end()
        .split('\n')
        .next()
        .map(String::from)?;

    // py:19  attached_clients_output = get_tmux_output(pl, 'list-clients', '-t', session_name)
    let attached_clients_output =
        get_tmux_output(pl, &["list-clients", "-t", &session_name])?;
    // py:20  attached_count = len(attached_clients_output.rstrip().split('\n'))
    let attached_count = attached_clients_output.trim_end().split('\n').count() as i32;

    // py:22  return None if attached_count < minimum else str(attached_count)
    if attached_count < minimum {
        None
    } else {
        Some(attached_count.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `attached_clients` returns None when tmux is unavailable (no
    /// tmux running in test environment). This is the safe default —
    /// the segment is simply hidden.
    #[test]
    fn attached_clients_safe_when_tmux_unavailable() {
        // Force tmux executable to a nonexistent path to guarantee
        // get_tmux_output returns None.
        std::env::set_var("POWERLINE_TMUX_EXE", "/nonexistent/tmux-powerliners-test");
        let r = attached_clients(&(), 1);
        std::env::remove_var("POWERLINE_TMUX_EXE");
        assert!(r.is_none());
    }
}
