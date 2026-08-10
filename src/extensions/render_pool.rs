// vim:fileencoding=utf-8:noet
//! Persistent worker pool for statusline renders. Sanctioned non-port
//! location per `docs/PORT.md` (which names "persistent worker pool"
//! as extensions-class functionality).
//!
//! Upstream's daemon renders inline in its `select` loop, so one slow
//! request delays every other client behind it. That is how a 30 s
//! Spotify segment turned into a dead statusline for every pane: the
//! loop spent its entire life inside one render while other clients sat
//! in `read()`.
//!
//! Here, the loop only ever does socket work. Renders run on worker
//! threads; completions come back over a channel, and a self-pipe byte
//! makes the loop's `poll` wake for them the same way it wakes for a
//! readable socket. The loop never calls a render function directly, so
//! no segment — however wedged — can stop it accepting, answering, or
//! reaping connections.
//!
//! Jobs carry a cancel flag. When the daemon gives up on a request
//! (client hung up, or the request blew its deadline) it cancels the
//! job; a worker that has not started it yet drops it instead of
//! spending a thread on output nobody will read.

use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

/// Worker count when `POWERLINERS_RENDER_THREADS` is unset. Renders are
/// dominated by subprocess and network waits rather than CPU, so a
/// small pool covers the concurrent-pane case without oversubscribing.
const DEFAULT_WORKERS: usize = 4;

/// Opaque handle tying a submitted job to its completion. Deliberately
/// not a file descriptor: descriptors are recycled the moment a
/// connection closes, and a late result must never be mistaken for the
/// answer to a new client that inherited the same number.
pub type JobId = u64;

struct Job {
    id: JobId,
    cancelled: Arc<AtomicBool>,
    work: Box<dyn FnOnce() -> Vec<u8> + Send + 'static>,
}

/// A finished render.
pub struct Done {
    pub id: JobId,
    pub bytes: Vec<u8>,
}

/// Pipe used purely to interrupt `poll`. Writing one byte makes the
/// read end readable, which is the only way to wake a loop that is
/// blocked on descriptors when the event we care about is a channel
/// send.
struct SelfPipe {
    read_fd: RawFd,
    write_fd: RawFd,
}

impl SelfPipe {
    fn new() -> std::io::Result<Self> {
        let mut fds = [0 as libc::c_int; 2];
        // SAFETY: `pipe` fills a two-element array of descriptors.
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        for fd in fds {
            // Non-blocking so draining never stalls the loop, and
            // close-on-exec so segments that fork don't inherit it.
            // SAFETY: both descriptors were just returned by `pipe`.
            unsafe {
                libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK);
                libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
            }
        }
        Ok(Self {
            read_fd: fds[0],
            write_fd: fds[1],
        })
    }

    /// Make the read end readable. A full pipe means wakeups are
    /// already pending, so a failed write is not an error.
    fn signal(write_fd: RawFd) {
        let byte = [1u8];
        // SAFETY: writing one byte from a live stack buffer to a
        // non-blocking descriptor we own.
        unsafe {
            libc::write(write_fd, byte.as_ptr().cast(), 1);
        }
    }

    /// Consume every pending wakeup byte.
    fn drain(&self) {
        let mut buf = [0u8; 256];
        loop {
            // SAFETY: reading into a live stack buffer from a
            // non-blocking descriptor we own.
            let n = unsafe { libc::read(self.read_fd, buf.as_mut_ptr().cast(), buf.len()) };
            if n <= 0 {
                break;
            }
        }
    }
}

impl Drop for SelfPipe {
    fn drop(&mut self) {
        // SAFETY: descriptors we own and never hand out.
        unsafe {
            libc::close(self.read_fd);
            libc::close(self.write_fd);
        }
    }
}

/// Fixed pool of render workers.
pub struct RenderPool {
    job_tx: Option<mpsc::Sender<Job>>,
    done_rx: mpsc::Receiver<Done>,
    wake: SelfPipe,
    next_id: AtomicU64,
    workers: Vec<JoinHandle<()>>,
}

impl RenderPool {
    /// Start a pool. Size comes from `POWERLINERS_RENDER_THREADS` when
    /// it parses as a positive integer, else [`DEFAULT_WORKERS`].
    pub fn new() -> std::io::Result<Self> {
        let size = std::env::var("POWERLINERS_RENDER_THREADS")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(DEFAULT_WORKERS);
        Self::with_size(size)
    }

    /// Start a pool with an explicit worker count.
    pub fn with_size(size: usize) -> std::io::Result<Self> {
        let wake = SelfPipe::new()?;
        let write_fd = wake.write_fd;

        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (done_tx, done_rx) = mpsc::channel::<Done>();
        // One receiver shared by every worker: whichever thread is free
        // takes the next job.
        let job_rx = Arc::new(Mutex::new(job_rx));

        let mut workers = Vec::with_capacity(size);
        for _ in 0..size.max(1) {
            let job_rx = job_rx.clone();
            let done_tx = done_tx.clone();
            workers.push(std::thread::spawn(move || loop {
                // Hold the queue lock only long enough to take a job,
                // never across the render itself.
                let job = {
                    let guard = match job_rx.lock() {
                        Ok(g) => g,
                        // A panicking worker poisons the lock; the
                        // remaining workers keep serving.
                        Err(e) => e.into_inner(),
                    };
                    guard.recv()
                };
                let Ok(job) = job else {
                    // Sender dropped: the pool is shutting down.
                    return;
                };
                if job.cancelled.load(Ordering::SeqCst) {
                    continue;
                }
                let bytes = (job.work)();
                if done_tx.send(Done { id: job.id, bytes }).is_err() {
                    return;
                }
                SelfPipe::signal(write_fd);
            }));
        }

        Ok(Self {
            job_tx: Some(job_tx),
            done_rx,
            wake,
            next_id: AtomicU64::new(1),
            workers,
        })
    }

