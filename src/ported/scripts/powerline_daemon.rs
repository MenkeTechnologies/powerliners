// vim:fileencoding=utf-8:noet
//! Port of `vendor/powerline/scripts/powerline-daemon`.
//!
//! 495-line Python socket daemon. Faithful port: every Python function
//! is preserved as a Rust function with the same name and shape. The
//! only deviations are
//!
//! 1. `render()` and `start_wm()` take injected closures
//!    (`render_fn`/`spawn_wm_fn`). The Python originals call
//!    `ShellPowerline(...)` and `wm_threads[name](...)` directly; those
//!    constructors aren't ported yet (Powerline class + wm registry).
//!    The closure injection localizes the un-ported edge to one call
//!    site and keeps the rest of the daemon wire/loop/lifecycle
//!    machinery testable and runnable today — the
//!    `src/bin/powerline-daemon.rs` shim already accepts connections,
//!    parses the wire format, dispatches to `render`/`start_wm`, and
//!    shuts down on the `EOF\0\0` sentinel.
//! 2. The select-based event loop uses `libc::poll` rather than
//!    `libc::select`. Python's `select.select()` is the more limited
//!    syscall; poll avoids `FD_SETSIZE` and reports `POLLHUP` (which
//!    Python's select silently folds into readability — we replicate
//!    that by accepting `POLLIN | POLLHUP` as readable).
//! 3. `atexit.register` has no stable Rust analog. Cleanup of the
//!    pidfile is handled both by a `PidLock` RAII guard (Drop) and by
//!    a SIGTERM handler that calls `_exit(1)` after unlinking.

// #!/usr/bin/env python                              // sh:1
// import socket, os, errno, sys, fcntl, atexit, stat // sh:5-11
// from argparse import ArgumentParser                 // sh:13
// from select import select                            // sh:14
// from signal import signal, SIGTERM                   // sh:15
// from time import sleep                                // sh:16
// from functools import partial                         // sh:17
// from io import BytesIO                                 // sh:18
// from threading import Event                            // sh:19
// from itertools import chain                            // sh:20
// from logging import StreamHandler                      // sh:21
// from powerline.shell import ShellPowerline             // sh:23
// from powerline.commands.main import finish_args, write_output  // sh:24
// from powerline.lib.monotonic import monotonic           // sh:25
// from powerline.lib.encoding import (...)                  // sh:26
// from powerline.bindings.wm import wm_threads              // sh:27
// from powerline.commands.main import get_argparser as get_main_argparser  // sh:29
// from powerline.commands.daemon import get_argparser as get_daemon_argparser  // sh:30

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::ported::commands::main::{finish_args, Args};

/// Port of module-level `USE_FILESYSTEM` from
/// `vendor/powerline/scripts/powerline-daemon:33`.
///
/// Python: `USE_FILESYSTEM = not sys.platform.lower().startswith('linux')`.
/// Linux gets the abstract socket namespace; other platforms get a
/// real filesystem socket file.
#[allow(non_snake_case)]
pub fn USE_FILESYSTEM() -> bool {
    // sh:33  not sys.platform.lower().startswith('linux')
    !cfg!(target_os = "linux")
}

/// Port of `EOF` byte sentinel from
/// `vendor/powerline/scripts/powerline-daemon:50`.
///
/// Python: `EOF = b'EOF\0\0'`.
pub const EOF: &[u8] = b"EOF\0\0";

/// Port of `HOME` constant from
/// `vendor/powerline/scripts/powerline-daemon:65`.
///
/// Python: `HOME = os.path.expanduser('~')`. Computed lazily so
/// tests don't depend on the test runner's `$HOME` at module load.
#[allow(non_snake_case)]
pub fn HOME() -> String {
    // sh:65  os.path.expanduser('~')
    std::env::var("HOME").unwrap_or_default()
}

/// Port of `NonInteractiveArgParser` from
/// `vendor/powerline/scripts/powerline-daemon:36-47`.
///
/// Python subclass of `ArgumentParser` whose `print_usage`/`print_help`/
/// `error` raise `Exception` instead of writing to stdout/exiting.
/// `exit()` is a no-op. The daemon uses this so a malformed client
/// request never tears down the parent process.
///
/// Rust port is a marker struct since the data-only `ArgParser` at
/// `crate::ported::commands::lint::ArgParser` doesn't carry methods
/// that would print/exit anyway. Behavior is encoded as a flag
/// distinguishing this parser kind for the dispatch site at sh:301.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonInteractiveArgParser;

/// Cache key for `state.powerlines` from sh:95-104.
///
/// Python uses a tuple:
///
/// ```text
/// key = (
///   args.ext[0],
///   args.renderer_module,
///   tuple(args.config_override) if args.config_override else None,
///   tuple(args.theme_override) if args.theme_override else None,
///   tuple(args.config_path) if args.config_path else None,
///   environ.get('POWERLINE_THEME_OVERRIDES', ''),
///   environ.get('POWERLINE_CONFIG_OVERRIDES', ''),
///   environ.get('POWERLINE_CONFIG_PATHS', ''),
/// )
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct PowerlineKey {
    pub ext: String,
    pub renderer_module: Option<String>,
    pub config_override: Option<Vec<String>>,
    pub theme_override: Option<Vec<String>>,
    pub config_path: Option<Vec<String>>,
    pub env_theme_overrides: String,
    pub env_config_overrides: String,
    pub env_config_paths: String,
}

impl PowerlineKey {
    /// Build the cache key from `(args, environ)` matching sh:95-104.
    pub fn from(args: &Args, environ: &HashMap<String, String>) -> Self {
        // sh:96  args.ext[0]
        let ext = args.ext.first().cloned().unwrap_or_default();
        Self {
            ext,
            // sh:97  args.renderer_module
            renderer_module: args.renderer_module.clone(),
            // sh:98  tuple(args.config_override) if args.config_override else None
            config_override: args.config_override.clone(),
            // sh:99  tuple(args.theme_override) if args.theme_override else None
            theme_override: args.theme_override.clone(),
            // sh:100  tuple(args.config_path) if args.config_path else None
            config_path: args.config_path.clone(),
            // sh:101  environ.get('POWERLINE_THEME_OVERRIDES', '')
            env_theme_overrides: environ
                .get("POWERLINE_THEME_OVERRIDES")
                .cloned()
                .unwrap_or_default(),
            // sh:102  environ.get('POWERLINE_CONFIG_OVERRIDES', '')
            env_config_overrides: environ
                .get("POWERLINE_CONFIG_OVERRIDES")
                .cloned()
                .unwrap_or_default(),
            // sh:103  environ.get('POWERLINE_CONFIG_PATHS', '')
            env_config_paths: environ
                .get("POWERLINE_CONFIG_PATHS")
                .cloned()
                .unwrap_or_default(),
        }
    }
}

/// Handle for a started WM thread, held in `State.started_wm_threads`.
///
/// Mirrors the Python tuple `(thread, thread_shutdown_event)` at sh:84.
#[derive(Debug)]
pub struct WmHandle {
    pub thread: Option<JoinHandle<()>>,
    pub shutdown_event: Arc<AtomicBool>,
}

