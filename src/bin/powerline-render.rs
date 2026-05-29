// vim:fileencoding=utf-8:noet
//! Thin shim for `vendor/powerline/scripts/powerline-render`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_render::main`].

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = powerliners::ported::scripts::powerline_render::main(&args);
    std::process::exit(code);
}
