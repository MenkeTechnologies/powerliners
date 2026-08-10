// vim:fileencoding=utf-8:noet
//! "Is a process named X running?" answered in-process, with no
//! subprocess and no window-server round-trip. Sanctioned non-port
//! location per `docs/PORT.md`.
//!
//! Upstream powerline answers this question inside AppleScript:
//!
//! ```text
//! tell application "System Events"
//!     set process_list to (name of every process)
//! end tell
//! ```
//!
//! That is a Mach round-trip to a GUI helper application, and it is the
//! most fragile line in the whole segment set. When the daemon's login
//! session no longer matches the active console session, LaunchServices
//! cannot hand it a System Events instance and the call blocks for
//! ~30 s before failing — every render, forever, for a question the
//! kernel answers in microseconds.
//!
//! Backends:
//!   - macOS: `proc_listallpids` + `proc_name` (libproc).
//!   - Linux: `/proc/<pid>/comm`.
//!   - Elsewhere: `None` ("cannot determine"), so callers can decide
//!     whether to fall back rather than assume "not running".

/// Whether a process whose executable name equals `name` (ASCII
/// case-insensitive) is currently running.
///
/// `None` means the platform has no supported backend, which is
/// deliberately distinct from `Some(false)`.
pub fn is_running(name: &str) -> Option<bool> {
    #[cfg(target_os = "macos")]
    {
        macos_is_running(name)
    }
    #[cfg(target_os = "linux")]
    {
        linux_is_running(name)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = name;
        None
    }
}

#[cfg(target_os = "macos")]
fn macos_is_running(name: &str) -> Option<bool> {
    // First call sizes the table; the kernel returns the byte length of
    // the full pid list when handed a null buffer.
    // SAFETY: libproc's documented sizing call — null buffer with size 0
    // asks for the required length and writes nothing.
    let bytes = unsafe { libc::proc_listallpids(std::ptr::null_mut(), 0) };
    if bytes <= 0 {
        return None;
    }

    // Pad the count: processes can spawn between the sizing call and the
    // fetch, and a truncated table would silently drop candidates.
    let capacity = (bytes as usize / std::mem::size_of::<libc::c_int>()) + 64;
    let mut pids: Vec<libc::c_int> = vec![0; capacity];
    let buf_bytes = (capacity * std::mem::size_of::<libc::c_int>()) as libc::c_int;
    // SAFETY: `pids` owns `capacity` ints and `buf_bytes` describes
    // exactly that allocation, so the kernel cannot overrun it.
    let written = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), buf_bytes) };
    if written <= 0 {
        return None;
    }
    let count = (written as usize / std::mem::size_of::<libc::c_int>()).min(capacity);

    // `proc_name` writes the executable's last path component,
    // truncated to 2 * MAXCOMLEN + 1; 256 is comfortably above that.
    let mut buf = [0u8; 256];
    for &pid in &pids[..count] {
        if pid <= 0 {
            continue;
        }
        // SAFETY: `buf` is a live 256-byte allocation and we pass its
        // real length; proc_name NUL-terminates within that bound.
        let n = unsafe { libc::proc_name(pid, buf.as_mut_ptr().cast(), buf.len() as u32) };
        if n <= 0 {
            // Process exited between the listing and this call, or it
            // belongs to another user. Not an error for our purposes.
            continue;
        }
        let got = &buf[..(n as usize).min(buf.len())];
        if got.eq_ignore_ascii_case(name.as_bytes()) {
            return Some(true);
        }
    }
    Some(false)
}

#[cfg(target_os = "linux")]
fn linux_is_running(name: &str) -> Option<bool> {
    let entries = std::fs::read_dir("/proc").ok()?;
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        // Only numeric entries are pids; skip `self`, `net`, `sys`, …
        if !file_name
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let comm = entry.path().join("comm");
        // A pid can exit mid-scan; a failed read is not a failed lookup.
        if let Ok(got) = std::fs::read(&comm) {
            let got = got.strip_suffix(b"\n").unwrap_or(&got);
            if got.eq_ignore_ascii_case(name.as_bytes()) {
                return Some(true);
            }
        }
    }
    Some(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a process we control and require the lookup to see it.
    /// `sleep` is short enough that no platform truncates its name. The
    /// negative direction is deliberately not asserted here: other
    /// `sleep` processes may exist on the host at any moment.
    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn sees_a_live_child_and_not_a_dead_one() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep");

        assert_eq!(
            is_running("sleep"),
            Some(true),
            "a child we just spawned must appear in the process table"
        );

        child.kill().expect("kill sleep");
        child.wait().expect("reap sleep");
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn absent_process_is_negative_not_unknown() {
        assert_eq!(is_running("powerliners-no-such-process-xyzzy"), Some(false));
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn lookup_is_case_insensitive() {
        // Whatever answer `init`/`launchd` gives, upper and lower case
        // must agree — the comparison, not the process, is under test.
        assert_eq!(is_running("LAUNCHD"), is_running("launchd"));
    }
}
