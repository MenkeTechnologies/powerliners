// vim:fileencoding=utf-8:noet
//! Port of `powerline/renderers/ipython/pre_5.py`.
//!
//! IPython renderer for pre-5.0 IPython versions. Python uses
//! multiple inheritance to mix `IPythonRenderer` + `ShellRenderer` +
//! `ReadlineRenderer` into the prompt / non-prompt renderer pair;
//! the `RendererProxy` wraps both behind an `is_prompt` dispatch.
//!
//! Rust port: structural skeletons of all four classes plus the
//! `client_id='ipython'` injection in `do_render`. The actual
//! `render` / `do_render` / `render_above_lines` / `shutdown`
//! delegations require the unported `ShellRenderer` /
//! `ReadlineRenderer` / `IPythonRenderer` bases.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// from powerline.renderers.shell import ShellRenderer                                      // py:4
// from powerline.renderers.shell.readline import ReadlineRenderer                          // py:5
// from powerline.renderers.ipython import IPythonRenderer                                  // py:6

use serde_json::{Map, Value};

/// Port of `class IPythonPre50Renderer(IPythonRenderer, ShellRenderer)`
/// from `powerline/renderers/ipython/pre_5.py:9`.
///
/// Doc: "Powerline ipython segment renderer for pre-5.0 IPython
/// versions."
#[derive(Debug)]
pub struct IPythonPre50Renderer;

impl Default for IPythonPre50Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonPre50Renderer {
    /// Returns an empty `IPythonPre50Renderer`. State lives on the
    /// unported `ShellRenderer` base; the Rust port stubs the
    /// constructor.
    pub fn new() -> Self {
        Self
    }

    /// Port of `IPythonPre50Renderer.render()` from
    /// `powerline/renderers/ipython/pre_5.py:11`.
    ///
    /// **Status:** stub. The Python source explicitly calls
    /// `super(ShellRenderer, self).render(**kwargs)` to skip the
    /// shell renderer's render and use the IPython renderer's
    /// instead. The Rust port can't faithfully reproduce that MRO
    /// skip without porting the bases.
    pub fn render(&self) -> String {
        // py:12-13 stub
        String::new()
    }

    /// Port of `IPythonPre50Renderer.do_render()` from
    /// `powerline/renderers/ipython/pre_5.py:15`.
    ///
    /// Injects `client_id='ipython'` into segment_info before
    /// delegating to the base do_render.
    pub fn do_render(&self, segment_info: &mut Map<String, Value>) -> String {
        // py:16  segment_info.update(client_id='ipython')
        segment_info.insert("client_id".to_string(), Value::String("ipython".into()));
        // py:17-20  base do_render — stubbed
        String::new()
    }
}

/// Port of `class IPythonPromptRenderer(IPythonPre50Renderer, ReadlineRenderer)`
/// from `powerline/renderers/ipython/pre_5.py:23`.
///
/// Doc: "Powerline ipython prompt (in and in2) renderer".
#[derive(Debug)]
pub struct IPythonPromptRenderer {
    pub base: IPythonPre50Renderer,
}

impl Default for IPythonPromptRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonPromptRenderer {
    pub fn new() -> Self {
        Self {
            base: IPythonPre50Renderer::new(),
        }
    }
}

/// Port of `class IPythonNonPromptRenderer(IPythonPre50Renderer)`
/// from `powerline/renderers/ipython/pre_5.py:28`.
///
/// Doc: "Powerline ipython non-prompt (out and rewrite) renderer".
#[derive(Debug)]
pub struct IPythonNonPromptRenderer {
    pub base: IPythonPre50Renderer,
}

impl Default for IPythonNonPromptRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl IPythonNonPromptRenderer {
    pub fn new() -> Self {
        Self {
            base: IPythonPre50Renderer::new(),
        }
    }
}

/// Port of `class RendererProxy(object)` from
/// `powerline/renderers/ipython/pre_5.py:33`.
///
/// Wraps two renderers (prompt + non-prompt) behind an `is_prompt`
/// dispatch in `render()`.
#[derive(Debug)]
pub struct RendererProxy {
    /// Python: `self.prompt_renderer`.
    pub prompt_renderer: IPythonPromptRenderer,
    /// Python: `self.non_prompt_renderer`.
    pub non_prompt_renderer: IPythonNonPromptRenderer,
}