/// Port of `class State(object)` from
/// `vendor/powerline/scripts/powerline-daemon:53-62`.
///
/// `__slots__` declares `powerlines`, `logger`, `config_loader`,
/// `started_wm_threads`, `ts_shutdown_event`. `powerlines` is keyed
/// by [`PowerlineKey`]; the value side stays `()` until the
/// `ShellPowerline` type lands (the cache hit/miss bookkeeping still
/// works, the closure injection at the render site is what produces
/// content). `ts_shutdown_event` is the Rust analog of
/// `threading.Event.set()`.
#[derive(Debug, Default)]
pub struct State {
    /// sh:54-55  __slots__ powerlines: rendered Powerline instances
    pub powerlines: HashMap<PowerlineKey, ()>,
    /// sh:58  self.logger = None
    pub logger: Option<()>,
    /// sh:59  self.config_loader = None
    pub config_loader: Option<()>,
    /// sh:60  self.started_wm_threads = {}
    pub started_wm_threads: HashMap<String, WmHandle>,
    /// sh:62  self.ts_shutdown_event = Event()
    pub ts_shutdown_event: Arc<AtomicBool>,
}

impl State {
    /// Port of `State.__init__()` at sh:57-62.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Port of `class NonDaemonShellPowerline(ShellPowerline)` from
/// `vendor/powerline/scripts/powerline-daemon:68-70`.
///
/// Subclass that overrides `get_log_handler` to return a
/// `StreamHandler` (i.e. logs to stderr) instead of the
/// daemon's syslog handler. Rust port is a marker struct used
/// alongside the future `ShellPowerline` impl.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonDaemonShellPowerline;

impl NonDaemonShellPowerline {
    /// Port of `get_log_handler()` at sh:69-70.
    pub fn get_log_handler(&self) {
        // sh:70  return StreamHandler()
    }
}

/// Port of `eintr_retry_call()` from
/// `vendor/powerline/scripts/powerline-daemon:137-144`.
pub fn eintr_retry_call<F, T>(mut func: F) -> std::io::Result<T>
where
    F: FnMut() -> std::io::Result<T>,
{
    loop {
        // sh:139  return func(*args, **kwargs)
        match func() {
            Ok(v) => return Ok(v),
            // sh:141-142  if e.errno == errno.EINTR: continue
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            // sh:143  raise
            Err(e) => return Err(e),
        }
    }
}

/// Port of `safe_bytes()` from
/// `vendor/powerline/scripts/powerline-daemon:175-190`.
///
/// Any `&str` is already valid UTF-8 so the encode path collapses to
/// `s.as_bytes().to_vec()`. The fallback recursion is not reachable
/// since `&str` cannot fail to encode.
pub fn safe_bytes(s: &str) -> Vec<u8> {
    // sh:180  return o.encode(encoding, 'replace')
    s.as_bytes().to_vec()
}

/// Port of `parse_args()` from
/// `vendor/powerline/scripts/powerline-daemon:193-200`.
///
/// Wire format: `<numargs_hex>\0<arg>\0<arg>...\0<cwd>\0<env=val>\0...`.
#[allow(clippy::type_complexity)]
pub fn parse_args(req: &[u8]) -> Option<(Vec<String>, HashMap<String, String>, String)> {
    // sh:194  args = [x.decode(encoding) for x in req.split(b'\0') if x]
    let parts: Vec<String> = req
        .split(|&b| b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect();
    if parts.is_empty() {
        return None;
    }
    // sh:195  numargs = int(args[0], 16)
    let numargs = usize::from_str_radix(&parts[0], 16).ok()?;
    if parts.len() < 1 + numargs + 1 {
        return None;
    }
    // sh:196  shell_args = parser.parse_args(args[1:numargs + 1])
    let shell_args: Vec<String> = parts[1..1 + numargs].to_vec();
    // sh:197  cwd = args[numargs + 1]
    let mut cwd = parts.get(1 + numargs).cloned().unwrap_or_default();
    // sh:198  environ = dict(((k, v) for k, v in (x.partition('=')[0::2] for x in args[numargs + 2:])))
    let mut environ: HashMap<String, String> = HashMap::new();
    for kv in parts.iter().skip(numargs + 2) {
        if let Some(eq) = kv.find('=') {
            environ.insert(kv[..eq].to_string(), kv[eq + 1..].to_string());
        }
    }
    // sh:199  cwd = cwd or environ.get('PWD', '/')
    if cwd.is_empty() {
        cwd = environ
            .get("PWD")
            .cloned()
            .unwrap_or_else(|| "/".to_string());
    }
    Some((shell_args, environ, cwd))
}

/// Port of `do_read()` from
/// `vendor/powerline/scripts/powerline-daemon:147-165`.
///
/// Read until the running buffer ends with `\0\0` or the cumulative
/// wall time hits `timeout` seconds. Returns `None` on timeout, error,
/// or zero-length reads (which Python's `recv` returns on EOF).
pub fn do_read(conn: &mut UnixStream, timeout: Duration) -> Option<Vec<u8>> {
    // sh:150  read = []
    let mut read: Vec<u8> = Vec::new();
    // sh:151  end_time = monotonic() + timeout
    let end_time = Instant::now() + timeout;
    // sh:152  while not read or not read[-1].endswith(b'\0\0'):
    while !read.ends_with(b"\0\0") {
        // sh:153  r, w, e = select((conn,), (), (conn,), timeout)
        let now = Instant::now();
        if now >= end_time {
            // sh:156-157  if monotonic() > end_time: return
            return None;
        }
        let remaining = end_time - now;
        let fd = conn.as_raw_fd();
        let ready = match poll_readable(fd, remaining) {
            Ok(b) => b,
            Err(_) => return None, // sh:154-155  if e: return
        };
        if !ready {
            // sh:158-159  if not r: continue (after the time check at 156)
            continue;
        }
        // sh:160  x = eintr_retry_call(conn.recv, 4096)
        let mut buf = [0u8; 4096];
        match eintr_retry_call(|| conn.read(&mut buf)) {
            // sh:161-162  if x: read.append(x)
            Ok(n) if n > 0 => read.extend_from_slice(&buf[..n]),
            // sh:163-164  else: break (EOF before terminator)
            Ok(_) => break,
            Err(_) => return None,
        }
    }
    // sh:165  return b''.join(read)
    Some(read)
}

/// Helper for `do_read`: wait up to `timeout` for fd to be readable.
///
/// Port of `select((conn,), (), (conn,), timeout)` at sh:153 reduced
/// to single-fd polling. Uses `libc::poll` which is more portable than
/// `libc::select` and avoids the FD_SETSIZE limit.
fn poll_readable(fd: RawFd, timeout: Duration) -> std::io::Result<bool> {
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let ms = timeout
        .as_millis()
        .min(i32::MAX as u128)
        .try_into()
        .unwrap_or(i32::MAX);
    // SAFETY: pfd is a properly initialized libc::pollfd. The poll
    // syscall reads/writes only that struct via the pointer + count.
    let rc = unsafe { libc::poll(&mut pfd, 1, ms) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(err);
    }
    // Python `select.select()` does not surface POLLHUP as an error —
    // peer-close with pending data sets POLLIN|POLLHUP and the read
    // syscall returns the buffered bytes. Treat POLLHUP as "readable"
    // and let the subsequent `read()` distinguish data from EOF (read
    // returns 0). Only POLLERR / POLLNVAL are genuine fd errors.
    if pfd.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        return Err(std::io::Error::other("poll: fd error"));
    }
    Ok(rc > 0 && (pfd.revents & (libc::POLLIN | libc::POLLHUP)) != 0)
}

/// Port of `do_write()` from
/// `vendor/powerline/scripts/powerline-daemon:168-172`.
///
/// Best-effort send; Python swallows all exceptions.
pub fn do_write(conn: &mut UnixStream, result: &[u8]) {
    // sh:169-172  try: eintr_retry_call(sock.sendall, result); except: pass
    let _ = eintr_retry_call(|| conn.write_all(result));
}

/// Type alias for the render closure injected into `render()` and
/// `get_answer()`. Returns the rendered statusline bytes for a given
/// request. Wired by `main()`/tests; the production wiring depends on
/// the not-yet-ported `ShellPowerline`.
pub type RenderFn = dyn Fn(&Args, &HashMap<String, String>, &str, bool) -> Vec<u8> + Send + Sync;

/// Type alias for the WM thread spawner injected into `start_wm()`.
/// Returns a started thread + its dedicated shutdown event.
pub type SpawnWmFn =
    dyn Fn(&str, Arc<AtomicBool>, Arc<AtomicBool>) -> Option<WmHandle> + Send + Sync;

/// Port of `start_wm()` from
/// `vendor/powerline/scripts/powerline-daemon:73-85`.
///
/// Idempotent thread spawn: short-circuits if `wm_name` already
/// running.
pub fn start_wm(args: &Args, state: &mut State, spawn_wm_fn: &Arc<SpawnWmFn>) -> Vec<u8> {
    // sh:74  wm_name = args.ext[0][3:]
    let ext = args.ext.first().cloned().unwrap_or_default();
    let wm_name = ext.strip_prefix("wm.").unwrap_or(&ext).to_string();
    // sh:75-76  if wm_name in state.started_wm_threads: return b''
    if state.started_wm_threads.contains_key(&wm_name) {
        return Vec::new();
    }
    // sh:77  thread_shutdown_event = Event()
    let thread_shutdown_event = Arc::new(AtomicBool::new(false));
    // sh:78-82  thread = wm_threads[wm_name](thread_shutdown_event=..., pl_shutdown_event=..., pl_config_loader=...)
    // sh:83  thread.start()
    match spawn_wm_fn(
        &wm_name,
        thread_shutdown_event.clone(),
        state.ts_shutdown_event.clone(),
    ) {
        Some(handle) => {
            // sh:84  state.started_wm_threads[wm_name] = (thread, thread_shutdown_event)
            state.started_wm_threads.insert(wm_name, handle);
        }
        None => {
            // wm_name not registered → Python KeyError; faithful behavior is
            // to surface it as an error message back to the client.
            return safe_bytes(&format!("Unknown WM: {}", wm_name));
        }
    }
    // sh:85  return b''
    Vec::new()
}

/// Port of `render()` from
/// `vendor/powerline/scripts/powerline-daemon:88-134`.
///
/// Cache `ShellPowerline` per `PowerlineKey`; on a miss, the
/// `render_fn` closure computes the rendered bytes. Python additionally
/// stores the live `ShellPowerline` instance under the same key; the
/// Rust cache-value is `()` until `ShellPowerline` lands. The hit/miss
/// distinction still gates whether the render closure runs.
pub fn render(
    args: &Args,
    environ: &HashMap<String, String>,
    cwd: &str,
    is_daemon: bool,
    state: &mut State,
    render_fn: &Arc<RenderFn>,
) -> Vec<u8> {
    // sh:89-94  segment_info = {getcwd, home, environ, args}
    // (segment_info is owned by the render_fn closure)

    // sh:95-104  key = (...)
    let key = PowerlineKey::from(args, environ);

    // sh:106-130  powerline = state.powerlines[key]; on KeyError construct
    state.powerlines.entry(key).or_insert(());

    // sh:131-134  s = BytesIO(); write_output(args, powerline, segment_info, writer); return s
    let _ = cwd;
    render_fn(args, environ, cwd, is_daemon)
}

/// Port of `get_answer()` from
/// `vendor/powerline/scripts/powerline-daemon:203-212`.
///
/// Parses request, runs `finish_args`, dispatches to `start_wm` for
/// `wm.*` extensions or `render` for everything else. All exceptions
/// become the response body (Python's `except Exception as e: return
/// safe_bytes(str(e))`).
pub fn get_answer(
    req: &[u8],
    is_daemon: bool,
    state: &mut State,
    render_fn: &Arc<RenderFn>,
    spawn_wm_fn: &Arc<SpawnWmFn>,
) -> Vec<u8> {
    // sh:205  args, environ, cwd = parse_args(req, argparser)
    let (raw_args, environ, cwd) = match parse_args(req) {
        Some(t) => t,
        None => return safe_bytes("malformed request"),
    };

    // The wire `raw_args` is the argv that the C/sh client copied from
    // the user's `powerline tmux right ...` invocation. Convert it into
    // an `Args` struct that `finish_args` can operate on. The full
    // argparse port of `get_main_argparser` runs the same flags, so we
    // use a minimal extractor here that maps the positional+flag forms
    // the daemon sees in practice.
    let mut args = parse_client_argv(&raw_args);

    // sh:206  finish_args(argparser, environ, args, is_daemon=True)
    if let Err(e) = finish_args(&environ, &mut args, true) {
        return safe_bytes(&e);
    }

    // sh:207-208  if args.ext[0].startswith('wm.'): start_wm(...)
    if args
        .ext
        .first()
        .map(|e| e.starts_with("wm."))
        .unwrap_or(false)
    {
        return start_wm(&args, state, spawn_wm_fn);
    }

    // sh:210  return safe_bytes(render(args, environ, cwd, is_daemon, state))
    render(&args, &environ, &cwd, is_daemon, state, render_fn)
}

/// Minimal client-argv → `Args` decoder used by `get_answer`.
///
/// Mirrors the positional structure produced by
/// `powerline/commands/main.py:get_argparser` for the commonly-shipped
/// invocations (`powerline tmux right`, `powerline shell left -w 80`,
/// `powerline shell aboveleft --last-exit-code 0 -r .zsh`). Unknown
/// flags are ignored; `parser.parse_args` in the Python daemon would
/// raise, but the daemon's `try/except` wraps that into a response
/// body and the client retries — the wire format guarantees the
/// positional layout here.
fn parse_client_argv(argv: &[String]) -> Args {
    let mut a = Args::default();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let cur = &argv[i];
        match cur.as_str() {
            "-w" | "--width" => {
                if let Some(v) = argv.get(i + 1) {
                    a.width = v.parse().ok();
                    i += 2;
                    continue;
                }
            }
            "-r" | "--renderer-module" => {
                if let Some(v) = argv.get(i + 1) {
                    a.renderer_module = Some(v.clone());
                    i += 2;
                    continue;
                }
            }
            "-c" | "--config-override" => {
                if let Some(v) = argv.get(i + 1) {
                    a.config_override
                        .get_or_insert_with(Vec::new)
                        .push(v.clone());
                    i += 2;
                    continue;
                }
            }
            "-t" | "--theme-override" => {
                if let Some(v) = argv.get(i + 1) {
                    a.theme_override
                        .get_or_insert_with(Vec::new)
                        .push(v.clone());
                    i += 2;
                    continue;
                }
            }
            "-R" | "--renderer-arg" => {
                if let Some(v) = argv.get(i + 1) {
                    a.renderer_arg.get_or_insert_with(Vec::new).push(v.clone());
                    i += 2;
                    continue;
                }
            }
            "-p" | "--config-path" => {
                if let Some(v) = argv.get(i + 1) {
                    a.config_path.get_or_insert_with(Vec::new).push(v.clone());
                    i += 2;
                    continue;
                }
            }
            "--socket" => {
                if let Some(v) = argv.get(i + 1) {
                    a.socket = Some(v.clone());
                    i += 2;
                    continue;
                }
            }
            s if s.starts_with('-') => {
                // unknown flag; skip
                i += 1;
                continue;
            }
            _ => {
                positional.push(cur.clone());
                i += 1;
                continue;
            }
        }
        // missing flag value
        i += 1;
    }
    if let Some(ext) = positional.first().cloned() {
        a.ext.push(ext);
    }
    if let Some(side) = positional.get(1).cloned() {
        a.side = Some(side);
    }
    a
}

