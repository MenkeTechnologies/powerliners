// vim:fileencoding=utf-8:noet
//! Thin shim for `scripts/powerline-config`.
//!
//! Delegates to [`powerliners::ported::scripts::powerline_config::main`].

#[path = "shared/house_help.rs"]
mod house_help;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // House `--help` / `--version` (display-only) — intercepted before
    // the subcommand dispatch so a bare `powerline-config --help` shows
    // the styled screen instead of the "missing function" error.
    if house_help::wants_help(&args) {
        print!("{}", config_help());
        std::process::exit(0);
    }
    if house_help::wants_version(&args) {
        println!("{}", house_help::version_line("powerline-config"));
        std::process::exit(0);
    }

    let code = powerliners::ported::scripts::powerline_config::main(&args);
    std::process::exit(code);
}

/// House `--help` screen for the config helper.
fn config_help() -> String {
    use house_help::Section;
    house_help::help(
        "Config helper — emits the shell / tmux binding snippets and \
         answers 'is this component enabled' checks.",
        "CONFIG HELPER // BINDINGS ON DEMAND",
        "powerline-config {tmux|shell|vim} ACTION [OPTIONS]",
        &[
            Section {
                title: "TMUX",
                items: &[
                    ("tmux source", "print the tmux source snippet"),
                    ("tmux setenv", "print the tmux setenv snippet"),
                    ("tmux setup", "run tmux source + setenv"),
                ],
            },
            Section {
                title: "SHELL",
                items: &[
                    ("shell command", "print the deduced powerline command path"),
                    (
                        "shell uses COMPONENT",
                        "exit 0 if COMPONENT (tmux/prompt) is enabled",
                    ),
                    ("  -s, --shell SHELL", "shell name for the uses check"),
                ],
            },
            Section {
                title: "VIM",
                items: &[("vim source-path", "print the bundled vim plugin path")],
            },
            Section {
                title: "OPTIONS",
                items: &[
                    (
                        "-p, --config-path PATH",
                        "add a config search path (repeatable)",
                    ),
                    ("-h, --help", "print this help"),
                    ("-V, --version", "print version"),
                ],
            },
        ],
        "The bindings write themselves.",
        "JACK IN. WIRE THE SHELL. LOAD THE LINE.",
    )
}
