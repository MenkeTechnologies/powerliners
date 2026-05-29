// vim:fileencoding=utf-8:noet
//! Port of `powerline/bindings/qtile/widget.py`.
//!
//! QTile window-manager widget bindings. Defines two classes:
//!   - `QTilePowerline` — extends Powerline with a do_setup that
//!     assigns `obj.powerline = self`
//!   - `PowerlineTextBox` — extends QTile's `TextBox` to render the
//!     powerline statusline into a pango-markup text region with a
//!     periodic timer-driven refresh
//!
//! Rust port surfaces both structurally + the constructor / update
//! flow / cmd_get / cmd_update / timer_setup state machine. The
//! QTile runtime hooks (`self.bar.draw`, `self.timeout_add`,
//! `self._configure`, `self.drawer.textlayout`) are stubbed.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from libqtile.bar import CALCULATED                                                       // py:4
// from libqtile.widget import TextBox                                                       // py:5
// from powerline import Powerline                                                            // py:7

use serde_json::{Map, Value};

/// Port of the `CALCULATED` width sentinel from
/// `libqtile.bar` (py:4 import). The value is `-1` in libqtile;
/// the Rust port surfaces the constant for callers that want to
/// pin the default width.
pub const CALCULATED: i64 = -1;

/// Port of `class QTilePowerline(Powerline)` from
/// `powerline/bindings/qtile/widget.py:10`.
///
/// Minimal subclass with a do_setup that assigns
/// `obj.powerline = self`.
pub struct QTilePowerline;

impl Default for QTilePowerline {
    fn default() -> Self {
        Self::new()
    }
}

impl QTilePowerline {
    /// Constructs a fresh `QTilePowerline`.
    pub fn new() -> Self {
        Self
    }

    /// Port of `QTilePowerline.do_setup()` from
    /// `powerline/bindings/qtile/widget.py:11`.
    pub fn do_setup(&self, obj: &mut Map<String, Value>) {
        // py:12  obj.powerline = self
        obj.insert(
            "powerline".to_string(),
            Value::String("<QTilePowerline>".into()),
        );
    }
}

/// Port of `class PowerlineTextBox(TextBox)` from
/// `powerline/bindings/qtile/widget.py:15`.
pub struct PowerlineTextBox {
    /// Python: `self.text` — current displayed text (bytes in
    /// Python; Rust port stores the UTF-8 String directly).
    pub text: String,
    /// Python: `self.width`.
    pub width: i64,
    /// Python: `self.side` — "left" or "right".
    pub side: String,
    /// Python: `self.update_interval` (seconds).
    pub update_interval: f64,
    /// Python: `self.did_run_timer_setup`.
    pub did_run_timer_setup: bool,
    /// Python: `self.configured` — flipped by `_configure`.
    pub configured: bool,
    /// Python: `self.bar_draw_calls` — for tests, mirrors how often
    /// `self.bar.draw()` would have fired.
    pub bar_draw_calls: u32,
    /// Python: `self.layout.markup` — flipped by `_configure`.
    pub layout_markup: bool,
}

impl PowerlineTextBox {
    /// Port of `PowerlineTextBox.__init__()` from
    /// `powerline/bindings/qtile/widget.py:18`.
    ///
    /// Defaults: `timeout=2`, `text=" "`, `width=CALCULATED`,
    /// `side="right"`, `update_interval=None`. Per py:21,
    /// `update_interval or timeout` collapses None to the timeout.
    pub fn new(
        timeout: f64,
        text: impl Into<String>,
        width: i64,
        side: impl Into<String>,
        update_interval: Option<f64>,
    ) -> Self {
        // py:19  super().__init__(text, width, **config)
        // py:21  self.update_interval = update_interval or timeout
        let update_interval = update_interval.unwrap_or(timeout);
        Self {
            text: text.into(),
            width,
            side: side.into(),
            update_interval,
            // py:22  self.did_run_timer_setup = False
            did_run_timer_setup: false,
            // QTile sets `configured = False` initially; flipped by
            // `_configure`.
            configured: false,
            bar_draw_calls: 0,
            // QTile sets `layout.markup = False` initially.
            layout_markup: false,
        }
    }

