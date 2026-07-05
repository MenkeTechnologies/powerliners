// vim:fileencoding=utf-8:noet
//! Thin shim for `scripts/powerline-render`.
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

#[path = "shared/house_help.rs"]
mod house_help;

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    // House `--help` / `--version` (display-only) — intercepted before
    // the render argparse so `-h` prints the styled screen instead of
    // the "--ext is required" error.
    if house_help::wants_help(&argv) {
        print!("{}", render_help());
        std::process::exit(0);
    }
    if house_help::wants_version(&argv) {
        println!("{}", house_help::version_line("powerline-render"));
        std::process::exit(0);
    }

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

/// House `--help` screen for the one-shot renderer (also the daemon
/// fallback path).
fn render_help() -> String {
    use house_help::Section;
    house_help::help(
        "One-shot renderer — builds one prompt / statusline per call \
         (the powerline client's fallback when the daemon is down).",
        "ONE-SHOT RENDER // NO DAEMON REQUIRED",
        "powerline-render EXT [SIDE] [OPTIONS]",
        &[
            Section {
                title: "MODES",
                items: &[
                    (
                        "powerline-render EXT [SIDE]",
                        "render EXT (tmux, shell, …) for SIDE",
                    ),
                    ("powerline-render tmux right", "tmux right-hand statusline"),
                    ("powerline-render shell left", "shell left prompt"),
                ],
            },
            Section {
                title: "OPTIONS",
                items: &[
                    ("-w, --width N", "truncate output to N columns"),
                    ("-r, --renderer-module M", "override the renderer module"),
                    (
                        "-c, --config-override K=V",
                        "override main-config keys (repeatable)",
                    ),
                    (
                        "-t, --theme-override K=V",
                        "override theme keys (repeatable)",
                    ),
                    ("-R, --renderer-arg K=V", "pass an argument to the renderer"),
                    (
                        "-p, --config-path PATH",
                        "add a config search path (repeatable)",
                    ),
                    ("-m, --mode MODE", "shorthand for -R mode=MODE"),
                    ("--last-exit-code N", "previous command exit status"),
                    ("--last-pipe-status L", "previous pipe exit statuses"),
                    ("--jobnum N", "number of background jobs"),
                    ("--socket PATH", "daemon socket path"),
                    ("-h, --help", "print this help"),
                    ("-V, --version", "print version"),
                ],
            },
        ],
        "No socket, no daemon, still the whole line.",
        "JACK IN. RENDER ONCE. PRINT THE LINE.",
    )
}