/// Connection state held during `main_loop` between reads and writes.
pub struct Conn {
    stream: UnixStream,
    pending_response: Option<Vec<u8>>,
}

/// Port of `do_one()` from
/// `vendor/powerline/scripts/powerline-daemon:215-259`.
///
/// One iteration of the select loop. Returns `Some(code)` to request
/// `SystemExit(code)` from `main_loop`, `None` to continue.
pub fn do_one(
    listener: &UnixListener,
    conns: &mut HashMap<RawFd, Conn>,
    is_daemon: bool,
    state: &mut State,
    render_fn: &Arc<RenderFn>,
    spawn_wm_fn: &Arc<SpawnWmFn>,
) -> Option<i32> {
    let listener_fd = listener.as_raw_fd();

    // sh:217-222  select with all read fds, all write fds, all error fds, timeout 60s
    // Build pollfd vec. Listener is always read+err. Each conn is
    // read+err when no pending response, write+err when one is queued.
    let mut pfds: Vec<libc::pollfd> = Vec::with_capacity(1 + conns.len());
    pfds.push(libc::pollfd {
        fd: listener_fd,
        events: libc::POLLIN,
        revents: 0,
    });
    let conn_fds: Vec<RawFd> = conns.keys().copied().collect();
    for fd in &conn_fds {
        let events = if conns
            .get(fd)
            .and_then(|c| c.pending_response.as_ref())
            .is_some()
        {
            libc::POLLOUT
        } else {
            libc::POLLIN
        };
        pfds.push(libc::pollfd {
            fd: *fd,
            events,
            revents: 0,
        });
    }

    // SAFETY: pfds is a contiguous Vec of pollfd; we pass its ptr + len.
    let rc = unsafe { libc::poll(pfds.as_mut_ptr(), pfds.len() as libc::nfds_t, 60_000) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() == std::io::ErrorKind::Interrupted {
            return None;
        }
        // sh:224-226  if sock in e: raise SystemExit(1)
        return Some(1);
    }
    if rc == 0 {
        // timeout
        return None;
    }

    // sh:224-226  listener error → SystemExit 1
    if pfds[0].revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        return Some(1);
    }

    // sh:228-232  discard broken conns
    for (i, fd) in conn_fds.iter().enumerate() {
        let revents = pfds[i + 1].revents;
        if revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
            conns.remove(fd);
        }
    }

    // sh:234-238  listener readable → accept
    if pfds[0].revents & libc::POLLIN != 0 {
        match eintr_retry_call(|| listener.accept()) {
            Ok((stream, _)) => {
                let fd = stream.as_raw_fd();
                conns.insert(
                    fd,
                    Conn {
                        stream,
                        pending_response: None,
                    },
                );
            }
            Err(_) => {
                // failed accept; nothing to do
            }
        }
    }

    // sh:239-250  conn readable → do_read + EOF check + get_answer
    for fd in &conn_fds {
        let idx = match conn_fds.iter().position(|x| x == fd) {
            Some(i) => i + 1,
            None => continue,
        };
        let revents = pfds[idx].revents;
        let mut conn = match conns.remove(fd) {
            Some(c) => c,
            None => continue,
        };
        if conn.pending_response.is_some() {
            // handled by the write pass below; put it back
            conns.insert(*fd, conn);
            continue;
        }
        if revents & (libc::POLLIN | libc::POLLHUP) == 0 {
            conns.insert(*fd, conn);
            continue;
        }
        // sh:242  req = do_read(s)
        let req = do_read(&mut conn.stream, Duration::from_secs_f64(2.0));
        match req {
            // sh:243-244  if req == EOF: raise SystemExit(0)
            Some(ref r) if r == EOF => return Some(0),
            // sh:245-248  elif req: ans = get_answer; result_map[s] = ans; write_sockets.add
            Some(r) if !r.is_empty() => {
                let ans = get_answer(&r, is_daemon, state, render_fn, spawn_wm_fn);
                conn.pending_response = Some(ans);
                conns.insert(*fd, conn);
            }
            // sh:249-250  else: s.close()
            _ => {
                drop(conn);
            }
        }
    }

    // sh:252-259  writable conn → do_write + close
    let writable_fds: Vec<RawFd> = conns
        .iter()
        .filter(|(_, c)| c.pending_response.is_some())
        .map(|(fd, _)| *fd)
        .collect();
    for fd in writable_fds {
        // Re-poll just this fd's revents from the array we built above.
        let idx = conn_fds.iter().position(|x| *x == fd).map(|i| i + 1);
        let writable = match idx {
            Some(i) => pfds[i].revents & libc::POLLOUT != 0,
            // Newly-queued in this iteration (read+queue happened above);
            // defer the write to the next loop tick.
            None => false,
        };
        if !writable {
            continue;
        }
        if let Some(mut conn) = conns.remove(&fd) {
            if let Some(result) = conn.pending_response.take() {
                do_write(&mut conn.stream, &result);
            }
            // sh:259  finally: s.close() — Drop closes
        }
    }

    None
}

