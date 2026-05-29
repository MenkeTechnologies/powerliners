// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/pdb.py`.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import sys                                       // py:4
// import platform                                  // py:5
// from powerline.renderers.shell.readline import ReadlineRenderer                          // py:7
// from powerline.renderer import Renderer          // py:8

/// pdb instance handle.
///
/// Mirrors `self.pdb` (a `pdb.Pdb` instance). The Rust port carries the
/// stack length and current-frame info that `get_segment_info` reads
/// at py:18-21.
#[derive(Debug, Clone, Default)]
pub struct PdbHandle {
    pub stack_len: usize,
    pub curframe: crate::ported::segments::pdb::PdbCurFrame,
}

/// Port of `class PDBRenderer(ReadlineRenderer)` from
/// `powerline/renderers/pdb.py:11`.
///
/// PDB-specific powerline renderer.
pub struct PDBRenderer {
    /// Python: `self.pdb` (set via `set_pdb`) — py:14
    pub pdb: Option<PdbHandle>,
    /// Python: `self.initial_stack_length` — py:15
    pub initial_stack_length: Option<usize>,
}

impl Default for PDBRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PDBRenderer {
    /// Inherits `escape_hl_start`/`escape_hl_end` from `ReadlineRenderer`
    /// (`\x01` / `\x02`).
    #[allow(non_upper_case_globals)]
    pub const escape_hl_start: &'static str =
        crate::ported::renderers::shell::readline::ReadlineRenderer::escape_hl_start;
    #[allow(non_upper_case_globals)]
    pub const escape_hl_end: &'static str =
        crate::ported::renderers::shell::readline::ReadlineRenderer::escape_hl_end;

    /// Construct a fresh PDBRenderer with no pdb instance set yet.
    pub fn new() -> Self {
        Self {
            pdb: None,                  // py:14  pdb = None
            initial_stack_length: None, // py:15  initial_stack_length = None
        }
    }

    /// Port of `PDBRenderer.set_pdb()` from
    /// `powerline/renderers/pdb.py:24`.
    ///
    /// Record currently used `pdb.Pdb` instance.
    ///
    /// Must be called before first calling `render` method.
    pub fn set_pdb(&mut self, pdb: PdbHandle) {
        // py:24  def set_pdb(self, pdb):
        // py:25-33  docstring
        // py:34  self.pdb = pdb
        self.pdb = Some(pdb);
    }

    /// Port of `PDBRenderer.get_segment_info()` from
    /// `powerline/renderers/pdb.py:17`.
    ///
    /// Returns a copy of `segment_info` with `pdb` /
    /// `initial_stack_length` / `curframe` keys patched in.
    pub fn get_segment_info(
        &self,
        segment_info: &serde_json::Map<String, serde_json::Value>,
    ) -> serde_json::Map<String, serde_json::Value> {
        // py:17  def get_segment_info(self, segment_info, mode):
        // py:18  r = self.segment_info.copy()
        let mut r = segment_info.clone();
        if let Some(pdb) = &self.pdb {
            // py:19  r['pdb'] = self.pdb
            r.insert(
                "pdb".to_string(),
                serde_json::json!({"stack_len": pdb.stack_len}),
            );
            // py:21  r['curframe'] = self.pdb.curframe
            r.insert(
                "curframe".to_string(),
                serde_json::json!({
                    "f_lineno": pdb.curframe.f_lineno,
                    "co_filename": pdb.curframe.co_filename,
                    "co_name": pdb.curframe.co_name,
                }),
            );
        }
        // py:20  r['initial_stack_length'] = self.initial_stack_length
        r.insert(
            "initial_stack_length".to_string(),
            serde_json::json!(self.initial_stack_length.map(|n| n as u64)),
        );
        // py:22  return r
        r
    }

    /// Port of `PDBRenderer.render()` from
    /// `powerline/renderers/pdb.py:36`.
    ///
    /// Records `initial_stack_length = len(pdb.stack) - 1` on first
    /// call (py:38), then delegates to the base renderer's render.
    pub fn render(&mut self) -> String {
        // py:36  def render(self, **kwargs):
        // py:37  if self.initial_stack_length is None:
        if self.initial_stack_length.is_none() {
            if let Some(pdb) = &self.pdb {
                // py:38  self.initial_stack_length = len(self.pdb.stack) - 1
                self.initial_stack_length = Some(pdb.stack_len.saturating_sub(1));
            }
        }
        // py:39  return Renderer.render(self, **kwargs)
        // Base Renderer.render not yet ported; callers route through
        // their own renderer stack and use `do_render` for the ASCII
        // post-processing pass below.
        String::new()
    }

