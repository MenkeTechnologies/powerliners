// vim:fileencoding=utf-8:noet
//! `powerline-daemon` binary entry — thin shim over
//! [`powerliners::scripts::powerline_daemon::main`].
//!
//! Wires placeholder `render_fn` / `spawn_wm_fn` closures since the
//! `Powerline` orchestrator and the `bindings/wm` thread registry
//! aren't ported yet. The lifecycle (UNIX socket bind, daemonize,
//! pidfile lock, accept loop, EOF shutdown) is fully functional —
//! rendering returns a placeholder until the orchestrator lands.

use std::sync::Arc;

use powerliners::scripts::powerline_daemon as daemon;
use powerliners::scripts::powerline_daemon::{RenderFn, SpawnWmFn};

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let render_fn: Arc<RenderFn> = Arc::new(|args, _environ, _cwd, _is_daemon| {
        let ext = args.ext.first().cloned().unwrap_or_default();
        let side = args.side.clone().unwrap_or_default();
        format!(
            "powerline-daemon: render path not yet wired (ext={} side={})\n",
            ext, side
        )
        .into_bytes()
    });
    let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_name, _t_evt, _pl_evt| None);
    let code = daemon::main(&argv, render_fn, spawn_wm_fn);
    std::process::exit(code);
}
