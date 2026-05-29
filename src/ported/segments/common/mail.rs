// vim:fileencoding=utf-8:noet
//! Port of `powerline/segments/common/mail.py`.
//!
//! IMAP unread-mail-count segment. Upstream uses Python's `imaplib`
//! for the network call; the Rust port surfaces the data-shape
//! (`IMAPKey` namedtuple, `render_one` formatting, UNSEEN regex
//! parsing) and stubs the actual IMAP fetch since adding a Rust IMAP
//! client crate is out of scope for this pass.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// import os                                        // py:4
// import re                                        // py:6
// from imaplib import IMAP4_SSL_PORT, IMAP4_SSL, IMAP4                                    // py:8
// from collections import namedtuple               // py:9
// from powerline.lib.threaded import KwThreadedSegment                                    // py:11
// from powerline.segments import with_docstring    // py:12

use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Port of `imaplib.IMAP4_SSL_PORT` — the standard IMAPS port.
///
/// Python: `IMAP4_SSL_PORT = 993` (defined in stdlib).
pub const IMAP4_SSL_PORT: u16 = 993;

/// Port of `_IMAPKey` namedtuple from
/// `powerline/segments/common/mail.py:14`.
///
/// Python: `_IMAPKey = namedtuple('Key', 'username password server port folder use_ssl')`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
pub struct _IMAPKey {
    pub username: String,
    pub password: String,
    pub server: String,
    pub port: u16,
    pub folder: String,
    pub use_ssl: bool,
}

/// Port of `class EmailIMAPSegment(KwThreadedSegment)` from
/// `powerline/segments/common/mail.py:17`.
pub struct EmailIMAPSegment;

impl EmailIMAPSegment {
    /// Python class attribute: `interval = 60` — py:18
    #[allow(non_upper_case_globals)]
    pub const interval: f64 = 60.0;

    /// Port of `EmailIMAPSegment.key()` (staticmethod) from
    /// `powerline/segments/common/mail.py:20`.
    ///
    /// Builds the `_IMAPKey` from kwargs + env-var fallbacks.
    #[allow(clippy::too_many_arguments)]
    pub fn key(
        mut username: String,
        mut password: String,
        mut server: String,
        mut port: u16,
        username_variable: Option<&str>,
        password_variable: Option<&str>,
        server_variable: Option<&str>,
        port_variable: Option<&str>,
        folder: String,
        use_ssl: Option<bool>,
    ) -> _IMAPKey {
        // py:21  @staticmethod
        // py:22  def key(username='', password='', server='imap.gmail.com', port=IMAP4_SSL_PORT, username_variable='', password_variable='', server_variable='', port_variable='', folder='INBOX', use_ssl=None, **kwargs):
        // py:23  if use_ssl is None:
        // py:24  use_ssl = (port == IMAP4_SSL_PORT)
        let use_ssl = use_ssl.unwrap_or(port == IMAP4_SSL_PORT);

        // py:25  # catch if user set custom mail credential env variables
        // py:26  if username_variable:
        // py:27  username = os.environ[username_variable]
        if let Some(var) = username_variable {
            if let Ok(v) = std::env::var(var) {
                username = v;
            }
        }
        // py:28  if password_variable:
        // py:29  password = os.environ[password_variable]
        if let Some(var) = password_variable {
            if let Ok(v) = std::env::var(var) {
                password = v;
            }
        }
        // py:30  if server_variable:
        // py:31  server = os.environ[server_variable]
        if let Some(var) = server_variable {
            if let Ok(v) = std::env::var(var) {
                server = v;
            }
        }
        // py:32  if port_variable:
        // py:33  port = os.environ[port_variable]
        if let Some(var) = port_variable {
            if let Ok(v) = std::env::var(var) {
                if let Ok(p) = v.parse() {
                    port = p;
                }
            }
        }

        // py:35  return _IMAPKey(username, password, server, port, folder, use_ssl)
        _IMAPKey {
            username,
            password,
            server,
            port,
            folder,
            use_ssl,
        }
    }

