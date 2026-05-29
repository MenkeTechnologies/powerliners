// vim:fileencoding=utf-8:noet
//! Port of `powerline/lib/url.py`.
//!
//! Used by the weather segment (`segments/common/wthr.py`) and the
//! IP-info / time-zone lookups in `segments/common/net.py` to fetch
//! small JSON responses from external APIs.
//!
//! Rust stdlib has no HTTP client. A faithful port would use `ureq`,
//! `reqwest`, or `hyper` — adding one of those is the right move when
//! the weather/net segments land. Until then, `urllib_read` is a
//! documented no-op returning `None` (which matches the upstream
//! HTTPError branch behaviour at py:16-17).
//!
//! `urllib_urlencode` is a pure-stdlib query-string formatter; it
//! ports independently of the HTTP client.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:2
// try:                                              // py:4
//     from urllib.error import HTTPError            // py:5
//     from urllib.request import urlopen            // py:6
//     from urllib.parse import urlencode as urllib_urlencode  // py:7
// except ImportError:                               // py:8
//     from urllib2 import urlopen, HTTPError        // py:9
//     from urllib import urlencode as urllib_urlencode  // py:10

/// Port of `urllib_read()` from `powerline/lib/url.py:13`.
///
/// Python:
/// ```python
/// def urllib_read(url):
///     try:
///         return urlopen(url, timeout=10).read().decode('utf-8')
///     except HTTPError:
///         return
/// ```
///
/// **Status:** stub — returns `None` (matches the HTTPError branch
/// behaviour at py:16-17). A real port requires an HTTP-client crate;
/// `ureq` is the recommended pick (smallest, blocking, no async
/// runtime baggage). The weather / IP-geolocation segments that
/// depend on this are deferred to Phase 3 of PORT_PLAN.md.
pub fn urllib_read(_url: &str) -> Option<String> {
    // py:14-17  try urlopen → return body; HTTPError → return None
    // Rust stub: behave as if every call hit HTTPError.
    None
}

/// Port of module-level binding `urllib_urlencode` from
/// `powerline/lib/url.py:7` (aliased from `urllib.parse.urlencode`).
///
/// Builds an URL-encoded query string from an iterable of
/// `(key, value)` pairs. Python's stdlib implementation handles
/// percent-escaping per RFC 3986; we replicate it here against
/// `std::collections::HashMap` / iterables.
pub fn urllib_urlencode<I, K, V>(items: I) -> String
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    items
        .into_iter()
        .map(|(k, v)| {
            format!(
                "{}={}",
                percent_encode(k.as_ref()),
                percent_encode(v.as_ref())
            )
        })
        .collect::<Vec<_>>()
        .join("&")
}

/// Inlined percent-encoding helper. Matches `urllib.parse.quote_plus`
/// for the `safe=''` default used by `urlencode`.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'), // urlencode replaces ' ' with '+'
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `urllib_read` returns None until the HTTP client is wired.
    #[test]
    fn urllib_read_returns_none_stub() {
        assert!(urllib_read("https://example.invalid/").is_none());
    }

    #[test]
    fn urllib_urlencode_simple() {
        let pairs = vec![("a", "1"), ("b", "2")];
        assert_eq!(urllib_urlencode(pairs), "a=1&b=2");
    }

    #[test]
    fn urllib_urlencode_escapes_spaces_as_plus() {
        let pairs = vec![("q", "hello world")];
        assert_eq!(urllib_urlencode(pairs), "q=hello+world");
    }

    #[test]
    fn urllib_urlencode_percent_escapes_non_safe() {
        let pairs = vec![("k", "a/b?c=d")];
        // / ? = → %2F %3F %3D
        assert_eq!(urllib_urlencode(pairs), "k=a%2Fb%3Fc%3Dd");
    }

    #[test]
    fn urllib_urlencode_preserves_safe_chars() {
        let pairs = vec![("k", "abc_-.~0")];
        assert_eq!(urllib_urlencode(pairs), "k=abc_-.~0");
    }
}
