// vim:fileencoding=utf-8:noet
//! Thin shim for `scripts/powerline-lint`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_lint::main`].

#[path = "shared/house_help.rs"]
mod house_help;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // House `--help` / `--version` (display-only) — intercepted before
    // lint's argparse so `-h` prints the styled screen instead of the
    // "--config-path is required" error.
    if house_help::wants_help(&args) {
        print!("{}", lint_help());
        std::process::exit(0);
    }
    if house_help::wants_version(&args) {
        println!("{}", house_help::version_line("powerline-lint"));
        std::process::exit(0);
    }

    let code = powerliners::ported::scripts::powerline_lint::main(&args);
    std::process::exit(code);
}

/// House `--help` screen for the config linter.
fn lint_help() -> String {
    use house_help::Section;
    house_help::help(
        "Config linter — checks powerline JSON config trees for errors \
         (exit 0 clean, 1 problems, 2 usage).",
        "CONFIG LINTER // CATCH IT BEFORE THE PROMPT",
        "powerline-lint -p PATH [OPTIONS]",
        &[Section {
            title: "OPTIONS",
            items: &[
                (
                    "-p, --config-path PATH",
                    "config search path (required, repeatable)",
                ),
                ("-d, --debug", "enable debug output"),
                ("-h, --help", "print this help"),
                ("-V, --version", "print version"),
            ],
        }],
        "Lint the config before the shell ever sees it.",
        "JACK IN. LINT THE TREE. TRUST THE LINE.",
    )
}