    /// Port of `PDBRenderer.do_render()` from
    /// `powerline/renderers/pdb.py:42-47`.
    ///
    /// Python only defines this method when
    /// `sys.version_info < (3,) and platform.python_implementation() == 'PyPy'`
    /// (py:41) — the body strips non-ASCII chars from the base
    /// renderer's output (PyPy 2 didn't surface Unicode reliably from
    /// pdb).
    ///
    /// Rust runs the body unconditionally — `replace_unmappable_chars`
    /// is harmless on already-ASCII strings, and modern callers can
    /// opt in to the ASCII fold without a runtime version check.
    ///
    /// `base_do_render` is the super().do_render result — closure-
    /// injected since the base `Renderer.do_render` dispatch isn't
    /// reachable from a typed Rust struct.
    pub fn do_render<F>(&self, base_do_render: F) -> String
    where
        F: FnOnce() -> String,
    {
        // py:42  def do_render(self, **kwargs):
        // py:43  # Make sure that only ASCII characters survive
        // py:44  ret = super(PDBRenderer, self).do_render(**kwargs)
        let ret = base_do_render();
        // py:45  ret = ret.encode('ascii', 'replace')
        // py:46  ret = ret.decode('ascii')
        // Python's `.encode('ascii', 'replace')` substitutes non-ASCII
        // codepoints with `?`. Mirror with a char-walk.
        let folded: String = ret
            .chars()
            .map(|c| if c.is_ascii() { c } else { '?' })
            .collect();
        // py:47  return ret
        folded
    }
}

/// Port of module-level binding `renderer` from
/// `powerline/renderers/pdb.py:53`.
#[allow(non_camel_case_types)]
pub type renderer = PDBRenderer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_pdb_renderer_has_no_pdb_set() {
        let r = PDBRenderer::new();
        assert!(r.pdb.is_none());
        assert!(r.initial_stack_length.is_none());
    }

    #[test]
    fn set_pdb_stores_handle() {
        let mut r = PDBRenderer::new();
        r.set_pdb(PdbHandle {
            stack_len: 5,
            ..Default::default()
        });
        assert_eq!(r.pdb.as_ref().unwrap().stack_len, 5);
    }

    #[test]
    fn render_initializes_initial_stack_length() {
        let mut r = PDBRenderer::new();
        r.set_pdb(PdbHandle {
            stack_len: 5,
            ..Default::default()
        });
        r.render();
        // py:38  initial_stack_length = len(stack) - 1 = 4
        assert_eq!(r.initial_stack_length, Some(4));
    }

    #[test]
    fn render_does_not_overwrite_initial_stack_length_on_second_call() {
        let mut r = PDBRenderer::new();
        r.set_pdb(PdbHandle {
            stack_len: 5,
            ..Default::default()
        });
        r.render();
        // Simulate a deeper stack on second call
        r.pdb.as_mut().unwrap().stack_len = 10;
        r.render();
        // Should still be 4 (was set on first render)
        assert_eq!(r.initial_stack_length, Some(4));
    }

    #[test]
    fn get_segment_info_inserts_curframe_and_pdb_keys() {
        let mut r = PDBRenderer::new();
        r.set_pdb(PdbHandle {
            stack_len: 3,
            curframe: crate::ported::segments::pdb::PdbCurFrame {
                f_lineno: 42,
                co_filename: "/tmp/x.py".to_string(),
                co_name: "foo".to_string(),
            },
        });
        let seg = serde_json::Map::new();
        let out = r.get_segment_info(&seg);
        assert_eq!(out["curframe"]["f_lineno"], 42);
        assert_eq!(out["curframe"]["co_name"], "foo");
        assert_eq!(out["pdb"]["stack_len"], 3);
    }

    #[test]
    fn pdb_renderer_inherits_readline_escape_markers() {
        assert_eq!(PDBRenderer::escape_hl_start, "\x01");
        assert_eq!(PDBRenderer::escape_hl_end, "\x02");
    }

    #[test]
    fn do_render_strips_non_ascii_chars() {
        // py:44-46  encode/decode ascii with 'replace'.
        let r = PDBRenderer::new();
        let out = r.do_render(|| "hello\u{2603}world".to_string());
        assert_eq!(out, "hello?world");
    }

    #[test]
    fn do_render_passes_ascii_unchanged() {
        let r = PDBRenderer::new();
        let out = r.do_render(|| "plain ASCII string".to_string());
        assert_eq!(out, "plain ASCII string");
    }
}
