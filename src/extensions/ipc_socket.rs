// vim:fileencoding=utf-8:noet
//! Unix-socket bind/connect that understands the Linux abstract
//! namespace. Sanctioned non-port location per `docs/PORT.md`.
//!
//! Upstream Python passes `b'\0powerline-ipc-<uid>'` straight to
//! `socket.bind()`, which CPython routes into the abstract namespace
//! because it accepts raw `bytes` addresses. Rust's
//! `UnixListener::bind` / `UnixStream::connect` take a path and reject
//! any interior NUL with
//! `InvalidInput: paths must not contain interior null bytes`
//! (`library/std/src/os/unix/net/addr.rs`), so the Linux default
//! address could never bind. Abstract addresses go through
//! `SocketAddr::from_abstract_name` + `bind_addr`/`connect_addr`
//! instead, which is wire-identical to CPython: both put exactly the
//! post-NUL bytes in `sun_path[1..]` and size `sockaddr_un` to that
//! length, so the Rust daemon and the C/Python clients meet on the
//! same address.
//!
//! Addresses without a leading NUL are ordinary filesystem sockets and
//! take the plain path API on every platform.

use std::io;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};

/// Build a `SocketAddr` for the abstract-namespace `name` (the bytes
/// after the leading NUL).
#[cfg(target_os = "linux")]
fn abstract_addr(name: &str) -> io::Result<SocketAddr> {
    use std::os::linux::net::SocketAddrExt;
    SocketAddr::from_abstract_name(name.as_bytes())
}

/// Non-Linux kernels have no abstract namespace; a NUL-prefixed
/// address is unusable rather than silently truncated to `""`.
#[cfg(not(target_os = "linux"))]
fn abstract_addr(_name: &str) -> io::Result<SocketAddr> {
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "abstract-namespace sockets are Linux-only",
    ))
}

/// Listen on `address`, routing `\0name` into the abstract namespace.
pub fn bind(address: &str) -> io::Result<UnixListener> {
    match address.strip_prefix('\0') {
        Some(name) => UnixListener::bind_addr(&abstract_addr(name)?),
        None => UnixListener::bind(address),
    }
}

/// Connect to `address`, routing `\0name` into the abstract namespace.
pub fn connect(address: &str) -> io::Result<UnixStream> {
    match address.strip_prefix('\0') {
        Some(name) => UnixStream::connect_addr(&abstract_addr(name)?),
        None => UnixStream::connect(address),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Unique abstract name per test process, so concurrent runs (and
    /// the user's live daemon on `powerline-ipc-<uid>`) never collide.
    #[cfg(target_os = "linux")]
    fn abstract_name(tag: &str) -> String {
        format!("\0powerliners-test-{}-{}", tag, std::process::id())
    }

    #[test]
    fn filesystem_socket_round_trips() {
        let dir = std::env::temp_dir().join(format!("powerliners-ipc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sock");
        let addr = path.to_str().unwrap().to_string();

        let listener = bind(&addr).expect("bind filesystem socket");
        let client = std::thread::spawn(move || {
            let mut s = connect(&addr).expect("connect filesystem socket");
            s.write_all(b"ping").unwrap();
        });
        let (mut conn, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4];
        conn.read_exact(&mut buf).unwrap();
        client.join().unwrap();

        assert_eq!(&buf, b"ping");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// The regression: `UnixListener::bind("\0name")` fails with
    /// "paths must not contain interior null bytes", which is exactly
    /// what `powerline-daemon -q` hit on Linux.
    #[cfg(target_os = "linux")]
    #[test]
    fn abstract_socket_round_trips() {
        let addr = abstract_name("roundtrip");

        assert!(
            UnixListener::bind(&addr).is_err(),
            "plain path bind must still reject the NUL — otherwise this helper is unnecessary"
        );

        let listener = bind(&addr).expect("bind abstract socket");
        let client_addr = addr.clone();
        let client = std::thread::spawn(move || {
            let mut s = connect(&client_addr).expect("connect abstract socket");
            s.write_all(b"ping").unwrap();
        });
        let (mut conn, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4];
        conn.read_exact(&mut buf).unwrap();
        client.join().unwrap();

        assert_eq!(&buf, b"ping");
    }

    /// A second bind on a live abstract address must surface
    /// `AddrInUse` so `check_existing()` can report "already running"
    /// instead of dying with a bind error.
    #[cfg(target_os = "linux")]
    #[test]
    fn abstract_socket_second_bind_is_addr_in_use() {
        let addr = abstract_name("inuse");
        let _held = bind(&addr).expect("first bind");
        let err = bind(&addr).expect_err("second bind must fail");
        assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
    }

    /// Connecting to an abstract name nobody is listening on fails
    /// (client falls back to `powerline-render`, daemon `--kill`
    /// reports "No running daemon found").
    #[cfg(target_os = "linux")]
    #[test]
    fn abstract_socket_connect_without_listener_fails() {
        assert!(connect(&abstract_name("nolistener")).is_err());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn abstract_address_rejected_off_linux() {
        let err = bind("\0powerliners-test").expect_err("no abstract namespace off Linux");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