impl Default for RendererProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererProxy {
    /// Port of `RendererProxy.__init__()` from
    /// `powerline/renderers/ipython/pre_5.py:39`.
    pub fn new() -> Self {
        // py:40-43  old_widths = {}; non_prompt_renderer = IPythonNonPromptRenderer(...)
        Self {
            prompt_renderer: IPythonPromptRenderer::new(),
            non_prompt_renderer: IPythonNonPromptRenderer::new(),
        }
    }

    /// Port of `RendererProxy.render_above_lines()` from
    /// `powerline/renderers/ipython/pre_5.py:45`.
    ///
    /// Delegates to the non-prompt renderer's render_above_lines.
    pub fn render_above_lines(&self) -> Vec<String> {
        // py:46  return self.non_prompt_renderer.render_above_lines(...)
        Vec::new()
    }

    /// Port of `RendererProxy.render()` from
    /// `powerline/renderers/ipython/pre_5.py:48`.
    ///
    /// `is_prompt`-based dispatch — true → prompt_renderer,
    /// false → non_prompt_renderer.
    pub fn render(&self, is_prompt: bool) -> String {
        // py:49-50  (prompt_renderer if is_prompt else non_prompt_renderer).render(...)
        if is_prompt {
            self.prompt_renderer.base.render()
        } else {
            self.non_prompt_renderer.base.render()
        }
    }

    /// Port of `RendererProxy.shutdown()` from
    /// `powerline/renderers/ipython/pre_5.py:52`.
    ///
    /// Shuts down both wrapped renderers.
    pub fn shutdown(&self) {
        // py:53-54  prompt_renderer.shutdown(); non_prompt_renderer.shutdown()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipython_pre50_renderer_constructs() {
        let _r = IPythonPre50Renderer::new();
    }

    #[test]
    fn do_render_injects_client_id_ipython() {
        // py:16  segment_info.update(client_id='ipython')
        let r = IPythonPre50Renderer::new();
        let mut info = Map::new();
        let _ = r.do_render(&mut info);
        assert_eq!(
            info.get("client_id"),
            Some(&Value::String("ipython".into()))
        );
    }

    #[test]
    fn do_render_preserves_existing_segment_info_keys() {
        let r = IPythonPre50Renderer::new();
        let mut info = Map::new();
        info.insert("user".to_string(), Value::String("alice".into()));
        let _ = r.do_render(&mut info);
        assert_eq!(info.get("user"), Some(&Value::String("alice".into())));
        assert_eq!(
            info.get("client_id"),
            Some(&Value::String("ipython".into()))
        );
    }

    #[test]
    fn do_render_overwrites_pre_existing_client_id() {
        // py:16  .update() overwrites
        let r = IPythonPre50Renderer::new();
        let mut info = Map::new();
        info.insert("client_id".to_string(), Value::String("other".into()));
        let _ = r.do_render(&mut info);
        assert_eq!(
            info.get("client_id"),
            Some(&Value::String("ipython".into()))
        );
    }

    #[test]
    fn ipython_prompt_renderer_wraps_base() {
        let r = IPythonPromptRenderer::new();
        // Construction just verifies the base wrap chain.
        let _ = r.base;
    }

    #[test]
    fn ipython_non_prompt_renderer_wraps_base() {
        let r = IPythonNonPromptRenderer::new();
        let _ = r.base;
    }

    #[test]
    fn renderer_proxy_has_two_inner_renderers() {
        let p = RendererProxy::new();
        // Both inner renderer types present (compilation pin).
        let _: &IPythonPromptRenderer = &p.prompt_renderer;
        let _: &IPythonNonPromptRenderer = &p.non_prompt_renderer;
    }

    #[test]
    fn renderer_proxy_render_dispatches_on_is_prompt() {
        // Stub: both branches currently return "".
        let p = RendererProxy::new();
        assert_eq!(p.render(true), "");
        assert_eq!(p.render(false), "");
    }

    #[test]
    fn renderer_proxy_render_above_lines_returns_vec() {
        let p = RendererProxy::new();
        let lines = p.render_above_lines();
        assert!(lines.is_empty());
    }

    #[test]
    fn renderer_proxy_shutdown_is_callable() {
        let p = RendererProxy::new();
        p.shutdown();
    }
}
