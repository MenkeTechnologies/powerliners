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
/// `status-interval`: segments run concurrently via [`run_all`], so
/// this is the worst-case contribution of any single one, and it clears
/// the slowest healthy segments observed in practice (network_load
/// ~520 ms, external_ip ~120 ms, gpu ~47 ms) with room to spare.
///
/// The concurrency this budget assumes is real as of [`run_all`]. It
/// was not before: the renderer called [`run`] once per segment and
/// blocked on each, so a statusline cost the *sum* of its segments
/// (~900 ms measured) rather than the slowest one (~520 ms), and this
/// comment described an arrangement the code did not implement.
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
///
/// Exactly [`start`] followed by [`finish`]. A caller with more than
/// one segment to run wants [`run_all`], which starts them all before
/// waiting on any.
pub fn run<F>(id: &str, timeout: Duration, f: F) -> (Option<Value>, Outcome)
where
    F: FnOnce() -> Option<Value> + Send + 'static,
{
    finish(start(id, timeout, f))
}

/// A segment that is running and has not been waited on yet.
pub struct Started {
    /// When this segment was started, for reporting how long a
    /// segment that never produced a value was outstanding.
    started_at: Instant,
    id: String,
    /// Absolute, fixed when the segment started rather than when its
    /// turn to be collected comes up. Collecting a batch in order must
    /// not charge a segment for the time spent waiting on the ones
    /// ahead of it — they were all running that whole time.
    deadline: Instant,
    state: StartedState,
}

enum StartedState {
    /// Thread running; the value and its body time will arrive here.
    Running(mpsc::Receiver<Result<(Option<Value>, Duration), String>>),
    /// Nothing was started, the answer is already known.
    Settled(Outcome),
}

/// Start `f` on its own thread and return without waiting for it.
///
/// Split out of [`run`] so an entire statusline's segments can be in
/// flight simultaneously. Every `start` must be paired with [`finish`].
pub fn start<F>(id: &str, timeout: Duration, f: F) -> Started
where
    F: FnOnce() -> Option<Value> + Send + 'static,
{
    let started_at = Instant::now();
    let deadline = started_at + timeout;

    if is_overdue(id) {
        return Started {
            id: id.to_string(),
            started_at,
            deadline,
            state: StartedState::Settled(Outcome::Skipped),
        };
    }

    // `Err` on the wire means the segment panicked. It travels as a
    // value rather than as a dropped sender so the caller can tell a
    // panic apart from a segment that is merely slow.
    //
    // The `Duration` is how long the segment body itself took, timed on
    // its own thread. In a batch that is the only honest per-segment
    // number: measuring at the collection site would charge every
    // segment for the ones collected before it, so a 3 ms clock segment
    // would report the 500 ms that the network segment ahead of it
    // spent — and this log is what gets read to find a slow segment.
    #[allow(clippy::type_complexity)]
    let (tx, rx) = mpsc::channel::<Result<(Option<Value>, Duration), String>>();
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
            let body_t0 = Instant::now();
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
            let _ = tx.send(Ok((value.clone(), body_t0.elapsed())));
            mark_done(&id_for_thread, value);
        });

    if let Err(e) = spawned {
        crate::extensions::diag_log::log(&format!(
            "segment {} could not be started: {} — serving the cached value",
            id, e
        ));
        return Started {
            id: id.to_string(),
            started_at,
            deadline,
            state: StartedState::Settled(Outcome::Failed),
        };
    }

    Started {
        id: id.to_string(),
        started_at,
        deadline,
        state: StartedState::Running(rx),
    }
}

/// Wait for a [`start`]ed segment, up to the deadline fixed when it
/// began. Discards the timing; see [`finish_timed`].
pub fn finish(started: Started) -> (Option<Value>, Outcome) {
    let (value, outcome, _) = finish_timed(started);
    (value, outcome)
}

/// [`finish`], also reporting how long the segment body ran.
///
/// For anything that did not produce a value — skipped, timed out,
/// failed — the duration is the wall time this segment was outstanding,
/// since there is no body time to report.
pub fn finish_timed(started: Started) -> (Option<Value>, Outcome, Duration) {
    let Started {
        id,
        deadline,
        started_at,
        state,
    } = started;

    let rx = match state {
        StartedState::Running(rx) => rx,
        StartedState::Settled(outcome) => return (cached(&id), outcome, started_at.elapsed()),
    };

    // Already past the deadline is a zero-length wait, not a negative
    // one: `saturating_duration_since` keeps that from wrapping into a
    // near-eternal timeout.
    let remaining = deadline.saturating_duration_since(Instant::now());

    match rx.recv_timeout(remaining) {
        Ok(Ok((value, body))) => (value, Outcome::Completed, body),
        // The segment panicked. There is no live thread to deduplicate
        // against, so this id stays eligible for the next render.
        Ok(Err(message)) => {
            crate::extensions::diag_log::log(&format!(
                "segment watchdog: {} — serving the cached value",
                message
            ));
            (cached(&id), Outcome::Failed, started_at.elapsed())
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            mark_overdue(&id);
            (cached(&id), Outcome::TimedOut, started_at.elapsed())
        }
        // The thread went away without sending anything at all. Like a
        // panic, it leaves nothing running, so it must not be marked
        // overdue either.
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            (cached(&id), Outcome::Failed, started_at.elapsed())
        }
    }
}

