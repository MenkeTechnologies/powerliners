// vim:fileencoding=utf-8:noet
//! Minimal IMAP(S) client behind the mail segment.
//!
//! Python's `EmailIMAPSegment` rides on the stdlib `imaplib`; the Rust
//! port has no stdlib equivalent, so the commands python's path issues
//! (server greeting, `LOGIN`, `STATUS <folder> (UNSEEN)`) are spoken
//! directly over `rustls` — no IMAP client crate. The ported
//! `compute_state` in `src/ported/segments/common/mail.rs` delegates
//! to [`unseen`]; this module is infrastructure rather than a port,
//! which is why it lives in `extensions` (the sanctioned non-port
//! location per `docs/PORT.md`).

use crate::ported::segments::common::mail::EmailIMAPSegment;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Overall budget for one probe: resolve + connect + TLS + LOGIN +
/// STATUS. Python has no timeout (KwThreadedSegment's worker thread
/// absorbs a hang); callers render synchronously, so the session gets
/// a hard deadline instead.
const PROBE_DEADLINE: Duration = Duration::from_secs(8);

/// mail.py:41-49 — connect (SSL when `use_ssl`), log in, read
/// `STATUS <folder> (UNSEEN)`. Every error collapses to `Err` so the
/// caller surfaces `None`, matching python's hide-on-failure.
pub fn unseen(
    server: &str,
    port: u16,
    use_ssl: bool,
    username: &str,
    password: &str,
    folder: &str,
) -> Result<i64, String> {
    let deadline = Instant::now() + PROBE_DEADLINE;
    let tcp = tcp_connect(server, port, deadline)?;
    // Separate handle purely to tune socket timeouts while the read
    // side is borrowed by the session; clones share the same socket.
    let sock = tcp.try_clone().map_err(|e| format!("clone: {e}"))?;
    if use_ssl {
        // py:41  if key.use_ssl:
        // py:42  mail = IMAP4_SSL(key.server, key.port)
        let mut tls = tls_connect(tcp, server)?;
        run_session(&mut tls, &sock, deadline, username, password, folder)
    } else {
        // py:43  else:
        // py:44  mail = IMAP4(key.server, key.port)
        // python never issues STARTTLS either — plaintext mirrors it.
        let mut plain = tcp;
        run_session(&mut plain, &sock, deadline, username, password, folder)
    }
}

/// Resolve and connect within the remaining deadline budget.
fn tcp_connect(server: &str, port: u16, deadline: Instant) -> Result<TcpStream, String> {
    let rem = deadline.saturating_duration_since(Instant::now());
    if rem.is_zero() {
        return Err("deadline before connect".to_string());
    }
    let addrs = (server, port)
        .to_socket_addrs()
        .map_err(|e| format!("resolve: {e}"))?;
    let mut last = "no addresses".to_string();
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, rem) {
            Ok(s) => return Ok(s),
            Err(e) => last = format!("connect: {e}"),
        }
    }
    Err(last)
}