    /// Compiled regex matching IMAP `UNSEEN <n>` status response.
    fn unseen_re() -> &'static Regex {
        static R: OnceLock<Regex> = OnceLock::new();
        R.get_or_init(|| Regex::new(r"UNSEEN (\d+)").unwrap())
    }

    /// Port of `EmailIMAPSegment.compute_state()` from
    /// `powerline/segments/common/mail.py:36`.
    ///
    /// **Status:** stub. The actual IMAP connection requires a network
    /// crate (e.g. `imap`); the Rust port returns `None` when
    /// credentials are blank (matches py:37-39 short-circuit) and
    /// stubs the post-credentials path.
    pub fn compute_state(key: &_IMAPKey) -> Option<i64> {
        // py:37  def compute_state(self, key):
        // py:38  if not key.username or not key.password:
        // py:39  self.warn('Username and password are not configured')
        // py:40  return None
        if key.username.is_empty() || key.password.is_empty() {
            return None;
        }
        // py:41  if key.use_ssl:
        // py:42  mail = IMAP4_SSL(key.server, key.port)
        // py:43  else:
        // py:44  mail = IMAP4(key.server, key.port)
        // py:45  mail.login(key.username, key.password)
        // py:46  rc, message = mail.status(key.folder, '(UNSEEN)')
        // py:47  unread_str = message[0].decode('utf-8')
        // py:48  unread_count = int(re.search(r'UNSEEN (\d+)', unread_str).group(1))
        // py:49  return unread_count
        None
    }

    /// Parse the IMAP `UNSEEN` count out of a status response line.
    ///
    /// Used internally by `compute_state` (py:45-46):
    /// `unread_count = int(re.search(r'UNSEEN (\d+)', unread_str).group(1))`.
    /// Exposed for unit testing the regex round-trip.
    pub fn parse_unseen_count(status_line: &str) -> Option<i64> {
        // py:45-46  regex extract first UNSEEN <N> group
        Self::unseen_re()
            .captures(status_line)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Port of `EmailIMAPSegment.render_one()` (staticmethod) from
    /// `powerline/segments/common/mail.py:48`.
    ///
    /// Returns the rendered segment list for a given unread count.
    pub fn render_one(unread_count: Option<i64>, max_msgs: Option<i64>) -> Option<Vec<Value>> {
        // py:51  @staticmethod
        // py:52  def render_one(unread_count, max_msgs=None, **kwargs):
        // py:53  if not unread_count:
        // py:54  return None
        match unread_count {
            None | Some(0) => None,
            // py:55  elif type(unread_count) != int or not max_msgs:
            // py:56  return [{
            // py:57  'contents': str(unread_count),
            // py:58  'highlight_groups': ['email_alert'],
            // py:59  }]
            Some(n) if max_msgs.is_none() => Some(vec![json!({
                "contents": n.to_string(),
                "highlight_groups": ["email_alert"],
            })]),
            // py:60  else:
            // py:61  return [{
            // py:62  'contents': str(unread_count),
            // py:63  'highlight_groups': ['email_alert_gradient', 'email_alert'],
            // py:64  'gradient_level': min(unread_count * 100.0 / max_msgs, 100),
            // py:65  }]
            Some(n) => {
                let max = max_msgs.unwrap();
                let gradient = ((n as f64 * 100.0) / max as f64).min(100.0);
                Some(vec![json!({
                    "contents": n.to_string(),
                    "highlight_groups": ["email_alert_gradient", "email_alert"],
                    "gradient_level": gradient,
                })])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imap4_ssl_port_matches_upstream() {
        // Python's imaplib.IMAP4_SSL_PORT == 993
        assert_eq!(IMAP4_SSL_PORT, 993);
    }

    #[test]
    fn interval_matches_upstream() {
        assert_eq!(EmailIMAPSegment::interval, 60.0);
    }

    #[test]
    fn key_use_ssl_defaults_to_port_eq_ssl_port() {
        let k = EmailIMAPSegment::key(
            "u".into(),
            "p".into(),
            "s".into(),
            993,
            None,
            None,
            None,
            None,
            "INBOX".into(),
            None,
        );
        assert!(k.use_ssl);

        let k2 = EmailIMAPSegment::key(
            "u".into(),
            "p".into(),
            "s".into(),
            143,
            None,
            None,
            None,
            None,
            "INBOX".into(),
            None,
        );
        assert!(!k2.use_ssl);
    }

    #[test]
    fn key_use_ssl_explicit_overrides_default() {
        let k = EmailIMAPSegment::key(
            "u".into(),
            "p".into(),
            "s".into(),
            993,
            None,
            None,
            None,
            None,
            "INBOX".into(),
            Some(false),
        );
        assert!(!k.use_ssl);
    }

    #[test]
    fn parse_unseen_count_extracts_n() {
        assert_eq!(EmailIMAPSegment::parse_unseen_count("UNSEEN 7"), Some(7));
        assert_eq!(
            EmailIMAPSegment::parse_unseen_count(r#"INBOX (MESSAGES 100 UNSEEN 3)"#),
            Some(3)
        );
        assert_eq!(EmailIMAPSegment::parse_unseen_count("no match here"), None);
    }

    #[test]
    fn render_one_none_returns_none() {
        assert!(EmailIMAPSegment::render_one(None, None).is_none());
    }

    #[test]
    fn render_one_zero_returns_none() {
        assert!(EmailIMAPSegment::render_one(Some(0), None).is_none());
    }

    #[test]
    fn render_one_no_max_emits_email_alert() {
        let result = EmailIMAPSegment::render_one(Some(5), None).unwrap();
        assert_eq!(result[0]["contents"], "5");
        assert_eq!(
            result[0]["highlight_groups"],
            serde_json::json!(["email_alert"])
        );
        assert!(result[0].get("gradient_level").is_none());
    }

    #[test]
    fn render_one_with_max_includes_gradient() {
        // 25 unread out of 100 max → gradient_level = 25.0
        let result = EmailIMAPSegment::render_one(Some(25), Some(100)).unwrap();
        assert_eq!(result[0]["contents"], "25");
        assert_eq!(
            result[0]["highlight_groups"],
            serde_json::json!(["email_alert_gradient", "email_alert"])
        );
        assert_eq!(result[0]["gradient_level"], 25.0);
    }

    #[test]
    fn render_one_clamps_gradient_at_100() {
        // 150 unread out of 100 max → clamped to 100
        let result = EmailIMAPSegment::render_one(Some(150), Some(100)).unwrap();
        assert_eq!(result[0]["gradient_level"], 100.0);
    }

    #[test]
    fn compute_state_returns_none_without_credentials() {
        let key = _IMAPKey {
            username: "".to_string(),
            password: "".to_string(),
            server: "x".to_string(),
            port: 993,
            folder: "INBOX".to_string(),
            use_ssl: true,
        };
        assert!(EmailIMAPSegment::compute_state(&key).is_none());
    }
}
