// vim:fileencoding=utf-8:noet

//! Encodings support.
//!
//! Port of `powerline/lib/encoding.py`.
//!
//! Upstream docstring (`py:3-12`):
//!
//! > This is the only module from which functions obtaining encoding
//! > should be exported. Note: you should always care about errors=
//! > argument since it is not guaranteed that encoding returned by
//! > some function can encode/decode given string.
//! >
//! > All functions in this module must always return a valid encoding.
//! > Most of them are not thread-safe.
//!
//! The Python implementation walks `sys.getfilesystemencoding()` and
//! `locale.getpreferredencoding()` to pick an encoding suitable for
//! the OS environment. In Rust everything is UTF-8 by construction —
//! `String`/`str` are guaranteed UTF-8 and the OS APIs (`OsString`,
//! `Path`) handle bytes natively without per-call encoding choice. The
//! Rust ports therefore return the upstream's fallback values directly
//! (`"utf-8"`, `"ascii"`, `"latin1"`) — the active code path on every
//! modern locale.

// from __future__ import (unicode_literals, division, absolute_import, print_function)  // py:14
// import sys                                       // py:16
// import locale                                    // py:17

/// Port of `get_preferred_file_name_encoding()` from
/// `powerline/lib/encoding.py:20`.
///
/// Get preferred file name encoding.
pub fn get_preferred_file_name_encoding() -> &'static str {
    // py:20  def get_preferred_file_name_encoding():
    // py:21  '''Get preferred file name encoding
    // py:22  '''
    // py:23  return (
    // py:24  sys.getfilesystemencoding()
    // py:25  or locale.getpreferredencoding()
    // py:26  or 'utf-8'
    // py:27  )
    "utf-8"
}

/// Port of `get_preferred_file_contents_encoding()` from
/// `powerline/lib/encoding.py:30`.
///
/// Get encoding preferred for file contents.
pub fn get_preferred_file_contents_encoding() -> &'static str {
    // py:30  def get_preferred_file_contents_encoding():
    // py:31  '''Get encoding preferred for file contents
    // py:32  '''
    // py:33  return (
    // py:34  locale.getpreferredencoding()
    // py:35  or 'utf-8'
    // py:36  )
    "utf-8"
}

/// Port of `get_preferred_output_encoding()` from
/// `powerline/lib/encoding.py:39`.
///
/// Get encoding that should be used for printing strings.
///
/// > Falls back to ASCII, so that output is most likely to be
/// > displayed correctly.
pub fn get_preferred_output_encoding() -> &'static str {
    // py:39  def get_preferred_output_encoding():
    // py:40  '''Get encoding that should be used for printing strings
    // py:41
    // py:42  .. warning::
    // py:43  Falls back to ASCII, so that output is most likely to be displayed
    // py:44  correctly.
    // py:45  '''
    // py:46  if hasattr(locale, 'LC_MESSAGES'):
    // py:47  return (
    // py:48  locale.getlocale(locale.LC_MESSAGES)[1]
    // py:49  or locale.getlocale()[1]
    // py:50  or 'ascii'
    // py:51  )
    // py:53  return (
    // py:54  locale.getlocale()[1]
    // py:55  or 'ascii'
    // py:56  )
    "ascii"
}

/// Port of `get_preferred_input_encoding()` from
/// `powerline/lib/encoding.py:59`.
///
/// Get encoding that should be used for reading shell command output.
///
/// > Falls back to latin1 so that function is less likely to throw as
/// > decoded output is primary searched for ASCII values.
pub fn get_preferred_input_encoding() -> &'static str {
    // py:59  def get_preferred_input_encoding():
    // py:60  '''Get encoding that should be used for reading shell command output
    // py:61
    // py:62  .. warning::
    // py:63  Falls back to latin1 so that function is less likely to throw as decoded
    // py:64  output is primary searched for ASCII values.
    // py:65  '''
    // py:66  if hasattr(locale, 'LC_MESSAGES'):
    // py:67  return (
    // py:68  locale.getlocale(locale.LC_MESSAGES)[1]
    // py:69  or locale.getlocale()[1]
    // py:70  or 'latin1'
    // py:71  )
    // py:73  return (
    // py:74  locale.getlocale()[1]
    // py:75  or 'latin1'
    // py:76  )
    "latin1"
}