/// Run every segment in `jobs` concurrently, returning their results in
/// the order given.
///
/// The reason this exists: [`run`] blocks until its one segment lands,
/// so a renderer calling it per segment pays the *sum* of every
/// segment's latency. A real statusline is ~20 segments that are almost
/// entirely subprocess and network waits — `netstat`, `top`, `git`,
/// `ioreg`, an HTTP call for the weather — and serialising those made a
/// render take longer than the `status-interval` that asked for it, so
/// tmux abandoned most requests before they finished.
///
/// Starting them all first makes the batch cost roughly the slowest
/// segment instead of the total, and each still answers to its own
/// deadline because [`Started`] fixes that at start time.
pub fn run_all(jobs: Vec<(String, Duration, Job)>) -> Vec<(Option<Value>, Outcome, Duration)> {
    // Two passes, deliberately: every thread is running before the
    // first `finish_timed` blocks. Fusing these into one loop would
    // serialise them again and silently undo the whole point. Collect
    // the `Vec` — a lazy iterator would interleave start and finish and
    // do exactly that.
    let started: Vec<Started> = jobs
        .into_iter()
        .map(|(id, timeout, f)| start(&id, timeout, f))
        .collect();

    started.into_iter().map(finish_timed).collect()
}

/// A segment body, boxed so a batch can hold bodies of differing types.
pub type Job = Box<dyn FnOnce() -> Option<Value> + Send + 'static>;

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

    /// The point of the batch API. Three segments that each sleep
    /// 300 ms must cost ~300 ms together, not 900 ms — a serial
    /// implementation passes every other assertion here, so the wall
    /// clock is the only one that catches a regression back to it.
    #[test]
    fn run_all_overlaps_the_segments_instead_of_summing_them() {
        let jobs: Vec<(String, Duration, Job)> = (0..3)
            .map(|i| {
                let job: Job = Box::new(move || {
                    std::thread::sleep(Duration::from_millis(300));
                    Some(Value::from(i))
                });
                (id(&format!("batch-{}", i)), Duration::from_secs(5), job)
            })
            .collect();

        let t0 = Instant::now();
        let out = run_all(jobs);
        let wall = t0.elapsed();

        assert_eq!(out.len(), 3);
        for (i, (value, outcome, _)) in out.iter().enumerate() {
            assert_eq!(*outcome, Outcome::Completed);
            assert_eq!(*value, Some(Value::from(i as i64)), "results stay in order");
        }
        assert!(
            wall < Duration::from_millis(750),
            "3x300ms took {:?} — that is the serial sum, so the batch is not overlapping",
            wall
        );
    }

    /// Each segment answers to its own deadline, measured from when it
    /// started. Collecting in order must not let a slow segment early in
    /// the batch eat the budget of the ones behind it.
    #[test]
    fn a_slow_segment_does_not_consume_a_later_segments_budget() {
        let slow: Job = Box::new(|| {
            std::thread::sleep(Duration::from_secs(30));
            Some(Value::from("never"))
        });
        let quick: Job = Box::new(|| Some(Value::from("in time")));

        let out = run_all(vec![
            (id("budget-slow"), Duration::from_millis(200), slow),
            // Would already be past a deadline measured from collection
            // time, but its own budget started with the batch.
            (id("budget-quick"), Duration::from_millis(400), quick),
        ]);

        assert_eq!(out[0].1, Outcome::TimedOut);
        assert_eq!(
            out[1].1,
            Outcome::Completed,
            "the second segment was charged for the first one's wait"
        );
        assert_eq!(out[1].0, Some(Value::from("in time")));
    }

    /// The per-segment duration must be the segment's own body time, not
    /// the batch wall — it is what the diagnostic log reports when
    /// someone asks which segment is slow.
    #[test]
    fn reported_duration_is_the_segments_own_time() {
        let slow: Job = Box::new(|| {
            std::thread::sleep(Duration::from_millis(400));
            Some(Value::from("slow"))
        });
        let fast: Job = Box::new(|| Some(Value::from("fast")));

        let out = run_all(vec![
            (id("timing-slow"), Duration::from_secs(5), slow),
            (id("timing-fast"), Duration::from_secs(5), fast),
        ]);

        assert!(
            out[0].2 >= Duration::from_millis(300),
            "slow: {:?}",
            out[0].2
        );
        assert!(
            out[1].2 < Duration::from_millis(200),
            "fast segment reported {:?} — it was charged for the slow one",
            out[1].2
        );
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
