// vim:fileencoding=utf-8:noet
//! Thin shim for `scripts/powerline-config`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_config::main`].

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = powerliners::ported::scripts::powerline_config::main(&args);
    std::process::exit(code);
}