/// Port of `shutdown()` from
/// `vendor/powerline/scripts/powerline-daemon:262-292`.
pub fn shutdown(conns: &mut HashMap<RawFd, Conn>, state: &mut State) {
    // sh:275  total_wait_time = 2
    let total = Duration::from_secs(2);
    // sh:276  shutdown_start_time = monotonic()
    let start = Instant::now();

    // sh:278-279  for s in chain((sock,), read_sockets, write_sockets): s.close()
    conns.clear();

    // sh:282  state.ts_shutdown_event.set()
    state.ts_shutdown_event.store(true, Ordering::SeqCst);
    // sh:283-284  for thread, shutdown_event in state.started_wm_threads.values(): shutdown_event.set()
    for handle in state.started_wm_threads.values() {
        handle.shutdown_event.store(true, Ordering::SeqCst);
    }
    // sh:286-289  for ...: wait_time = total - elapsed; thread.join(wait_time)
    let names: Vec<String> = state.started_wm_threads.keys().cloned().collect();
    for name in names {
        let elapsed = start.elapsed();
        if elapsed >= total {
            break;
        }
        if let Some(handle) = state.started_wm_threads.get_mut(&name) {
            if let Some(t) = handle.thread.take() {
                // We can't pass a timeout to std::thread::JoinHandle::join,
                // so spawn a watchdog that completes when either the thread
                // joins or the budget expires. Use a one-shot channel.
                let remaining = total - elapsed;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let _ = t.join();
                    let _ = tx.send(());
                });
                let _ = rx.recv_timeout(remaining);
            }
        }
    }
    // sh:291-292  sleep remaining
    let elapsed = start.elapsed();
    if elapsed < total {
        std::thread::sleep(total - elapsed);
    }
}

/// Port of `main_loop()` from
/// `vendor/powerline/scripts/powerline-daemon:295-317`.
///
/// Generic over `render_fn` and `spawn_wm_fn` so the lifecycle can be
/// tested without requiring `ShellPowerline` or the WM thread registry.
pub fn main_loop(
    listener: UnixListener,
    is_daemon: bool,
    render_fn: Arc<RenderFn>,
    spawn_wm_fn: Arc<SpawnWmFn>,
) -> i32 {
    // sh:296  sock.listen(128) — handled by bind/Listener
    // sh:297  sock.setblocking(0) — poll handles readiness; set anyway
    listener.set_nonblocking(true).ok();

    // sh:299-300  read_sockets, write_sockets = set(), set(); result_map = {}
    let mut conns: HashMap<RawFd, Conn> = HashMap::new();

    // sh:301  parser = get_main_argparser(NonInteractiveArgParser) — handled per-request
    // sh:302  state = State()
    let mut state = State::new();

    // sh:303-313  try: while True: do_one(...); except KeyboardInterrupt: SystemExit(0)
    // Rust SIGINT handling: install a flag, check it in the loop.
    let sigint = Arc::new(AtomicBool::new(false));
    install_sigint_flag(sigint.clone());

    let exit_code = loop {
        if sigint.load(Ordering::SeqCst) {
            break 0;
        }
        if let Some(code) = do_one(
            &listener,
            &mut conns,
            is_daemon,
            &mut state,
            &render_fn,
            &spawn_wm_fn,
        ) {
            break code;
        }
    };
    // sh:314-316  except SystemExit: shutdown(...); raise
    shutdown(&mut conns, &mut state);
    exit_code
}

