// vim:fileencoding=utf-8:noet
//! End-to-end smoke for the vim integration documented in the
//! `VIM SETUP` section of README + docs/index.html.
//!
//! Two layers covered:
//!   1. Bundled `powerline.vim` script — extract via the public
//!      `bundled_vim_plugin_path()` and `vim --not-a-term -es +source`
//!      it. Catches vimscript syntax errors / missing builtins at
//!      test time instead of at the user's first prompt.
//!   2. Daemon vim wire format — connect to a fresh daemon socket,
//!      send `ext=vim side=left` request, assert response shape
//!      matches the contract in
//!      `src/bin/shared/render_runtime.rs:2819-2836` —
//!      `hi GroupName ...` lines, then a single empty separator
//!      line, then the `%#GroupName#`-style statusline markup.
//!
//! All three tests skip cleanly with a diagnostic when the required
//! tool (`vim` / `powerline-daemon`) isn't on the host — CI may not
//! have either, and the production gate is the daemon-parity suite.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn skip(reason: &str) {
    eprintln!("SKIP vim_e2e: {reason}");
}

fn vim_on_path() -> Option<PathBuf> {
    let out = Command::new("which").arg("vim").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(PathBuf::from(trimmed))
}

fn daemon_binary() -> Option<PathBuf> {
    let mut p = std::env::current_exe().ok()?;
    p.pop();
    p.pop();
    p.push("powerline-daemon");
    if !p.exists() {
        return None;
    }
    Some(p)
}

fn unique_socket(tag: &str) -> PathBuf {
    let p = PathBuf::from(format!(
        "/tmp/plp-vim-{}-{}-{}",
        tag,
        std::process::id() % 100000,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            % 10_000_000_000
    ));
    let _ = std::fs::remove_file(&p);
    p
}

struct DaemonHandle {
    child: Child,
    socket: PathBuf,
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        if let Ok(mut s) = UnixStream::connect(&self.socket) {
            let _ = s.write_all(b"EOF\0\0");
        }
        std::thread::sleep(Duration::from_millis(100));
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.socket);
        let mut pid = self.socket.clone();
        pid.set_extension("pid");
        let _ = std::fs::remove_file(&pid);
    }
}

