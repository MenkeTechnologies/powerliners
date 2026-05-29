// vim:fileencoding=utf-8:noet
//! Minimal `powerliners` binary entry point.
//!
//! **Status:** scaffold — exercises only the ported leaf functions.
//! Does not yet perform full statusline rendering; the orchestrator
//! (Powerline class + Renderer base + segment dispatcher) is still
//! partial. This binary exists so `cargo run` produces SOMETHING
//! observable end-to-end against the parts that DO work, and so
//! `cargo install` produces an executable to demo with.
//!
//! Current capabilities, all sourced from already-ported modules:
//!   - `powerliners version`              → version constant
//!   - `powerliners attached-clients`     → tmux attached client count
//!   - `powerliners tmux-version`         → parsed tmux version
//!   - `powerliners humanize-bytes <N>`   → humanized byte display
//!
//! Once the orchestrator lands, this binary will gain the real CLI
//! surface mirroring `powerline ext [side] [args...]` from
//! `commands/main.py:get_argparser`.

use powerliners as pl;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("");

    match cmd {
        "version" | "--version" | "-V" => {
            println!("{}", pl::version::get_version());
        }
        "attached-clients" => {
            let minimum: i32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            match pl::segments::tmux::attached_clients(&(), minimum) {
                Some(s) => println!("{}", s),
                None => std::process::exit(2), // tmux unavailable or below minimum
            }
        }
        "tmux-version" => match pl::bindings::tmux::get_tmux_version(&()) {
            Some(v) => println!("tmux {}.{}{}",
                v.major, v.minor,
                v.suffix.as_deref().unwrap_or("")
            ),
            None => {
                eprintln!("powerliners: tmux not available");
                std::process::exit(1);
            }
        },
        "humanize-bytes" => {
            let n: f64 = args.get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| usage_exit("humanize-bytes <number>"));
            println!("{}", pl::lib::humanize_bytes::humanize_bytes(n, "B", false));
        }
        _ => usage_exit_msg(&format!("unknown command {:?}", cmd)),
    }
}

fn usage_exit(arg_hint: &str) -> ! {
    eprintln!(
        "powerliners — scaffold binary (orchestrator not yet ported)\n\
         \n\
         usage: powerliners <command> [args]\n\
         \n\
         commands:\n\
         \x20 version                       Print powerline-status version\n\
         \x20 attached-clients [MIN]        Print tmux attached-client count\n\
         \x20 tmux-version                  Print parsed tmux version\n\
         \x20 humanize-bytes <N>            {}\n",
        arg_hint,
    );
    std::process::exit(64);
}

fn usage_exit_msg(msg: &str) -> ! {
    eprintln!("powerliners: {}\n", msg);
    usage_exit("humanize-bytes <number>")
}