    /// Descriptor to add to the event loop's poll set. Readable exactly
    /// when at least one render has finished.
    pub fn wake_fd(&self) -> RawFd {
        self.wake.read_fd
    }

    /// Queue `work`. Returns the job's id and its cancel flag; setting
    /// the flag stops a not-yet-started job from running at all.
    pub fn submit<F>(&self, work: F) -> Option<(JobId, Arc<AtomicBool>)>
    where
        F: FnOnce() -> Vec<u8> + Send + 'static,
    {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let cancelled = Arc::new(AtomicBool::new(false));
        let job = Job {
            id,
            cancelled: cancelled.clone(),
            work: Box::new(work),
        };
        self.job_tx.as_ref()?.send(job).ok()?;
        Some((id, cancelled))
    }

    /// Collect every completed render without blocking.
    pub fn drain(&self) -> Vec<Done> {
        // Drain the pipe *before* the channel. The reverse order loses
        // wakeups: a worker that sends between our `try_recv` and our
        // pipe read would have its byte consumed here while its result
        // stayed queued, and the loop would sleep on it until the poll
        // timeout expired.
        self.wake.drain();
        let mut out = Vec::new();
        while let Ok(done) = self.done_rx.try_recv() {
            out.push(done);
        }
        out
    }
}

impl Drop for RenderPool {
    fn drop(&mut self) {
        // Close the queue so idle workers observe a disconnect and
        // return, then join the ones that are free. A worker still
        // inside a wedged render is left detached on purpose: the whole
        // point of this module is that nothing waits on a hung segment,
        // and that includes shutdown.
        self.job_tx = None;
        for worker in std::mem::take(&mut self.workers) {
            if worker.is_finished() {
                let _ = worker.join();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_for<F: FnMut() -> bool>(mut cond: F, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        cond()
    }

    #[test]
    fn completed_job_comes_back_with_its_id() {
        let pool = RenderPool::with_size(2).expect("pool");
        let (id, _cancel) = pool.submit(|| b"rendered".to_vec()).expect("submit");

        let mut got = Vec::new();
        assert!(
            wait_for(
                || {
                    got.extend(pool.drain());
                    !got.is_empty()
                },
                Duration::from_secs(5)
            ),
            "job never completed"
        );
        assert_eq!(got[0].id, id);
        assert_eq!(got[0].bytes, b"rendered");
    }

    /// The regression that motivated the pool: a slow request must not
    /// delay an unrelated one.
    #[test]
    fn a_slow_job_does_not_block_a_fast_one() {
        let pool = RenderPool::with_size(2).expect("pool");
        pool.submit(|| {
            std::thread::sleep(Duration::from_secs(10));
            b"slow".to_vec()
        })
        .expect("submit slow");
        let (fast_id, _) = pool.submit(|| b"fast".to_vec()).expect("submit fast");

        let t0 = Instant::now();
        let mut seen = Vec::new();
        assert!(
            wait_for(
                || {
                    seen.extend(pool.drain());
                    seen.iter().any(|d| d.id == fast_id)
                },
                Duration::from_secs(5)
            ),
            "fast job was stuck behind the slow one"
        );
        assert!(t0.elapsed() < Duration::from_secs(5));
    }

    /// The wake descriptor is what lets the daemon's `poll` notice a
    /// completion at all; if it never becomes readable the loop sleeps
    /// until its timeout and the client waits for nothing.
    #[test]
    fn wake_fd_becomes_readable_on_completion() {
        let pool = RenderPool::with_size(1).expect("pool");
        pool.submit(|| b"x".to_vec()).expect("submit");

        let mut pfd = libc::pollfd {
            fd: pool.wake_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: single initialized pollfd, count 1.
        let rc = unsafe { libc::poll(&mut pfd, 1, 5000) };
        assert!(rc > 0, "poll did not wake for a finished render");
        assert!(pfd.revents & libc::POLLIN != 0);
    }

    #[test]
    fn cancelled_job_never_runs() {
        // One worker, occupied by a slow job, so the second job is still
        // queued when we cancel it.
        let pool = RenderPool::with_size(1).expect("pool");
        pool.submit(|| {
            std::thread::sleep(Duration::from_millis(300));
            b"blocker".to_vec()
        })
        .expect("submit blocker");

        let ran = Arc::new(AtomicBool::new(false));
        let ran_in_job = ran.clone();
        let (_, cancel) = pool
            .submit(move || {
                ran_in_job.store(true, Ordering::SeqCst);
                b"should not run".to_vec()
            })
            .expect("submit cancellable");
        cancel.store(true, Ordering::SeqCst);

        std::thread::sleep(Duration::from_millis(800));
        assert!(
            !ran.load(Ordering::SeqCst),
            "a cancelled job must be dropped, not executed"
        );
    }
}
