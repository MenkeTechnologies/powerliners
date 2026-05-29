// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/ipython/__init__.py`.
//!
//! Powerline ipython segment renderer.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.theme import Theme                // py:4
// from powerline.renderers.shell import PromptRenderer                                     // py:5

pub mod pre_5;
pub mod since_5;
pub mod since_7;

use serde_json::{Map, Value};

/// Port of `class IPythonRenderer(PromptRenderer)` from
/// `powerline/renderers/ipython/__init__.py:8`.
///
/// Powerline ipython segment renderer. Inherits from PromptRenderer
/// (unported); the Rust port carries the IPython-specific methods.
pub struct IPythonRenderer {
    /// Python: `self.segment_info` (inherited from Renderer base).
    pub segment_info: Map<String, Value>,
}

impl Default for IPythonRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonRenderer {
    /// Construct a fresh IPythonRenderer.
    pub fn new() -> Self {
        Self {
            segment_info: Map::new(),
        }
    }

    /// Port of `IPythonRenderer.get_segment_info()` from
    /// `powerline/renderers/ipython/__init__.py:10`.
    ///
    /// Returns a copy of `self.segment_info` with `ipython` key
    /// patched in from the supplied per-call segment_info.
    pub fn get_segment_info(&self, segment_info: &Value) -> Map<String, Value> {
        // py:11-13  r = segment_info.copy(); r['ipython'] = segment_info; return r
        let mut r = self.segment_info.clone();
        r.insert("ipython".to_string(), segment_info.clone());
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn get_segment_info_patches_ipython_key() {
        let mut r = IPythonRenderer::new();
        r.segment_info
            .insert("color_scheme".into(), json!("monokai"));
        let payload = json!({"prompt_count": 42});
        let out = r.get_segment_info(&payload);
        assert_eq!(out["ipython"]["prompt_count"], 42);
        assert_eq!(out["color_scheme"], "monokai");
    }
}