/// Wrap the socket in rustls with the web roots (the same CA bundle
/// verification python's ssl context performs).
fn tls_connect(
    tcp: TcpStream,
    server: &str,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, String> {
    static CFG: OnceLock<std::sync::Arc<rustls::ClientConfig>> = OnceLock::new();
    let cfg = CFG.get_or_init(|| {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        std::sync::Arc::new(
            rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        )
    });
    let name = rustls::pki_types::ServerName::try_from(server.to_string())
        .map_err(|e| format!("sni: {e}"))?;
    let conn = rustls::ClientConnection::new(cfg.clone(), name).map_err(|e| format!("tls: {e}"))?;
    Ok(rustls::StreamOwned::new(conn, tcp))
}

/// One tagged exchange: server greeting, `a001 LOGIN` (py:45),
/// `a002 STATUS <folder> (UNSEEN)` (py:46), best-effort LOGOUT.
fn run_session<S: Read + Write>(
    io: &mut S,
    sock: &TcpStream,
    deadline: Instant,
    username: &str,
    password: &str,
    folder: &str,
) -> Result<i64, String> {
    // py:42/44 — imaplib reads the greeting before logging in.
    let greeting = read_line(io, sock, deadline)?;
    if !greeting.starts_with("* OK") && !greeting.starts_with("* PREAUTH") {
        return Err(format!("bad greeting: {}", clip(&greeting)));
    }
    // py:45  mail.login(username, password)
    if !greeting.starts_with("* PREAUTH") {
        write_line(
            io,
            sock,
            deadline,
            &format!(
                "a001 LOGIN {} {}",
                quote_imap(username),
                quote_imap(password)
            ),
        )?;
        wait_tag(io, sock, deadline, "a001")?;
    }
    // py:46  rc, message = mail.status(folder, '(UNSEEN)')
    write_line(
        io,
        sock,
        deadline,
        &format!("a002 STATUS {} (UNSEEN)", quote_imap(folder)),
    )?;
    let mut status = None;
    loop {
        let line = read_line(io, sock, deadline)?;
        if let Some(rest) = line.strip_prefix("a002 ") {
            if !rest.starts_with("OK") {
                return Err(format!("status rejected: {}", clip(&line)));
            }
            break;
        }
        if line.starts_with("* STATUS") {
            status = Some(line);
        }
    }
    // Best-effort teardown — the count is already in hand.
    let _ = write_line(io, sock, deadline, "a003 LOGOUT");
    let line = status.ok_or_else(|| "no STATUS response".to_string())?;
    // py:47  unread_str = message[0].decode('utf-8')
    // py:48  unread_count = int(re.search(r'UNSEEN (\d+)', unread_str).group(1))
    EmailIMAPSegment::parse_unseen_count(&line).ok_or_else(|| "no UNSEEN group".to_string())
}

/// Read until the tagged response for `tag`; `NO`/`BAD` are errors
/// (python's `login`/`status` raise on them).
fn wait_tag<S: Read + Write>(
    io: &mut S,
    sock: &TcpStream,
    deadline: Instant,
    tag: &str,
) -> Result<(), String> {
    loop {
        let line = read_line(io, sock, deadline)?;
        if let Some(rest) = line.strip_prefix(tag).and_then(|r| r.strip_prefix(' ')) {
            return if rest.starts_with("OK") {
                Ok(())
            } else {
                Err(format!("rejected: {}", clip(&line)))
            };
        }
    }
}

/// Read one CRLF-terminated line, bounded by the deadline (the socket
/// read timeout is re-armed to the remaining budget per call).
fn read_line<S: Read>(io: &mut S, sock: &TcpStream, deadline: Instant) -> Result<String, String> {
    let rem = deadline.saturating_duration_since(Instant::now());
    if rem.is_zero() {
        return Err("probe deadline".to_string());
    }
    sock.set_read_timeout(Some(rem))
        .map_err(|e| format!("timeout: {e}"))?;
    let mut raw = Vec::new();
    let mut b = [0u8; 1];
    loop {
        match io.read(&mut b) {
            Ok(0) => return Err("eof".to_string()),
            Ok(_) => {}
            Err(e) => return Err(format!("read: {e}")),
        }
        if b[0] == b'\n' {
            break;
        }
        raw.push(b[0]);
        if raw.len() > 16 * 1024 {
            return Err("line too long".to_string());
        }
    }
    while raw.last() == Some(&b'\r') {
        raw.pop();
    }
    String::from_utf8(raw).map_err(|_| "non-utf8 line".to_string())
}

/// Write one CRLF-terminated command.
fn write_line<S: Write>(
    io: &mut S,
    sock: &TcpStream,
    deadline: Instant,
    line: &str,
) -> Result<(), String> {
    let rem = deadline.saturating_duration_since(Instant::now());
    if rem.is_zero() {
        return Err("probe deadline".to_string());
    }
    sock.set_write_timeout(Some(rem))
        .map_err(|e| format!("timeout: {e}"))?;
    io.write_all(line.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    io.write_all(b"\r\n").map_err(|e| format!("write: {e}"))?;
    io.flush().map_err(|e| format!("flush: {e}"))
}

/// IMAP quoted-string (RFC 3501 §4.3) — imaplib quotes credentials
/// the same way for ascii values.
fn quote_imap(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Truncate server text for diagnostics — never log whole banners.
fn clip(s: &str) -> String {
    s.chars().take(80).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted peer: preloaded server responses, captured writes.
    struct Scripted {
        rd: std::io::Cursor<Vec<u8>>,
        wr: Vec<u8>,
    }

    impl Scripted {
        fn new(server_script: &str) -> Self {
            Self {
                rd: std::io::Cursor::new(server_script.as_bytes().to_vec()),
                wr: Vec::new(),
            }
        }
    }

    impl Read for Scripted {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.rd.read(buf)
        }
    }

    impl Write for Scripted {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.wr.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A real socket handle (timeouts are the only use of it in the
    /// session; no traffic crosses it in these tests).
    fn dummy_sock() -> TcpStream {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        TcpStream::connect(l.local_addr().unwrap()).unwrap()
    }

    fn deadline_5s() -> Instant {
        Instant::now() + Duration::from_secs(5)
    }

    #[test]
    fn session_login_status_parses_unseen() {
        let mut s = Scripted::new(concat!(
            "* OK [CAPABILITY IMAP4rev1] Gimap ready\r\n",
            "a001 OK logged in\r\n",
            "* STATUS \"INBOX\" (MESSAGES 9 UNSEEN 2)\r\n",
            "a002 OK status completed\r\n",
        ));
        let sock = dummy_sock();
        let n = run_session(&mut s, &sock, deadline_5s(), "u", "p", "INBOX").unwrap();
        assert_eq!(n, 2);
        let sent = String::from_utf8(s.wr).unwrap();
        assert!(sent.contains("a001 LOGIN \"u\" \"p\""));
        assert!(sent.contains("a002 STATUS \"INBOX\" (UNSEEN)"));
        assert!(sent.contains("a003 LOGOUT"));
    }

    #[test]
    fn session_rejects_failed_login() {
        let mut s = Scripted::new(concat!(
            "* OK ready\r\n",
            "a001 NO [AUTHENTICATIONFAILED] Invalid credentials (Failure)\r\n",
        ));
        let sock = dummy_sock();
        let err = run_session(&mut s, &sock, deadline_5s(), "u", "bad", "INBOX").unwrap_err();
        assert!(err.contains("AUTHENTICATIONFAILED"), "got: {err}");
    }

    #[test]
    fn session_errors_when_status_lacks_unseen() {
        let mut s = Scripted::new(concat!(
            "* OK ready\r\n",
            "a001 OK logged in\r\n",
            "* STATUS \"INBOX\" (MESSAGES 4)\r\n",
            "a002 OK\r\n",
        ));
        let sock = dummy_sock();
        let err = run_session(&mut s, &sock, deadline_5s(), "u", "p", "INBOX").unwrap_err();
        assert_eq!(err, "no UNSEEN group");
    }

    #[test]
    fn session_rejects_bad_greeting() {
        let mut s = Scripted::new("* BYE server shutting down\r\n");
        let sock = dummy_sock();
        let err = run_session(&mut s, &sock, deadline_5s(), "u", "p", "INBOX").unwrap_err();
        assert!(err.contains("bad greeting"), "got: {err}");
    }

    #[test]
    fn quote_imap_escapes_quotes_and_backslashes() {
        assert_eq!(quote_imap(r#"a"b\c"#), r#""a\"b\\c""#);
        assert_eq!(quote_imap("plain"), "\"plain\"");
    }
}