    /// Returns a `PowerlineTextBox` with all-default args
    /// (matches Python's bare `PowerlineTextBox()` call).
    pub fn with_defaults() -> Self {
        // py:18  defaults: timeout=2, text=b' ', width=CALCULATED, side='right'
        Self::new(2.0, " ", CALCULATED, "right", None)
    }

    /// Port of `PowerlineTextBox.update()` from
    /// `powerline/bindings/qtile/widget.py:26`.
    ///
    /// Re-renders the text via the supplied render callback,
    /// triggers a bar draw, and returns true so the timer
    /// re-arms. Skips when `configured == false`.
    pub fn update<R>(&mut self, mut render: R) -> bool
    where
        R: FnMut(&str) -> String,
    {
        // py:27-28  if not self.configured: return True
        if !self.configured {
            return true;
        }
        // py:29  self.text = self.powerline.render(side=self.side).encode('utf-8')
        self.text = render(&self.side);
        // py:30  self.bar.draw()
        self.bar_draw_calls += 1;
        // py:31  return True
        true
    }

    /// Port of `PowerlineTextBox.cmd_update()` from
    /// `powerline/bindings/qtile/widget.py:33`.
    ///
    /// Python: `self.update(text)` — note the upstream code passes
    /// the `text` arg to `update()` even though `update()` takes
    /// no positional argument. The Rust port mirrors this faithfully
    /// (no-op for `text`).
    pub fn cmd_update<R>(&mut self, _text: &str, render: R) -> bool
    where
        R: FnMut(&str) -> String,
    {
        // py:34  self.update(text)
        self.update(render)
    }

    /// Port of `PowerlineTextBox.cmd_get()` from
    /// `powerline/bindings/qtile/widget.py:36`.
    pub fn cmd_get(&self) -> &str {
        // py:37  return self.text
        &self.text
    }

    /// Port of `PowerlineTextBox.timer_setup()` from
    /// `powerline/bindings/qtile/widget.py:39`.
    ///
    /// Idempotent: first call flips the `did_run_timer_setup` flag
    /// and would invoke `self.timeout_add(update_interval, update)`.
    /// Subsequent calls are no-ops.
    pub fn timer_setup(&mut self) {
        // py:40-42  if not did_run_timer_setup: did_run_timer_setup = True
        if !self.did_run_timer_setup {
            self.did_run_timer_setup = true;
            // py:42  self.timeout_add(self.update_interval, self.update)
        }
    }

