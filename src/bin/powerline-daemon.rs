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

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();

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