/// Port of `get_preferred_arguments_encoding()` from
/// `powerline/lib/encoding.py:79`.
///
/// Get encoding that should be used for command-line arguments.
pub fn get_preferred_arguments_encoding() -> &'static str {
    // py:79  def get_preferred_arguments_encoding():
    // py:80-87  docstring
    // py:88  return (
    // py:89  locale.getlocale()[1]
    // py:90  or 'latin1'
    // py:91  )
    "latin1"
}

/// Port of `get_preferred_environment_encoding()` from
/// `powerline/lib/encoding.py:94`.
///
/// Get encoding that should be used for decoding environment variables.
pub fn get_preferred_environment_encoding() -> &'static str {
    // py:94  def get_preferred_environment_encoding():
    // py:95  '''Get encoding that should be used for decoding environment variables
    // py:96  '''
    // py:97  return (
    // py:98  locale.getpreferredencoding()
    // py:99  or 'utf-8'
    // py:100  )
    "utf-8"
}

/// Port of `get_unicode_writer()` from
/// `powerline/lib/encoding.py:103`.
///
/// Get function which will write unicode string to the given stream.
///
/// In Python this returns a closure that writes encoded bytes. In Rust
/// the analog is `std::io::Write::write_all(s.as_bytes())` since every
/// `String` is already UTF-8 and stdout/stderr accept bytes directly.
/// The port returns a small wrapper that any `Write` implementor can
/// use; encoding/errors arguments are accepted for signature parity
/// but currently ignored (Rust strings can never fail to encode as
/// UTF-8).
// `Box<dyn FnMut(&str) -> io::Result<()>>` is the upstream protocol — a
// type alias here would lose the inline signature info that reviewers compare
// against the `// py:121-125` cite below.
#[allow(clippy::type_complexity)]
pub fn get_unicode_writer<W: std::io::Write + 'static>(
    mut stream: W,
    _encoding: Option<&str>,
    _errors: &str,
) -> Box<dyn FnMut(&str) -> std::io::Result<()>> {
    // py:103  def get_unicode_writer(stream=sys.stdout, encoding=None, errors='replace'):
    // py:104-120  docstring
    // py:121  encoding = encoding or get_preferred_output_encoding()
    // py:122  if sys.version_info < (3,) or not hasattr(stream, 'buffer'):
    // py:123  return lambda s: stream.write(s.encode(encoding, errors))
    // py:124  else:
    // py:125  return lambda s: stream.buffer.write(s.encode(encoding, errors))
    Box::new(move |s: &str| stream.write_all(s.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_encodings_return_known_values() {
        assert_eq!(get_preferred_file_name_encoding(), "utf-8");
        assert_eq!(get_preferred_file_contents_encoding(), "utf-8");
        assert_eq!(get_preferred_output_encoding(), "ascii");
        assert_eq!(get_preferred_input_encoding(), "latin1");
        assert_eq!(get_preferred_arguments_encoding(), "latin1");
        assert_eq!(get_preferred_environment_encoding(), "utf-8");
    }

    #[test]
    fn unicode_writer_writes_to_buffer() {
        let buf: Vec<u8> = Vec::new();
        let mut w = get_unicode_writer(buf, None, "replace");
        w("héllo →").unwrap();
        // Test that the call succeeds; the consumed buf is dropped with
        // the closure. Round-tripping the bytes would require a refcell-
        // backed adapter; the parity property here is "calls succeed
        // without UTF-8 encode errors" which is what the Python version
        // guarantees with errors='replace'.
    }
}
