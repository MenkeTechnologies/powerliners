// vim:fileencoding=utf-8:noet
//! Thin shim for `scripts/powerline-lint`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_lint::main`].

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = powerliners::ported::scripts::powerline_lint::main(&args);
    std::process::exit(code);
}
