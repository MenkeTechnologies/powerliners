// vim:fileencoding=utf-8:noet
//! Per-segment deadline enforcement. Sanctioned non-port location per
//! `docs/PORT.md` — upstream powerline runs every segment inline with
//! no time budget, so a single wedged segment stalls the whole
//! statusline.
//!
//! The guarantee: [`run`] returns within roughly `timeout`, whatever
//! the segment does. A segment that overruns keeps running on its own
//! thread (Rust cannot safely interrupt arbitrary code mid-flight), but
//! it no longer holds the render, and three things keep the fallout
//! bounded:
//!
//! 1. **In-flight dedupe.** While a segment id is overdue, further
//!    calls for that id return immediately instead of stacking up a new
//!    thread per render. A statusline refreshing every 2 s against a
//!    30 s hang would otherwise accumulate 15 live threads.
//! 2. **Last-good cache.** An overrun serves the previous value for up
//!    to `MAX_STALENESS`, so a briefly-slow segment holds its place
//!    in the bar instead of blinking out and shoving every segment
//!    beside it sideways.
//! 3. **Subprocess kills.** Segments that shell out should use
//!    `extensions::proc_timeout`, which SIGKILLs the child. The
//!    watchdog frees the *render*; `proc_timeout` frees the *process*.
//!    They are complementary, not alternatives.

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::Value;

/// Default budget for one segment. Chosen against a 2 s tmux
/// `status-interval`: segments run concurrently, so this is the
/// worst-case contribution of any single one, and it clears the
/// slowest healthy segments observed in practice (network_load ~510 ms,
/// cpu_load_percent ~350 ms, external_ip ~110 ms) with room to spare.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);

/// How long a cached value may be served after the segment that
/// produced it stops responding. Past this, the segment renders empty —
/// a stale-but-plausible reading is worse than an absent one.
const MAX_STALENESS: Duration = Duration::from_secs(60);

struct Entry {
    /// Last value the segment returned, and when.
    last_good: Option<(Instant, Option<Value>)>,
    /// Set while a call for this id has overrun and its thread is still
    /// unaccounted for.
    overdue: bool,
}

fn table() -> &'static Mutex<HashMap<String, Entry>> {
    static T: OnceLock<Mutex<HashMap<String, Entry>>> = OnceLock::new();
    T.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Outcome of a watchdogged call, for the caller's diagnostics.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// The segment finished inside its budget.
    Completed,
    /// The segment overran; the value (if any) came from the cache.
    TimedOut,
    /// A previous call for this id is still overdue, so this one never
    /// ran; the value (if any) came from the cache.
    Skipped,
    /// The segment panicked, or its thread could not be started. The
    /// value (if any) came from the cache.
    ///
    /// Distinct from [`Outcome::TimedOut`] because the two need
    /// opposite bookkeeping: a timeout leaves a thread still running,
    /// which is what `overdue` exists to deduplicate against, while a
    /// panic leaves nothing behind. Marking a panic overdue was a
    /// one-way door — `mark_done` runs at the end of the segment
    /// thread, so a thread that panicked never cleared the flag and
    /// that segment returned `Skipped` for the rest of the daemon's
    /// life.
    Failed,
}