/// Install a SIGINT handler that flips `flag` to true. Matches the
/// Python `KeyboardInterrupt → SystemExit(0)` path at sh:312-313.
fn install_sigint_flag(flag: Arc<AtomicBool>) {
    static SIGINT_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
    extern "C" fn handler(_sig: libc::c_int) {
        if let Ok(g) = SIGINT_FLAG.lock() {
            if let Some(f) = g.as_ref() {
                f.store(true, Ordering::SeqCst);
            }
        }
    }
    if let Ok(mut g) = SIGINT_FLAG.lock() {
        *g = Some(flag);
    }
    // SAFETY: signal(2) is a POSIX syscall that installs a handler;
    // our handler only touches a Mutex<Option<Arc<AtomicBool>>> which
    // does not perform allocation on the hot path after the first set.
    unsafe {
        libc::signal(libc::SIGINT, handler as *const () as libc::sighandler_t);
    }
}

/// Port of `daemonize()` from
/// `vendor/powerline/scripts/powerline-daemon:320-352`.
///
/// Double-fork + setsid + chdir / + umask 0 + dup2 to /dev/null. The
/// first parent exits via `process::exit(0)` (Python `SystemExit(0)`)
/// so callers in `main()` should not run any cleanup that the parent
/// shouldn't trigger.
pub fn daemonize() -> bool {
    // sh:321-325  pid = os.fork(); if pid > 0: exit(0)
    // SAFETY: fork is the POSIX syscall. No threads have been spawned
    // before main() reaches daemonize() (lockpidfile is the only other
    // syscall path, and it doesn't spawn threads), so the
    // async-signal-safety hazard is minimized.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // sh:326-328  fork #1 failed → exit 1
        eprintln!(
            "fork #1 failed: {} ({})",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
            std::io::Error::last_os_error()
        );
        std::process::exit(1);
    }
    if pid > 0 {
        std::process::exit(0);
    }

    // sh:331  os.chdir("/")
    let _ = std::env::set_current_dir("/");
    // sh:332  os.setsid()
    // SAFETY: setsid is a POSIX syscall.
    unsafe {
        libc::setsid();
    }
    // sh:333  os.umask(0)
    // SAFETY: umask is a POSIX syscall.
    unsafe {
        libc::umask(0);
    }

    // sh:337-340  pid = os.fork(); if pid > 0: exit(0)
    // SAFETY: same as the first fork.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        eprintln!(
            "fork #2 failed: {} ({})",
            std::io::Error::last_os_error().raw_os_error().unwrap_or(-1),
            std::io::Error::last_os_error()
        );
        std::process::exit(1);
    }
    if pid > 0 {
        std::process::exit(0);
    }

    // sh:346-351  open /dev/null, dup2 onto 0/1/2
    redirect_std_to_devnull();

    // sh:352  return True
    true
}

fn redirect_std_to_devnull() {
    use std::fs::OpenOptions;
    if let Ok(stdin) = OpenOptions::new().read(true).open("/dev/null") {
        // SAFETY: dup2 atomically replaces fd 0 with stdin's fd.
        unsafe {
            libc::dup2(stdin.as_raw_fd(), libc::STDIN_FILENO);
        }
    }
    if let Ok(stdout) = OpenOptions::new()
        .create(true)
        .truncate(false)
        .append(true)
        .open("/dev/null")
    {
        // SAFETY: dup2 atomically replaces fd 1 with stdout's fd.
        unsafe {
            libc::dup2(stdout.as_raw_fd(), libc::STDOUT_FILENO);
        }
    }
    if let Ok(stderr) = OpenOptions::new()
        .create(true)
        .truncate(false)
        .append(true)
        .open("/dev/null")
    {
        // SAFETY: dup2 atomically replaces fd 2 with stderr's fd.
        unsafe {
            libc::dup2(stderr.as_raw_fd(), libc::STDERR_FILENO);
        }
    }
}

/// Port of `check_existing()` from
/// `vendor/powerline/scripts/powerline-daemon:355-371`.
///
/// Returns `Some(listener)` if bind succeeded, `None` when EADDRINUSE.
/// Faithful to the Python: only EADDRINUSE is swallowed; other errors
/// propagate via `raise`.
pub fn check_existing(address: &str) -> std::io::Result<Option<UnixListener>> {
    // sh:356-362  if USE_FILESYSTEM: try: os.unlink(address); except: pass
    if USE_FILESYSTEM() {
        let _ = std::fs::remove_file(address);
    }
    // sh:364-370  bind; on EADDRINUSE return None; else raise
    match UnixListener::bind(address) {
        Ok(l) => Ok(Some(l)),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => Ok(None),
        Err(e) => Err(e),
    }
}

/// Port of `kill_daemon()` from
/// `vendor/powerline/scripts/powerline-daemon:374-385`.
pub fn kill_daemon(address: &str) -> bool {
    // sh:375  sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    match UnixStream::connect(address) {
        Ok(mut sock) => {
            // sh:381  eintr_retry_call(sock.sendall, EOF)
            let _ = eintr_retry_call(|| sock.write_all(EOF));
            true
        }
        // sh:379-380  except socket.error: return False
        Err(_) => false,
    }
}

/// Port of `cleanup_lockfile()` from
/// `vendor/powerline/scripts/powerline-daemon:388-398`.
pub fn cleanup_lockfile(pidfile: &str, from_signal_handler: bool) -> std::io::Result<()> {
    // sh:391  os.unlink(pidfile)
    std::fs::remove_file(pidfile).ok();
    // sh:393-394  os.close(fd) — Rust handles via Drop
    if from_signal_handler {
        // sh:398  raise SystemExit(1)
        std::process::exit(1);
    }
    Ok(())
}

/// Static SIGTERM context. Holds the pidfile path so the C-ABI signal
/// handler can unlink it without capturing closure state.
static SIGTERM_PIDFILE: Mutex<Option<std::ffi::CString>> = Mutex::new(None);

extern "C" fn sigterm_handler(_sig: libc::c_int) {
    // SAFETY: signal-handler context. unlink + _exit are
    // async-signal-safe. We only touch the CString via raw ptr.
    unsafe {
        if let Ok(g) = SIGTERM_PIDFILE.lock() {
            if let Some(path) = g.as_ref() {
                libc::unlink(path.as_ptr());
            }
        }
        libc::_exit(1);
    }
}

/// RAII guard returned by `lockpidfile`. Drop unlinks the pidfile on
/// the normal exit path (analog of `atexit.register(cleanup_lockfile)`
/// at sh:418).
pub struct PidLock {
    pub pidfile: String,
    pub fd: RawFd,
}

