// vim:fileencoding=utf-8:noet
//! Which multiplexer binary the tmux bindings shell out to. Sanctioned
//! non-port location per `docs/PORT.md`.
//!
//! Upstream is `os.environ.get('POWERLINE_TMUX_EXE', 'tmux')`
//! (`powerline/bindings/tmux/__init__.py:22`) — written when tmux was
//! the only implementation. `ztmux` is a drop-in tmux server that
//! speaks the same command set but keeps its sockets in
//! `$TMUX_TMPDIR/ztmux-<uid>/` instead of `tmux-<uid>/`, so a hardcoded
//! `tmux` either fails outright (ztmux-only host: `tmux: command not
//! found`) or silently configures the wrong server
//! (`error connecting to /tmp/tmux-<uid>/default`). Either way
//! `powerline-config tmux setup` never reaches the running server and
//! the statusline stays at the tmux default.
//!
//! Resolution order:
//!   1. `POWERLINE_TMUX_EXE` — explicit override, upstream contract.
//!   2. Inside a session: the binary matching `$TMUX`'s socket path.
//!   3. Outside a session: the multiplexer that actually has a live
//!      socket directory for this uid (tmux wins ties).
//!   4. Whichever of `tmux` / `ztmux` is on `PATH`, else `tmux` so the
//!      failure reads the same as upstream's.

use std::path::{Path, PathBuf};

/// The multiplexer binary to invoke.
pub fn resolve() -> String {
    // 1. Explicit override.
    if let Ok(exe) = std::env::var("POWERLINE_TMUX_EXE") {
        if !exe.is_empty() {
            return exe;
        }
    }

    // 2. `$TMUX` is `<socket-path>,<pid>,<session>`; the socket path
    //    names the server we are actually running under.
    if let Ok(tmux) = std::env::var("TMUX") {
        let socket = tmux.split(',').next().unwrap_or("");
        if !socket.is_empty() {
            let exe = if socket.contains("/ztmux-") {
                "ztmux"
            } else {
                "tmux"
            };
            if on_path(exe) {
                return exe.to_string();
            }
        }
    }

    // 3. No session to key off: prefer the multiplexer with a live
    //    socket directory, so a manual `powerline-config tmux setup`
    //    on a ztmux-only host reaches ztmux's server.
    if has_sockets("ztmux") && !has_sockets("tmux") && on_path("ztmux") {
        return "ztmux".to_string();
    }

    // 4. PATH order, upstream default last.
    for exe in ["tmux", "ztmux"] {
        if on_path(exe) {
            return exe.to_string();
        }
    }
    "tmux".to_string()
}

/// Whether `name` resolves to an executable on `PATH`.
fn on_path(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .any(|dir| is_executable(&Path::new(dir).join(name)))
}

fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// Whether `<tmpdir>/<name>-<uid>/` holds at least one server socket.
fn has_sockets(name: &str) -> bool {
    // SAFETY: getuid is a POSIX syscall with no preconditions.
    let uid = unsafe { libc::getuid() };
    let base = std::env::var("TMUX_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    std::fs::read_dir(base.join(format!("{}-{}", name, uid)))
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Restores `POWERLINE_TMUX_EXE` / `TMUX` on drop so a panicking
    /// assert can't leak state into the rest of the suite.
    struct EnvGuard {
        saved: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn new(vars: &[(&'static str, Option<&str>)]) -> Self {
            let saved = vars
                .iter()
                .map(|(k, v)| {
                    let prev = std::env::var(k).ok();
                    match v {
                        Some(val) => std::env::set_var(k, val),
                        None => std::env::remove_var(k),
                    }
                    (*k, prev)
                })
                .collect();
            Self { saved }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, prev) in &self.saved {
                match prev {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
    }

    #[test]
    fn explicit_override_wins() {
        let _lock = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new(&[
            ("POWERLINE_TMUX_EXE", Some("/opt/custom/tmux")),
            ("TMUX", Some("/tmp/ztmux-501/default,1,0")),
        ]);
        assert_eq!(resolve(), "/opt/custom/tmux");
    }

    /// The reported bug: inside ztmux, `powerline-config tmux setup`
    /// must drive `ztmux`, not tmux's `/tmp/tmux-<uid>/default`.
    #[test]
    fn ztmux_socket_in_tmux_env_selects_ztmux() {
        let _lock = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new(&[
            ("POWERLINE_TMUX_EXE", None),
            ("TMUX", Some("/private/tmp/ztmux-501/default,4242,0")),
        ]);
        // Only assert the ztmux preference when ztmux is installed;
        // elsewhere the resolver correctly falls through to PATH.
        if on_path("ztmux") {
            assert_eq!(resolve(), "ztmux");
        }
    }

    #[test]
    fn tmux_socket_in_tmux_env_selects_tmux() {
        let _lock = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new(&[
            ("POWERLINE_TMUX_EXE", None),
            ("TMUX", Some("/private/tmp/tmux-501/default,4242,0")),
        ]);
        if on_path("tmux") {
            assert_eq!(resolve(), "tmux");
        }
    }

    #[test]
    fn resolves_to_a_known_multiplexer_without_env() {
        let _lock = crate::ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let _guard = EnvGuard::new(&[("POWERLINE_TMUX_EXE", None), ("TMUX", None)]);
        let exe = resolve();
        assert!(exe == "tmux" || exe == "ztmux", "unexpected exe: {exe}");
    }

    #[test]
    fn on_path_rejects_nonexistent_binary() {
        assert!(!on_path("powerliners-no-such-multiplexer"));
    }
}