/// Run `f` under a deadline, keyed by segment `id`.
///
/// `f` must be `Send + 'static` because it may outlive this call.
pub fn run<F>(id: &str, timeout: Duration, f: F) -> (Option<Value>, Outcome)
where
    F: FnOnce() -> Option<Value> + Send + 'static,
{
    if is_overdue(id) {
        return (cached(id), Outcome::Skipped);
    }

    // `Err` on the wire means the segment panicked. It travels as a
    // value rather than as a dropped sender so the caller can tell a
    // panic apart from a segment that is merely slow.
    let (tx, rx) = mpsc::channel::<Result<Option<Value>, String>>();
    // Detached by design: if the segment overruns we abandon the handle
    // rather than join it. `mark_done` below runs on the segment's own
    // thread, so the dedupe flag clears whenever it eventually finishes.
    let id_for_thread = id.to_string();
    // `Builder::spawn` over `thread::spawn`: the latter panics when the
    // OS refuses a thread, and that panic would unwind into the render
    // that called us rather than being contained to this segment.
    let spawned = std::thread::Builder::new()
        .name(format!("pl-seg-{}", id))
        .spawn(move || {
            // A panicking segment must still clear its own bookkeeping.
            let value = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
                Ok(value) => value,
                Err(_) => {
                    // The process-wide panic hook has already logged the
                    // message and location; all this needs to do is stop
                    // the panic here and report it upward.
                    let _ = tx.send(Err(format!("segment {} panicked", id_for_thread)));
                    // Only the flag: the cached value from the last good
                    // render has to survive, or the segment drops out of
                    // the bar instead of holding its place.
                    clear_overdue(&id_for_thread);
                    return;
                }
            };
            // The receiver is gone if we already timed out; that is the
            // normal abandoned-work path, not an error.
            let _ = tx.send(Ok(value.clone()));
            mark_done(&id_for_thread, value);
        });

    if let Err(e) = spawned {
        crate::extensions::diag_log::log(&format!(
            "segment {} could not be started: {} — serving the cached value",
            id, e
        ));
        return (cached(id), Outcome::Failed);
    }

    match rx.recv_timeout(timeout) {
        Ok(Ok(value)) => (value, Outcome::Completed),
        // The segment panicked. There is no live thread to deduplicate
        // against, so this id stays eligible for the next render.
        Ok(Err(message)) => {
            crate::extensions::diag_log::log(&format!(
                "segment watchdog: {} — serving the cached value",
                message
            ));
            (cached(id), Outcome::Failed)
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            mark_overdue(id);
            (cached(id), Outcome::TimedOut)
        }
        // The thread went away without sending anything at all. Like a
        // panic, it leaves nothing running, so it must not be marked
        // overdue either.
        Err(mpsc::RecvTimeoutError::Disconnected) => (cached(id), Outcome::Failed),
    }
}

fn is_overdue(id: &str) -> bool {
    table()
        .lock()
        .map(|t| t.get(id).map(|e| e.overdue).unwrap_or(false))
        .unwrap_or(false)
}

fn mark_overdue(id: &str) {
    if let Ok(mut t) = table().lock() {
        t.entry(id.to_string())
            .or_insert(Entry {
                last_good: None,
                overdue: false,
            })
            .overdue = true;
    }
}

/// Make `id` eligible again without touching its cached value.
///
/// The panic path needs exactly this and not [`mark_done`]: a segment
/// that panicked produced no value, and recording `None` as its
/// last-good reading would throw away the perfectly good one from the
/// render before it — the segment would then vanish from the bar rather
/// than hold its place, which is the thing the cache exists to prevent.
fn clear_overdue(id: &str) {
    if let Ok(mut t) = table().lock() {
        if let Some(entry) = t.get_mut(id) {
            entry.overdue = false;
        }
    }
}

/// Record a completed call: clears the dedupe flag and refreshes the
/// cache. Called from the segment's thread, including when that thread
/// finishes long after the render gave up on it.
fn mark_done(id: &str, value: Option<Value>) {
    if let Ok(mut t) = table().lock() {
        let entry = t.entry(id.to_string()).or_insert(Entry {
            last_good: None,
            overdue: false,
        });
        entry.overdue = false;
        entry.last_good = Some((Instant::now(), value));
    }
}