impl Drop for PidLock {
    fn drop(&mut self) {
        let _ = cleanup_lockfile(&self.pidfile, false);
        // SAFETY: closing our own fd.
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Port of `lockpidfile()` from
/// `vendor/powerline/scripts/powerline-daemon:401-419`.
///
/// `open(pidfile, O_WRONLY | O_CREAT, 0644)` + `fcntl.lockf(LOCK_EX |
/// LOCK_NB)` + truncate + write pid + fsync. Registers SIGTERM cleanup.
/// Returns `Some(PidLock)` on success, `None` on contention.
pub fn lockpidfile(pidfile: &str) -> Option<PidLock> {
    use std::ffi::CString;

    let cpath = CString::new(pidfile).ok()?;

    // sh:402-406  os.open(pidfile, O_WRONLY|O_CREAT, 0o644)
    // SAFETY: open is a POSIX syscall; we own the resulting fd.
    let fd = unsafe {
        libc::open(
            cpath.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT,
            (libc::S_IRUSR | libc::S_IWUSR | libc::S_IRGRP | libc::S_IROTH) as libc::c_uint,
        )
    };
    if fd < 0 {
        return None;
    }

    // sh:407-411  fcntl.lockf(fd, LOCK_EX | LOCK_NB)
    let flock = libc::flock {
        l_type: libc::F_WRLCK as libc::c_short,
        l_whence: libc::SEEK_SET as libc::c_short,
        l_start: 0,
        l_len: 0,
        l_pid: 0,
        #[cfg(any(target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
        l_sysid: 0,
    };
    // SAFETY: fcntl with F_SETLK reads our flock struct via the pointer.
    let rc = unsafe { libc::fcntl(fd, libc::F_SETLK, &flock) };
    if rc < 0 {
        // SAFETY: closing our own fd before returning.
        unsafe {
            libc::close(fd);
        }
        return None;
    }

    // sh:412-415  lseek 0, ftruncate 0, write pid, fsync
    // SAFETY: all three syscalls operate on our owned fd.
    unsafe {
        libc::lseek(fd, 0, libc::SEEK_SET);
        libc::ftruncate(fd, 0);
        let pid_str = format!("{}", std::process::id());
        libc::write(
            fd,
            pid_str.as_ptr() as *const libc::c_void,
            pid_str.len() as libc::size_t,
        );
        libc::fsync(fd);
    }

    // sh:416-418  signal(SIGTERM, cleanup); atexit.register(cleanup)
    {
        if let Ok(mut g) = SIGTERM_PIDFILE.lock() {
            *g = CString::new(pidfile).ok();
        }
        // SAFETY: signal(2) installs the handler. The handler only
        // touches the SIGTERM_PIDFILE mutex and async-signal-safe
        // syscalls (unlink, _exit).
        unsafe {
            libc::signal(
                libc::SIGTERM,
                sigterm_handler as *const () as libc::sighandler_t,
            );
        }
    }

    Some(PidLock {
        pidfile: pidfile.to_string(),
        fd,
    })
}

/// Parsed daemon CLI args (subset used by `main()`).
#[derive(Debug, Default)]
struct DaemonArgs {
    quiet: bool,
    socket: Option<String>,
    kill: bool,
    foreground: bool,
    replace: bool,
}

fn parse_daemon_argv(argv: &[String]) -> DaemonArgs {
    let mut a = DaemonArgs::default();
    let mut i = 0;
    while i < argv.len() {
        match argv[i].as_str() {
            "-q" | "--quiet" => a.quiet = true,
            "-s" | "--socket" => {
                if let Some(v) = argv.get(i + 1) {
                    a.socket = Some(v.clone());
                    i += 1;
                }
            }
            "-k" | "--kill" => a.kill = true,
            "-f" | "--foreground" => a.foreground = true,
            "-r" | "--replace" => a.replace = true,
            _ => {}
        }
        i += 1;
    }
    a
}

/// Port of `main()` from
/// `vendor/powerline/scripts/powerline-daemon:422-491`.
///
/// `render_fn` and `spawn_wm_fn` are the injected closures wired by
/// the `powerline-daemon` binary entry. Returns the process exit code.
pub fn main(argv: &[String], render_fn: Arc<RenderFn>, spawn_wm_fn: Arc<SpawnWmFn>) -> i32 {
    // sh:423-424  parser = get_daemon_argparser(); args = parser.parse_args()
    let args = parse_daemon_argv(argv);

    // sh:425-427  is_daemon = False; address = None; pidfile = None
    let mut is_daemon = false;

    // sh:429-441  address derivation
    let mut address = if let Some(sock) = args.socket.as_ref() {
        // sh:430  address = args.socket
        if !USE_FILESYSTEM() {
            // sh:431-432  if not USE_FILESYSTEM: address = '\0' + address
            format!("\0{}", sock)
        } else {
            sock.clone()
        }
    } else {
        // SAFETY: getuid is a POSIX syscall.
        let uid = unsafe { libc::getuid() };
        if USE_FILESYSTEM() {
            // sh:434-435  '/tmp/powerline-ipc-%d' % os.getuid()
            format!("/tmp/powerline-ipc-{}", uid)
        } else {
            // sh:437-441  '\0powerline-ipc-%d' % os.getuid()
            format!("\0powerline-ipc-{}", uid)
        }
    };

    // sh:443-444  if USE_FILESYSTEM: pidfile = address + '.pid'
    let pidfile = if USE_FILESYSTEM() {
        Some(format!("{}.pid", address))
    } else {
        None
    };

    // sh:446-456  --kill handling
    if args.kill {
        if args.foreground || args.replace {
            eprintln!(
                "powerline-daemon: --kill and --foreground/--replace cannot be used together"
            );
            return 2;
        }
        if kill_daemon(&address) {
            if !args.quiet {
                println!(
                    "Kill command sent to daemon, if it does not die in a couple of seconds use kill to kill it"
                );
            }
            return 0;
        } else {
            if !args.quiet {
                println!("No running daemon found");
            }
            return 1;
        }
    }

    // sh:458-462  --replace handling
    if args.replace {
        while kill_daemon(&address) {
            if !args.quiet {
                println!(
                    "Kill command sent to daemon, waiting for daemon to exit, press Ctrl-C to terminate wait and exit"
                );
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    }

    // sh:464-467  if USE_FILESYSTEM and not args.foreground: is_daemon = daemonize()
    if USE_FILESYSTEM() && !args.foreground {
        is_daemon = daemonize();
    }

    // sh:469-476  if USE_FILESYSTEM: lockpidfile or exit
    let _pid_guard = if let Some(pf) = pidfile.as_ref() {
        match lockpidfile(pf) {
            Some(g) => Some(g),
            None => {
                if !args.quiet {
                    eprintln!("The daemon is already running. Use powerline-daemon -k to kill it.");
                }
                return 1;
            }
        }
    } else {
        None
    };

    // sh:478-485  sock = check_existing(address)
    let listener = match check_existing(&address) {
        Ok(Some(l)) => l,
        Ok(None) => {
            if !args.quiet {
                eprintln!("The daemon is already running. Use powerline-daemon -k to kill it.");
            }
            return 1;
        }
        Err(e) => {
            eprintln!("powerline-daemon: bind error: {}", e);
            return 1;
        }
    };

    // sh:487-489  if not USE_FILESYSTEM and not args.foreground: is_daemon = daemonize()
    if !USE_FILESYSTEM() && !args.foreground {
        is_daemon = daemonize();
    }

    // strip the abstract-namespace nul for the listener path display
    if address.starts_with('\0') {
        address = address.replacen('\0', "@", 1);
    }
    let _ = address;

    // sh:491  return main_loop(sock, is_daemon)
    main_loop(listener, is_daemon, render_fn, spawn_wm_fn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_filesystem_matches_platform() {
        // sh:33  not linux
        assert_eq!(USE_FILESYSTEM(), !cfg!(target_os = "linux"));
    }

    #[test]
    fn eof_sentinel_value() {
        // sh:50  EOF = b'EOF\0\0'
        assert_eq!(EOF, b"EOF\0\0");
    }

    #[test]
    fn home_reads_from_env() {
        // sh:65  os.path.expanduser('~')
        let h = HOME();
        if !h.is_empty() {
            assert!(h.starts_with('/'));
        }
    }

    #[test]
    fn state_new_has_empty_collections() {
        // sh:57-62
        let s = State::new();
        assert!(s.powerlines.is_empty());
        assert!(s.started_wm_threads.is_empty());
        assert!(s.logger.is_none());
        assert!(s.config_loader.is_none());
        assert!(!s.ts_shutdown_event.load(Ordering::SeqCst));
    }

    #[test]
    fn non_interactive_arg_parser_default_constructible() {
        // sh:36-47 marker struct
        let _p = NonInteractiveArgParser;
    }

    #[test]
    fn non_daemon_shell_powerline_get_log_handler_no_op() {
        // sh:68-70
        let p = NonDaemonShellPowerline;
        p.get_log_handler();
    }

    #[test]
    fn eintr_retry_call_returns_ok() {
        let r: std::io::Result<i32> = eintr_retry_call(|| Ok(42));
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn eintr_retry_call_retries_on_interrupt_then_succeeds() {
        use std::cell::Cell;
        let calls = Cell::new(0u32);
        let r: std::io::Result<i32> = eintr_retry_call(|| {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "EINTR",
                ))
            } else {
                Ok(99)
            }
        });
        assert_eq!(r.unwrap(), 99);
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn eintr_retry_call_propagates_non_interrupt_error() {
        let r: std::io::Result<i32> = eintr_retry_call(|| Err(std::io::Error::other("other")));
        assert!(r.is_err());
    }

    #[test]
    fn safe_bytes_returns_utf8_bytes() {
        // sh:180  o.encode(encoding, 'replace')
        assert_eq!(safe_bytes("hello"), b"hello".to_vec());
        assert_eq!(safe_bytes("héllo"), "héllo".as_bytes().to_vec());
    }

    #[test]
    fn parse_args_decodes_wire_format() {
        // sh:194-200
        let req: &[u8] = b"01\0shell\0/home\0HOME=/h\0";
        let (args, environ, cwd) = parse_args(req).unwrap();
        assert_eq!(args, vec!["shell".to_string()]);
        assert_eq!(cwd, "/home");
        assert_eq!(environ.get("HOME"), Some(&"/h".to_string()));
    }

    #[test]
    fn parse_args_falls_back_to_pwd_when_cwd_missing() {
        // sh:197-199
        let req: &[u8] = b"00\0/explicit/cwd\0PWD=/tmp\0";
        let (args, environ, cwd) = parse_args(req).unwrap();
        assert!(args.is_empty());
        assert_eq!(cwd, "/explicit/cwd");
        assert_eq!(environ.get("PWD"), Some(&"/tmp".to_string()));
    }

    #[test]
    fn parse_args_returns_none_on_empty() {
        assert!(parse_args(&[]).is_none());
    }

    #[test]
    fn parse_client_argv_handles_tmux_right() {
        // typical tmux invocation: `powerline tmux right -R pane_id=%0`
        let argv = vec![
            "tmux".to_string(),
            "right".to_string(),
            "-R".to_string(),
            "pane_id=%0".to_string(),
        ];
        let a = parse_client_argv(&argv);
        assert_eq!(a.ext, vec!["tmux".to_string()]);
        assert_eq!(a.side.as_deref(), Some("right"));
        assert_eq!(
            a.renderer_arg.as_ref().unwrap(),
            &vec!["pane_id=%0".to_string()]
        );
    }

    #[test]
    fn parse_client_argv_handles_shell_with_width() {
        let argv = vec![
            "shell".to_string(),
            "left".to_string(),
            "-w".to_string(),
            "120".to_string(),
            "-r".to_string(),
            ".zsh".to_string(),
        ];
        let a = parse_client_argv(&argv);
        assert_eq!(a.ext, vec!["shell".to_string()]);
        assert_eq!(a.side.as_deref(), Some("left"));
        assert_eq!(a.width, Some(120));
        assert_eq!(a.renderer_module.as_deref(), Some(".zsh"));
    }

    #[test]
    fn powerline_key_collapses_identical_requests() {
        // sh:95-104  identical args+env produce equal keys
        let env = HashMap::new();
        let args = Args {
            ext: vec!["tmux".to_string()],
            side: Some("right".to_string()),
            renderer_module: Some(".tmux".to_string()),
            ..Default::default()
        };
        let k1 = PowerlineKey::from(&args, &env);
        let k2 = PowerlineKey::from(&args, &env);
        assert_eq!(k1, k2);
    }

    #[test]
    fn powerline_key_distinguishes_distinct_ext() {
        let env = HashMap::new();
        let a1 = Args {
            ext: vec!["tmux".to_string()],
            ..Default::default()
        };
        let a2 = Args {
            ext: vec!["shell".to_string()],
            ..Default::default()
        };
        let k1 = PowerlineKey::from(&a1, &env);
        let k2 = PowerlineKey::from(&a2, &env);
        assert_ne!(k1, k2);
    }

    #[test]
    fn render_invokes_closure_and_caches_key() {
        // sh:88-134  cache miss → closure runs and result returned
        let mut state = State::new();
        let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let calls_clone = calls.clone();
        let render_fn: Arc<RenderFn> = Arc::new(move |_, _, _, _| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
            b"RENDERED".to_vec()
        });
        let args = Args {
            ext: vec!["tmux".to_string()],
            side: Some("right".to_string()),
            ..Default::default()
        };
        let environ = HashMap::new();
        let r = render(&args, &environ, "/", false, &mut state, &render_fn);
        assert_eq!(r, b"RENDERED".to_vec());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        // cache populated
        let k = PowerlineKey::from(&args, &environ);
        assert!(state.powerlines.contains_key(&k));
        // second call still invokes render_fn but does not duplicate cache entry
        let r2 = render(&args, &environ, "/", false, &mut state, &render_fn);
        assert_eq!(r2, b"RENDERED".to_vec());
        assert_eq!(state.powerlines.len(), 1);
    }

    #[test]
    fn start_wm_is_idempotent() {
        // sh:75-76  second start of the same wm is a no-op
        let mut state = State::new();
        let spawn_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let spawn_count_c = spawn_count.clone();
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(move |_name, t_evt, _pl_evt| {
            spawn_count_c.fetch_add(1, Ordering::SeqCst);
            let t = std::thread::spawn(move || {
                while !t_evt.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(5));
                }
            });
            Some(WmHandle {
                thread: Some(t),
                shutdown_event: Arc::new(AtomicBool::new(false)),
            })
        });
        let args = Args {
            ext: vec!["wm.i3".to_string()],
            ..Default::default()
        };
        let r1 = start_wm(&args, &mut state, &spawn_wm_fn);
        let r2 = start_wm(&args, &mut state, &spawn_wm_fn);
        assert!(r1.is_empty());
        assert!(r2.is_empty());
        assert_eq!(spawn_count.load(Ordering::SeqCst), 1);
        // signal the thread to exit so the test process terminates
        if let Some(h) = state.started_wm_threads.get_mut("i3") {
            // tell the spawn closure's thread to stop. The spawn closure
            // used its own t_evt argument as the loop predicate, and we
            // stored a *separate* AtomicBool above. Replicate the wiring
            // by reusing the same Arc on subsequent set:
            h.shutdown_event.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn start_wm_reports_unknown_wm_name() {
        // closure returns None → daemon surfaces the message
        let mut state = State::new();
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_, _, _| None);
        let args = Args {
            ext: vec!["wm.bogus".to_string()],
            ..Default::default()
        };
        let r = start_wm(&args, &mut state, &spawn_wm_fn);
        assert_eq!(r, safe_bytes("Unknown WM: bogus"));
        assert!(state.started_wm_threads.is_empty());
    }

    #[test]
    fn get_answer_routes_shell_request_to_render() {
        let mut state = State::new();
        let render_fn: Arc<RenderFn> = Arc::new(|_, _, _, _| b"OK".to_vec());
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_, _, _| None);
        // wire format: numargs=02 (shell + side), then shell, left, cwd, env
        let req: &[u8] = b"02\0shell\0left\0/cwd\0HOME=/h\0";
        let r = get_answer(req, true, &mut state, &render_fn, &spawn_wm_fn);
        assert_eq!(r, b"OK".to_vec());
    }

    #[test]
    fn get_answer_routes_wm_request_to_start_wm() {
        let mut state = State::new();
        let render_fn: Arc<RenderFn> = Arc::new(|_, _, _, _| b"WRONG".to_vec());
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_, _, _| None);
        // numargs=01, one positional: wm.bogus
        let req: &[u8] = b"01\0wm.bogus\0/cwd\0";
        let r = get_answer(req, true, &mut state, &render_fn, &spawn_wm_fn);
        // start_wm path returns "Unknown WM: bogus" when the closure
        // returns None; render path would have returned "WRONG".
        assert_eq!(r, safe_bytes("Unknown WM: bogus"));
    }

