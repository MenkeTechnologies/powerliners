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
        // py:18-22  r = segment_info.copy(); patch in pdb/init/curframe; return r
        let mut r = segment_info.clone();
        if let Some(pdb) = &self.pdb {
            r.insert(
                "pdb".to_string(),
                serde_json::json!({"stack_len": pdb.stack_len}),
            );
            r.insert(
                "curframe".to_string(),
                serde_json::json!({
                    "f_lineno": pdb.curframe.f_lineno,
                    "co_filename": pdb.curframe.co_filename,
                    "co_name": pdb.curframe.co_name,
                }),
            );
        }
        r.insert(
            "initial_stack_length".to_string(),
            serde_json::json!(self.initial_stack_length.map(|n| n as u64)),
        );
        r
    }

    /// Port of `PDBRenderer.render()` from
    /// `powerline/renderers/pdb.py:36`.
    ///
    /// Records `initial_stack_length = len(pdb.stack) - 1` on first
    /// call (py:38), then delegates to the base renderer's render.
    pub fn render(&mut self) -> String {
        // py:37-38  if self.initial_stack_length is None: ...
        if self.initial_stack_length.is_none() {
            if let Some(pdb) = &self.pdb {
                self.initial_stack_length = Some(pdb.stack_len.saturating_sub(1));
            }
        }
        // py:39  return Renderer.render(self, **kwargs)
        // (Renderer base not ported — return empty until orchestrator lands.)
        String::new()
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
}