/// Last value for `id`, if one was recorded inside [`MAX_STALENESS`].
fn cached(id: &str) -> Option<Value> {
    let t = table().lock().ok()?;
    let (at, value) = t.get(id)?.last_good.as_ref()?;
    if at.elapsed() > MAX_STALENESS {
        return None;
    }
    value.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids are process-global; keep tests from colliding with each other.
    fn id(tag: &str) -> String {
        format!("test.{}.{}", tag, std::process::id())
    }

    #[test]
    fn fast_segment_returns_its_own_value() {
        let (v, outcome) = run(&id("fast"), Duration::from_secs(5), || {
            Some(Value::from("ok"))
        });
        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(v, Some(Value::from("ok")));
    }

    /// The core guarantee: the caller is released on the deadline, not
    /// when the segment finishes.
    #[test]
    fn slow_segment_releases_the_caller_at_the_deadline() {
        let t0 = Instant::now();
        let (_, outcome) = run(&id("slow"), Duration::from_millis(100), || {
            std::thread::sleep(Duration::from_secs(10));
            Some(Value::from("too late"))
        });
        let dt = t0.elapsed();
        assert_eq!(outcome, Outcome::TimedOut);
        assert!(
            dt < Duration::from_secs(5),
            "render blocked {:?}; the watchdog did not release it",
            dt
        );
    }

    /// A segment that overran once must not be re-entered on the next
    /// render — that is what turns one hang into a thread leak.
    #[test]
    fn overdue_segment_is_not_re_entered() {
        let key = id("dedupe");
        let (_, first) = run(&key, Duration::from_millis(100), || {
            std::thread::sleep(Duration::from_secs(10));
            None
        });
        assert_eq!(first, Outcome::TimedOut);

        let t0 = Instant::now();
        let (_, second) = run(&key, Duration::from_secs(30), || {
            panic!("must not run while the previous call is overdue")
        });
        assert_eq!(second, Outcome::Skipped);
        assert!(
            t0.elapsed() < Duration::from_secs(1),
            "skip path should be immediate"
        );
    }

    /// A timeout serves the previous good value so the segment keeps its
    /// place in the bar instead of vanishing.
    #[test]
    fn timeout_serves_the_last_good_value() {
        let key = id("cache");
        let (v, outcome) = run(&key, Duration::from_secs(5), || Some(Value::from("warm")));
        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(v, Some(Value::from("warm")));

        let (v, outcome) = run(&key, Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_secs(10));
            Some(Value::from("never"))
        });
        assert_eq!(outcome, Outcome::TimedOut);
        assert_eq!(v, Some(Value::from("warm")), "cached value should survive");
    }

    /// Once the overdue call finally lands, the id must become usable
    /// again — otherwise a single slow render disables the segment for
    /// the daemon's whole lifetime.
    #[test]
    fn segment_recovers_after_the_overdue_call_finishes() {
        let key = id("recover");
        let (_, outcome) = run(&key, Duration::from_millis(50), || {
            std::thread::sleep(Duration::from_millis(300));
            Some(Value::from("late"))
        });
        assert_eq!(outcome, Outcome::TimedOut);

        // Wait past the slow closure's own duration so its thread has
        // run `mark_done`.
        let deadline = Instant::now() + Duration::from_secs(5);
        while is_overdue(&key) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }

        let (v, outcome) = run(&key, Duration::from_secs(5), || Some(Value::from("fresh")));
        assert_eq!(outcome, Outcome::Completed);
        assert_eq!(v, Some(Value::from("fresh")));
    }

    /// A panicking segment must not take the caller down with it, and
    /// must be reported as `Failed` rather than dressed up as a timeout.
    #[test]
    fn panicking_segment_is_contained_and_reported() {
        let key = id("panics");
        let (v, outcome) = run(&key, Duration::from_secs(5), || panic!("segment exploded"));
        assert_eq!(outcome, Outcome::Failed);
        assert_eq!(v, None, "no cached value had ever been recorded");
    }

    /// The bug this variant exists for: a panic used to be classed as a
    /// timeout, which marked the id overdue, and `mark_done` never ran
    /// on the dead thread — so the segment was skipped forever after.
    #[test]
    fn segment_still_runs_after_it_panicked_once() {
        let key = id("panic-recover");

        let (_, warm) = run(&key, Duration::from_secs(5), || Some(Value::from("warm")));
        assert_eq!(warm, Outcome::Completed);

        let (v, outcome) = run(&key, Duration::from_secs(5), || panic!("boom"));
        assert_eq!(outcome, Outcome::Failed);
        assert_eq!(
            v,
            Some(Value::from("warm")),
            "the last good value should still be served"
        );
        assert!(
            !is_overdue(&key),
            "a panic leaves no thread running, so the id must stay eligible"
        );

        let (v, outcome) = run(&key, Duration::from_secs(5), || Some(Value::from("fresh")));
        assert_eq!(
            outcome,
            Outcome::Completed,
            "the segment must recover on the very next render"
        );
        assert_eq!(v, Some(Value::from("fresh")));
    }
}