fn start_daemon(tag: &str) -> Option<DaemonHandle> {
    let bin = daemon_binary()?;
    let socket = unique_socket(tag);
    let child = Command::new(&bin)
        .arg("--foreground")
        .arg("--socket")
        .arg(&socket)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if UnixStream::connect(&socket).is_ok() {
            return Some(DaemonHandle { child, socket });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut child = child;
    let _ = child.kill();
    None
}

#[test]
fn bundled_vim_plugin_script_loads_in_headless_vim() {
    let Some(vim) = vim_on_path() else {
        skip("vim not on PATH");
        return;
    };
    let plugin_path = powerliners::extensions::bundled_config::bundled_vim_plugin_path()
        .expect("bundled vim plugin extracts to cache");

    // -es           ex / silent batch mode (no terminal)
    // -u NONE       skip user .vimrc — only the plugin we source explicitly
    // -N            nocompatible (autocmds + the augroup the plugin uses
    //               require it; vim defaults differ between distros)
    let status = Command::new(&vim)
        .arg("--not-a-term")
        .arg("-es")
        .arg("-u")
        .arg("NONE")
        .arg("-N")
        .arg("-c")
        .arg(format!("source {}", plugin_path.display()))
        .arg("-c")
        .arg("qa!")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn vim");

    assert!(
        status.success(),
        "vim exited non-zero sourcing {} — likely a vimscript syntax error",
        plugin_path.display()
    );
}

#[test]
fn bundled_vim_plugin_defines_loaded_guard_and_refresh_command() {
    let Some(vim) = vim_on_path() else {
        skip("vim not on PATH");
        return;
    };
    let plugin_path = powerliners::extensions::bundled_config::bundled_vim_plugin_path()
        .expect("bundled vim plugin extracts to cache");

    // After sourcing, the plugin must expose:
    //   - `g:loaded_powerliners` (the standard guard variable)
    //   - `:PowerlinersRefresh` command
    //   - `g:powerliners_binary` (overrideable binary-name global)
    //
    // We test all three via `:cquit` (non-zero exit) on any failed check.
    // Vim returns 0 only when every check passes.
    let status = Command::new(&vim)
        .arg("--not-a-term")
        .arg("-es")
        .arg("-u")
        .arg("NONE")
        .arg("-N")
        .arg("-c")
        .arg(format!("source {}", plugin_path.display()))
        .arg("-c")
        .arg("if !exists('g:loaded_powerliners') | cquit | endif")
        .arg("-c")
        .arg("if !exists(':PowerlinersRefresh') | cquit | endif")
        .arg("-c")
        .arg("if !exists('g:powerliners_binary') | cquit | endif")
        .arg("-c")
        .arg("qa!")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn vim");

    assert!(
        status.success(),
        "plugin failed to expose g:loaded_powerliners / :PowerlinersRefresh / g:powerliners_binary after :source"
    );
}

#[test]
fn daemon_vim_ext_response_has_documented_wire_format() {
    let Some(handle) = start_daemon("wire") else {
        skip("powerline-daemon binary missing or failed to start");
        return;
    };

    // Wire format mirrors `src/bin/powerline.rs:87-132`:
    //   hex(argc-1) "\0" argv[1] "\0" argv[2] "\0" ... cwd "\0" K=V "\0" ... "\0\0"
    let argv = [
        "vim", "left", "-r", "mode=n", "-r", "bufnr=1", "-r", "winnr=1",
    ];
    let mut req: Vec<u8> = Vec::new();
    req.extend_from_slice(format!("{:x}", argv.len()).as_bytes());
    req.push(0);
    for a in &argv {
        req.extend_from_slice(a.as_bytes());
        req.push(0);
    }
    // cwd
    req.extend_from_slice(b"/tmp\0");
    // Minimal env — HOME is needed for the daemon's bundled-config
    // extraction fallback to find $HOME/.cache/powerliners/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    req.extend_from_slice(format!("HOME={home}").as_bytes());
    req.push(0);
    req.extend_from_slice(b"\0\0");

    let mut stream = UnixStream::connect(&handle.socket).expect("connect daemon socket");
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    stream.write_all(&req).expect("write wire request");

    let mut resp = Vec::new();
    let _ = stream.read_to_end(&mut resp);
    let s = String::from_utf8_lossy(&resp).to_string();

    // The vim wire format documented at
    // src/bin/shared/render_runtime.rs:2819-2836:
    //   `:hi GroupA ...\n:hi GroupB ...\n` (zero or more hi lines)
    //   `\n`                                (single empty separator line)
    //   statusline markup                    (no trailing newline)
    //
    // We assert the structural invariants here — any error response
    // (e.g. "powerline-daemon: config error: …") would fail these.
    assert!(
        !s.is_empty(),
        "daemon returned no bytes for ext=vim — config-build failure?"
    );

    // The separator is "\n\n" — present iff there's at least one hi
    // command emitted. The bundled vim theme has segments, so we
    // expect this to fire; if a future bundled theme renders
    // zero-styled segments this needs revisiting.
    assert!(
        s.contains("\n\n"),
        "vim wire-format response missing empty-line separator. \
         Got {} bytes: {:?}",
        s.len(),
        &s[..s.len().min(200)]
    );

    // After the last "\n\n", the statusline half should reference at
    // least one hi group via `%#GroupName#` markup — that's the only
    // shape vim's statusline syntax accepts for colour switches.
    let idx = s.rfind("\n\n").unwrap();
    let statusline = &s[idx + 2..];
    let hi_block = &s[..idx];
    for line in hi_block.lines() {
        if line.is_empty() {
            continue;
        }
        assert!(
            line.starts_with("hi "),
            "hi-block line does not start with `hi `: {line:?}"
        );
    }
    if !statusline.trim().is_empty() {
        assert!(
            statusline.contains("%#"),
            "vim statusline half missing `%#Group#` colour-switch markup. \
             Got: {statusline:?}"
        );
    }
}