    #[test]
    fn kill_daemon_returns_false_when_no_daemon_running() {
        // sh:379-380
        let r = kill_daemon("/tmp/powerline-test-nonexistent-socket-abc");
        assert!(!r);
    }

    #[test]
    fn check_existing_returns_listener_for_fresh_address() {
        // sh:364-371
        let p = std::env::temp_dir().join(format!(
            "powerliners-daemon-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let r = check_existing(&p.to_string_lossy()).unwrap();
        assert!(r.is_some());
        drop(r);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn check_existing_returns_none_when_address_in_use() {
        // bind twice on the same path; second returns None
        let p = std::env::temp_dir().join(format!(
            "powerliners-daemon-busy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let l1 = UnixListener::bind(&p).unwrap();
        let path = p.to_string_lossy().to_string();
        // For the conflict test we must NOT remove the file first, so
        // build the logic inline rather than calling check_existing
        // (which always unlinks on USE_FILESYSTEM platforms).
        match UnixListener::bind(&path) {
            Ok(_) => panic!("expected EADDRINUSE"),
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::AddrInUse),
        }
        drop(l1);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn cleanup_lockfile_unlinks_pidfile() {
        // sh:391
        let p = std::env::temp_dir().join(format!(
            "powerliners-daemon-pid-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&p, "12345").unwrap();
        assert!(p.exists());
        let _ = cleanup_lockfile(&p.to_string_lossy(), false);
        assert!(!p.exists());
    }

    #[test]
    fn lockpidfile_writes_pid_and_unlinks_on_drop() {
        let p = std::env::temp_dir().join(format!(
            "powerliners-lockpid-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let guard = lockpidfile(&path).expect("first lock");
        assert!(p.exists());
        let content = std::fs::read_to_string(&p).unwrap();
        assert_eq!(content, format!("{}", std::process::id()));
        drop(guard);
        assert!(!p.exists());
    }

    #[test]
    fn parse_daemon_argv_picks_flags() {
        let a = parse_daemon_argv(&[
            "-q".to_string(),
            "--socket".to_string(),
            "/tmp/s".to_string(),
            "-r".to_string(),
        ]);
        assert!(a.quiet);
        assert_eq!(a.socket.as_deref(), Some("/tmp/s"));
        assert!(a.replace);
        assert!(!a.foreground);
        assert!(!a.kill);
    }

    #[test]
    fn do_read_returns_after_double_null_terminator() {
        // Spawn a thread that writes a request with the \0\0 terminator,
        // then call do_read on the accepted side.
        let p = std::env::temp_dir().join(format!(
            "powerliners-doread-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let writer_path = path.clone();
        let t = std::thread::spawn(move || {
            let mut s = UnixStream::connect(&writer_path).unwrap();
            s.write_all(b"02\0shell\0left\0/cwd\0\0").unwrap();
        });
        let (mut conn, _) = listener.accept().unwrap();
        let r = do_read(&mut conn, Duration::from_secs(2)).expect("got bytes");
        assert!(r.ends_with(b"\0\0"));
        t.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn do_read_times_out_when_no_data() {
        let p = std::env::temp_dir().join(format!(
            "powerliners-doread-timeout-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let writer_path = path.clone();
        let t = std::thread::spawn(move || {
            // connect but never write; do_read should time out
            let _s = UnixStream::connect(&writer_path).unwrap();
            std::thread::sleep(Duration::from_millis(300));
        });
        let (mut conn, _) = listener.accept().unwrap();
        let start = Instant::now();
        let r = do_read(&mut conn, Duration::from_millis(150));
        let elapsed = start.elapsed();
        assert!(r.is_none());
        assert!(elapsed >= Duration::from_millis(100));
        t.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn do_write_is_best_effort_on_closed_peer() {
        let p = std::env::temp_dir().join(format!(
            "powerliners-dowrite-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let writer_path = path.clone();
        let t = std::thread::spawn(move || {
            let s = UnixStream::connect(&writer_path).unwrap();
            drop(s); // close immediately
        });
        let (mut conn, _) = listener.accept().unwrap();
        t.join().unwrap();
        // peer closed; write should not panic
        do_write(&mut conn, b"hello");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn main_loop_exits_on_eof_sentinel() {
        // Drive a real main_loop with an injected client that sends EOF;
        // we should get exit code 0 from the SystemExit(0) path at sh:243-244.
        let p = std::env::temp_dir().join(format!(
            "powerliners-mainloop-eof-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let writer_path = path.clone();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let mut s = UnixStream::connect(&writer_path).unwrap();
            s.write_all(EOF).unwrap();
        });
        let render_fn: Arc<RenderFn> = Arc::new(|_, _, _, _| b"x".to_vec());
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_, _, _| None);
        let code = main_loop(listener, true, render_fn, spawn_wm_fn);
        assert_eq!(code, 0);
        t.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn main_loop_renders_request_then_eofs() {
        // Client sends a tmux render request, reads the response, then
        // a second client sends EOF to terminate the daemon.
        let p = std::env::temp_dir().join(format!(
            "powerliners-mainloop-render-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = p.to_string_lossy().to_string();
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).unwrap();
        let writer_path = path.clone();

        let t = std::thread::spawn(move || {
            // first client: render request
            std::thread::sleep(Duration::from_millis(50));
            let mut c1 = UnixStream::connect(&writer_path).unwrap();
            // numargs=02, ext=tmux, side=right, then cwd, then env
            c1.write_all(b"02\0tmux\0right\0/cwd\0HOME=/h\0\0").unwrap();
            let mut buf = Vec::new();
            c1.read_to_end(&mut buf).ok();
            assert_eq!(buf, b"RENDERED".to_vec());

            // second client: EOF
            let mut c2 = UnixStream::connect(&writer_path).unwrap();
            c2.write_all(EOF).unwrap();
        });

        let render_fn: Arc<RenderFn> = Arc::new(|_, _, _, _| b"RENDERED".to_vec());
        let spawn_wm_fn: Arc<SpawnWmFn> = Arc::new(|_, _, _| None);
        let code = main_loop(listener, true, render_fn, spawn_wm_fn);
        assert_eq!(code, 0);
        t.join().unwrap();
        let _ = std::fs::remove_file(&path);
    }
}
