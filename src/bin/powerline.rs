// vim:fileencoding=utf-8:noet
//! Rust port of upstream's `client/powerline.c`.
//!
//! The thin native client that tmux / shells invoke via
//! `$POWERLINE_COMMAND`. Builds the powerline-daemon wire request from
//! argv + cwd + env, sends it over the Unix socket, and copies the
//! response to stdout. Falls back to `execvp("powerline-render", …)`
//! when the daemon isn't reachable, mirroring upstream C client at
//! `client/powerline.c:115-124`.
//!
//! Wire format (per the C client, lines 126-148):
//!   `hex(argc-1)` "\0" `argv[0]` "\0" `argv[1]` "\0" … cwd "\0" `env[0]` "\0" … "\0\0"
//!
//! Socket path:
//!   - macOS / BSD: filesystem socket `/tmp/powerline-ipc-<uid>`
//!   - Linux: abstract socket `\0powerline-ipc-<uid>`
//!   - Either is overridable via the leading `--socket PATH` argument
//!     (per C client lines 98-105).

use std::ffi::CString;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::ExitCode;

#[path = "shared/house_help.rs"]
mod house_help;

// Same source as the daemon's bind path (`crate::extensions::
// ipc_socket`), included by path so this client stays a standalone
// binary that doesn't link the library.
// (`bind` is daemon-only; the client just connects.)
#[allow(dead_code)]
#[path = "../extensions/ipc_socket.rs"]
mod ipc_socket;

#[cfg(target_os = "linux")]
const SOCKET_PREFIX: &str = "\0powerline-ipc-";
#[cfg(not(target_os = "linux"))]
const SOCKET_PREFIX: &str = "/tmp/powerline-ipc-";

fn default_socket_path() -> String {
    // C:103  snprintf(address_buf, ADDRESS_SIZE, ADDRESS_TEMPLATE, getuid())
    // SAFETY: getuid is a thread-safe POSIX syscall with no preconditions.
    let uid = unsafe { libc::getuid() };
    format!("{}{}", SOCKET_PREFIX, uid)
}

fn main() -> ExitCode {
    // C:78-105  argv parsing
    let mut argv: Vec<String> = std::env::args().collect();

    // House `--help` / `--version` (display-only; never touches the wire
    // request or the render fallback). tmux / shells invoke the client
    // only as `powerline EXT SIDE …`, never with these flags, so the
    // normal statusline path is unaffected.
    if house_help::wants_help(&argv[1..]) {
        print!("{}", client_help());
        return ExitCode::SUCCESS;
    }
    if house_help::wants_version(&argv[1..]) {
        println!("{}", house_help::version_line("powerline"));
        return ExitCode::SUCCESS;
    }

    // C:98-105  --socket SOCKET
    let address = if argv.len() > 3 && argv[1] == "--socket" {
        let s = argv[2].clone();
        // Drop argv[1] ("--socket") and argv[2] (the path).
        argv.drain(1..=2);
        s
    } else {
        default_socket_path()
    };

    if argv.len() < 2 {
        // C:93-96  Must provide at least one argument
        println!("Must provide at least one argument.");
        return ExitCode::from(1);
    }

    // C:107-124  socket() + connect() with fallback to powerline-render.
    // `ipc_socket::connect` is the same helper the daemon binds with,
    // so client and daemon agree on abstract vs filesystem addresses.
    let mut stream = match ipc_socket::connect(&address) {
        Ok(s) => s,
        Err(_) => {
            // C:117-123  We failed to connect to the daemon, execute
            // powerline-render instead.
            return exec_render_fallback(&argv);
        }
    };

    // C:126-127  hex(argc-1)
    let num_args = format!("{:x}", argv.len() - 1);
    if stream.write_all(num_args.as_bytes()).is_err() {
        return write_failed();
    }
    // C:128  write the separator NUL.
    if stream.write_all(b"\0").is_err() {
        return write_failed();
    }

    // C:130-133  for argv[1..] write each + NUL.
    for arg in &argv[1..] {
        if stream.write_all(arg.as_bytes()).is_err() {
            return write_failed();
        }
        if stream.write_all(b"\0").is_err() {
            return write_failed();
        }
    }

    // C:135-141  cwd + NUL.
    if let Ok(cwd) = std::env::current_dir() {
        if stream.write_all(cwd.as_os_str().as_bytes()).is_err() {
            return write_failed();
        }
    }
    if stream.write_all(b"\0").is_err() {
        return write_failed();
    }

    // C:143-146  env "K=V" + NUL each.
    for (k, v) in std::env::vars_os() {
        let mut item: Vec<u8> = Vec::with_capacity(k.len() + 1 + v.len() + 1);
        item.extend_from_slice(k.as_bytes());
        item.push(b'=');
        item.extend_from_slice(v.as_bytes());
        item.push(0);
        if stream.write_all(&item).is_err() {
            return write_failed();
        }
    }

    // C:148  double NUL EOF marker (request terminator).
    if stream.write_all(b"\0\0").is_err() {
        return write_failed();
    }

    // C:150-159  copy response bytes to stdout in 4 KiB chunks.
    let mut stdout = std::io::stdout().lock();
    let mut buf = [0u8; 4096];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if stdout.write_all(&buf[..n]).is_err() {
                    return ExitCode::from(1);
                }
            }
            Err(_) => return ExitCode::from(1),
        }
    }

    ExitCode::SUCCESS
}