    /// Port of `PowerlineTextBox._configure()` from
    /// `powerline/bindings/qtile/widget.py:44`.
    ///
    /// Marks the widget as configured. When `layout.markup` is true
    /// (QTile-0.9.1+), skips the layout recreation + timer_setup
    /// per py:46-47.
    pub fn _configure(&mut self) {
        // py:45  super()._configure(qtile, bar) — stub
        self.configured = true;
        // py:46-47  if self.layout.markup: return
        if self.layout_markup {
            return;
        }
        // py:48-55  self.layout = ... textlayout(..., markup=True)
        // The Rust port sets the markup flag itself to mirror the
        // post-recreate state.
        self.layout_markup = true;
        // py:56  self.timer_setup()
        self.timer_setup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculated_constant_matches_libqtile_sentinel() {
        // libqtile.bar.CALCULATED == -1
        assert_eq!(CALCULATED, -1);
    }

    #[test]
    fn qtile_powerline_do_setup_assigns_powerline_attribute() {
        // py:11-12  obj.powerline = self
        let p = QTilePowerline::new();
        let mut obj = Map::new();
        p.do_setup(&mut obj);
        assert!(obj.contains_key("powerline"));
    }

    #[test]
    fn powerline_text_box_defaults_match_upstream_signature() {
        // py:18  timeout=2, text=b' ', width=CALCULATED, side='right'
        let t = PowerlineTextBox::with_defaults();
        assert_eq!(t.text, " ");
        assert_eq!(t.width, CALCULATED);
        assert_eq!(t.side, "right");
        // py:21  update_interval or timeout → 2
        assert!((t.update_interval - 2.0).abs() < 1e-9);
        assert!(!t.did_run_timer_setup);
        assert!(!t.configured);
    }

    #[test]
    fn update_interval_overrides_timeout_when_supplied() {
        // py:21  update_interval = update_interval or timeout
        let t = PowerlineTextBox::new(2.0, " ", CALCULATED, "right", Some(5.0));
        assert!((t.update_interval - 5.0).abs() < 1e-9);
    }

    #[test]
    fn update_interval_falls_back_to_timeout_when_none() {
        let t = PowerlineTextBox::new(3.5, " ", CALCULATED, "right", None);
        assert!((t.update_interval - 3.5).abs() < 1e-9);
    }

    #[test]
    fn update_skips_when_not_configured() {
        // py:27-28  if not self.configured: return True
        let mut t = PowerlineTextBox::with_defaults();
        let mut render_calls = 0;
        let r = t.update(|_side| {
            render_calls += 1;
            "rendered".to_string()
        });
        assert!(r);
        assert_eq!(render_calls, 0);
        assert_eq!(t.text, " ");
        assert_eq!(t.bar_draw_calls, 0);
    }

    #[test]
    fn update_renders_and_draws_when_configured() {
        // py:29-31  text = render(side); bar.draw(); return True
        let mut t = PowerlineTextBox::with_defaults();
        t.configured = true;
        let mut last_side = String::new();
        let r = t.update(|side| {
            last_side = side.to_string();
            "POWERLINE".to_string()
        });
        assert!(r);
        assert_eq!(t.text, "POWERLINE");
        assert_eq!(last_side, "right");
        assert_eq!(t.bar_draw_calls, 1);
    }

    #[test]
    fn update_passes_side_left_for_left_widget() {
        let mut t = PowerlineTextBox::new(2.0, " ", CALCULATED, "left", None);
        t.configured = true;
        let mut last_side = String::new();
        let _ = t.update(|side| {
            last_side = side.to_string();
            String::new()
        });
        assert_eq!(last_side, "left");
    }

    #[test]
    fn cmd_get_returns_current_text() {
        let mut t = PowerlineTextBox::with_defaults();
        t.text = "HELLO".to_string();
        assert_eq!(t.cmd_get(), "HELLO");
    }

    #[test]
    fn cmd_update_delegates_to_update() {
        // py:33-34  cmd_update(text) calls self.update(text)
        let mut t = PowerlineTextBox::with_defaults();
        t.configured = true;
        let mut calls = 0;
        let r = t.cmd_update("ignored", |_side| {
            calls += 1;
            "RENDERED".to_string()
        });
        assert!(r);
        assert_eq!(calls, 1);
        assert_eq!(t.text, "RENDERED");
    }

    #[test]
    fn timer_setup_first_call_flips_flag() {
        // py:40-42  if not did_run_timer_setup: ...; timeout_add(...)
        let mut t = PowerlineTextBox::with_defaults();
        assert!(!t.did_run_timer_setup);
        t.timer_setup();
        assert!(t.did_run_timer_setup);
    }

    #[test]
    fn timer_setup_subsequent_calls_no_op() {
        let mut t = PowerlineTextBox::with_defaults();
        t.timer_setup();
        t.timer_setup();
        t.timer_setup();
        // Still true, no panic.
        assert!(t.did_run_timer_setup);
    }

    #[test]
    fn configure_marks_widget_configured_and_runs_timer_setup() {
        let mut t = PowerlineTextBox::with_defaults();
        assert!(!t.configured);
        assert!(!t.did_run_timer_setup);
        t._configure();
        assert!(t.configured);
        // py:48-56 path: layout_markup was false → re-create layout
        // and call timer_setup
        assert!(t.did_run_timer_setup);
        assert!(t.layout_markup);
    }

    #[test]
    fn configure_skips_relayout_when_layout_markup_already_true() {
        // py:46-47  if self.layout.markup: return
        let mut t = PowerlineTextBox::with_defaults();
        t.layout_markup = true;
        t._configure();
        assert!(t.configured);
        // timer_setup should NOT have been called via this branch
        assert!(!t.did_run_timer_setup);
    }

    #[test]
    fn update_returns_true_unconditionally() {
        // py:28, 31  return True in both branches
        let mut t = PowerlineTextBox::with_defaults();
        assert!(t.update(|_| String::new()));
        t.configured = true;
        assert!(t.update(|_| String::new()));
    }

    #[test]
    fn bar_draw_calls_accumulate_across_updates() {
        let mut t = PowerlineTextBox::with_defaults();
        t.configured = true;
        for _ in 0..5 {
            t.update(|_side| "x".to_string());
        }
        assert_eq!(t.bar_draw_calls, 5);
    }
}
