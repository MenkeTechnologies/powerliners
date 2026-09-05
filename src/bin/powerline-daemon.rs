// vim:fileencoding=utf-8:noet
//! `powerline-daemon` binary entry.
//!
//! Wires `scripts::powerline_daemon::main` (the script port) into the
//! shared render runtime under `src/bin/shared/render_runtime.rs`,
//! which holds the bin-private adapter dispatch + config build code.
//!
//! Lives in `src/bin/` (sanctioned non-port location). No new fns
//! land under `src/ported/` from this file.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use powerliners::ported::scripts::powerline_daemon as daemon;
use powerliners::ported::scripts::powerline_daemon::{RenderFn, SpawnWmFn};

#[path = "shared/render_runtime.rs"]
mod render_runtime;

#[path = "shared/house_help.rs"]
mod house_help;

/// Send panics to the diagnostic log as well as stderr.
///
/// `daemonize` points stderr at `/dev/null`, so the default hook writes
/// a backgrounded daemon's panics nowhere. A render that panicked was
/// therefore completely invisible: the log simply stopped mid-render
/// with no line saying a thread had died, and the only symptom was an
/// empty statusline hours later. Logging the message, its location and
/// a backtrace means the next one names the code that caused it.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("unnamed").to_string();
        // Captured only when RUST_BACKTRACE asks for one; otherwise
        // this renders as the usual "run with RUST_BACKTRACE=1" note.
        let backtrace = std::backtrace::Backtrace::capture();
        powerliners::extensions::diag_log::log(&format!(
            "PANIC in thread {} at {}: {}\n{}",
            thread, location, info, backtrace
        ));
        previous(info);
    }));
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

    install_panic_hook();

    // House `--help` / `--version` (display-only) — intercepted before
    // the daemon's own argv parse so `-h` prints the styled screen
    // instead of the old no-op.
    if house_help::wants_help(&argv) {
        print!("{}", daemon_help());
        std::process::exit(0);
    }
    if house_help::wants_version(&argv) {
        println!("{}", house_help::version_line("powerline-daemon"));
        std::process::exit(0);
    }

    // One slot per ext — the daemon caches keyed by PowerlineKey but
    // the configs themselves only depend on `ext`. Lazy-load on first
    // request, then reuse for the daemon's lifetime.
    let store: Arc<Mutex<HashMap<String, render_runtime::Configs>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let renderer = render_runtime::make_renderer();

    let store_clone = store.clone();
    let renderer_clone = renderer.clone();
    let render_fn: Arc<RenderFn> = Arc::new(move |args, environ, cwd, _is_daemon| {
        let ext = args.ext.first().cloned().unwrap_or_default();
        let side = args.side.clone().unwrap_or_default();
        powerliners::extensions::diag_log::log(&format!(
            "daemon REQ ext={} side={} client_cwd={}",
            ext, side, cwd
        ));

        // Reactive Prompt Push (extension, non-port): if the client
        // advertised a wake FIFO, (re)arm an edge-triggered filesystem
        // watch over the inputs backing its prompt so an external change
        // (e.g. `git checkout` in another pane) redraws it in place. This
        // is strictly additive — the pull render below is unchanged.
        if let Some(fifo) = environ.get("POWERLINE_RESET_FIFO") {
            if !fifo.is_empty() {
                powerliners::extensions::watch::register(fifo, std::path::Path::new(cwd));
            }
        }

        let configs = {
            let mut guard = store_clone.lock().expect("config store poisoned");
            // py:851-866  update_renderer's reload-check: when any
            // tracked config file's mtime differs vs the cached load,
            // drop the cache so build_configs re-reads from disk.
            // Honors `common.reload_config` (default true).
            let cached = guard.get(&ext).cloned();
            let stale = cached.as_ref().map(|c| c.is_stale()).unwrap_or(false);
            if stale {
                powerliners::extensions::diag_log::log(&format!(
                    "daemon configs RELOAD ext={} (stale config files detected)",
                    ext
                ));
                guard.remove(&ext);
            }
            match (cached, stale) {
                (Some(c), false) => c,
                _ => match render_runtime::build_configs(&ext) {
                    Ok(c) => {
                        powerliners::extensions::diag_log::log(&format!(
                            "daemon configs BUILT ext={} (cache miss)",
                            ext
                        ));
                        guard.insert(ext.clone(), c.clone());
                        c
                    }
                    Err(e) => {
                        powerliners::extensions::diag_log::log(&format!(
                            "daemon configs ERROR ext={} err={}",
                            ext, e
                        ));
                        return format!("powerline-daemon: config error: {}\n", e).into_bytes();
                    }
                },
            }
        };

        render_runtime::render_once(args, environ, cwd, &configs, &renderer_clone)
    });
    let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_name, _t_evt, _pl_evt| None);
    let code = daemon::main(&argv, render_fn, spawn_wm_fn);
    std::process::exit(code);
}

/// House `--help` screen for the render daemon.
fn daemon_help() -> String {
    use house_help::Section;
    house_help::help(
        "Long-running render daemon — caches configs and serves powerline \
         clients over a Unix socket.",
        "RENDER DAEMON // WARM CONFIGS, FAST PROMPTS",
        "powerline-daemon [OPTIONS]",
        &[Section {
            title: "OPTIONS",
            items: &[
                ("(no args)", "start the daemon in the background"),
                ("-f, --foreground", "run in the foreground (do not fork)"),
                ("-r, --replace", "replace a daemon already running"),
                ("-k, --kill", "kill the running daemon and exit"),
                ("-s, --socket PATH", "socket to bind / connect to"),
                ("-q, --quiet", "suppress the 'already running' notice"),
                ("-h, --help", "print this help"),
                ("-V, --version", "print version"),
            ],
        }],
        "Warm the machine once. Render forever.",
        "JACK IN. HOLD THE SOCKET. SERVE THE PROMPT.",
    )
}
