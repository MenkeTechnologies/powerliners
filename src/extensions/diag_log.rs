// vim:fileencoding=utf-8:noet
//! Diagnostic log writer. Appends timestamped lines to
//! `$HOME/.powerliners/powerliners.log` for post-mortem analysis when a
//! segment renders unexpectedly (silently dropped, slow, returning
//! `None`, etc.). Lives in `extensions` because it's not part of the
//! upstream powerline contract.
//!
//! Rotation: when the active file exceeds [`MAX_BYTES`], it is renamed
//! to `powerliners.log.1` and the existing `.1` → `.2`, … up to
//! [`KEEP_ROTATIONS`]. The oldest file is dropped.

use std::fs::{create_dir_all, rename, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const MAX_BYTES: u64 = 5 * 1024 * 1024; // 5 MiB
const KEEP_ROTATIONS: usize = 3;

static LOG_STATE: OnceLock<Mutex<Option<LogState>>> = OnceLock::new();

struct LogState {
    path: PathBuf,
    file: std::fs::File,
}

fn log_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let mut dir = PathBuf::from(home);
    dir.push(".powerliners");
    create_dir_all(&dir).ok()?;
    dir.push("powerliners.log");
    Some(dir)
}

fn open(path: &Path) -> Option<std::fs::File> {
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

fn rotate(path: &Path) {
    // Shift .1 → .2, .2 → .3, …, .N drops off the end.
    for i in (1..KEEP_ROTATIONS).rev() {
        let from = path.with_extension(format!("log.{}", i));
        let to = path.with_extension(format!("log.{}", i + 1));
        let _ = rename(&from, &to);
    }
    let dot1 = path.with_extension("log.1");
    let _ = rename(path, &dot1);
}

fn init_state() -> Option<LogState> {
    let path = log_path()?;
    let file = open(&path)?;
    Some(LogState { path, file })
}

/// Append `msg` to the diagnostic log with a microsecond-precision
/// epoch timestamp + the calling process id. No-op when the file
/// can't be opened (permissions, missing $HOME, etc.) so the renderer
/// never fails on logging issues.
pub fn log(msg: &str) {
    let cell = LOG_STATE.get_or_init(|| Mutex::new(init_state()));
    if let Ok(mut guard) = cell.lock() {
        if let Some(state) = guard.as_mut() {
            // Rotate if the active file has grown past the cap. We
            // check via metadata rather than tracking writes so that
            // log files left behind by a previous run still rotate
            // correctly on the next append.
            if let Ok(meta) = state.file.metadata() {
                if meta.len() >= MAX_BYTES {
                    rotate(&state.path);
                    if let Some(f) = open(&state.path) {
                        state.file = f;
                    }
                }
            }
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs_f64())
                .unwrap_or(0.0);
            let _ = writeln!(
                state.file,
                "[{:.3} pid={}] {}",
                ts,
                std::process::id(),
                msg
            );
            let _ = state.file.flush();
        }
    }
}
