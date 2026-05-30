// vim:fileencoding=utf-8:noet
//! Thin shim for `vendor/powerline/scripts/powerline-render`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_render::main`]
//! with a one-shot render closure backed by the shared render runtime
//! under `src/bin/shared/render_runtime.rs`. When the user's prompt
//! invokes `powerline` and the daemon socket is unreachable, the C-
//! client at `src/bin/powerline.rs:78-85` execvp's this binary as the
//! fallback path (mirrors upstream's `powerline-render` script
//! behavior) — so the tmux statusline keeps rendering even when the
//! daemon is down, just per-call instead of socketed.

#[path = "shared/render_runtime.rs"]
mod render_runtime;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let renderer = render_runtime::make_renderer();
    let render_fn = |args: &powerliners::ported::commands::main::Args,
                     environ: &std::collections::HashMap<String, String>,
                     cwd: &str|
     -> Vec<u8> {
        let ext = args.ext.first().cloned().unwrap_or_default();
        let configs = match render_runtime::build_configs(&ext) {
            Ok(c) => c,
            Err(e) => return format!("powerline-render: config error: {}\n", e).into_bytes(),
        };
        render_runtime::render_once(args, environ, cwd, &configs, &renderer)
    };
    let code = powerliners::ported::scripts::powerline_render::main(&argv, render_fn);
    std::process::exit(code);
}