/// Mirror of the C client's `execvp("powerline-render", newargv)` at
/// `client/powerline.c:121-123`. Replaces the
/// current process image, so on success this never returns.
fn exec_render_fallback(argv: &[String]) -> ExitCode {
    let prog = CString::new("powerline-render").expect("nul in program name");
    // newargv[0] = "powerline-render"; newargv[1..argc] = argv[1..];
    let mut c_args: Vec<CString> = vec![prog.clone()];
    for a in &argv[1..] {
        c_args.push(match CString::new(a.as_str()) {
            Ok(c) => c,
            Err(_) => return ExitCode::from(1),
        });
    }
    let mut ptrs: Vec<*const libc::c_char> = c_args.iter().map(|c| c.as_ptr()).collect();
    ptrs.push(std::ptr::null());
    // SAFETY: execvp takes a NUL-terminated argv pointer array. The
    // CString backing memory outlives the syscall because c_args owns
    // it and shadows the only return path.
    unsafe {
        libc::execvp(prog.as_ptr(), ptrs.as_ptr());
    }
    // If we reach here, execvp failed.
    eprintln!("powerline: execvp(powerline-render) failed");
    ExitCode::from(1)
}

fn write_failed() -> ExitCode {
    eprintln!("powerline: write() to daemon failed");
    ExitCode::from(1)
}

/// House `--help` screen for the native client. EXT/SIDE and the render
/// flags are forwarded verbatim to the daemon (or to `powerline-render`
/// when the socket is unreachable), so they are documented here.
fn client_help() -> String {
    use house_help::Section;
    house_help::help(
        "Native client — sends EXT/SIDE + flags to powerline-daemon; \
         falls back to powerline-render when the socket is unreachable.",
        "STATUSLINE CLIENT // ONE SOCKET, ONE PROMPT",
        "powerline [--socket PATH] EXT [SIDE] [OPTIONS]",
        &[
            Section {
                title: "MODES",
                items: &[
                    (
                        "powerline EXT [SIDE]",
                        "render EXT (tmux, shell, …) for SIDE (left/right)",
                    ),
                    ("powerline tmux right", "tmux right-hand statusline"),
                    ("powerline shell left", "shell left prompt"),
                    ("powerline --socket P …", "target a specific daemon socket"),
                ],
            },
            Section {
                title: "OPTIONS (forwarded to daemon / render)",
                items: &[
                    ("-w, --width N", "truncate output to N columns"),
                    ("-r, --renderer-module M", "override the renderer module"),
                    (
                        "-c, --config-override K=V",
                        "override main-config keys (repeatable)",
                    ),
                    (
                        "-t, --theme-override K=V",
                        "override theme keys (repeatable)",
                    ),
                    ("-R, --renderer-arg K=V", "pass an argument to the renderer"),
                    (
                        "-p, --config-path PATH",
                        "add a config search path (repeatable)",
                    ),
                    ("-m, --mode MODE", "shorthand for -R mode=MODE"),
                    ("--last-exit-code N", "previous command exit status"),
                    ("--last-pipe-status L", "previous pipe exit statuses"),
                    ("--jobnum N", "number of background jobs"),
                    ("--socket PATH", "daemon socket path"),
                    ("-h, --help", "print this help"),
                    ("-V, --version", "print version"),
                ],
            },
        ],
        "One socket. One binary. The whole statusline.",
        "JACK IN. ONE SOCKET. RENDER THE PROMPT.",
    )
}
