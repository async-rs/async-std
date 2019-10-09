//! A thread pool for running blocking functions asynchronously.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;

use crate::task::task::{JoinHandle, Tag};
use crate::utils::abort_on_panic;

const MAX_THREADS: u64 = 10_000;

static DYNAMIC_THREAD_COUNT: AtomicU64 = AtomicU64::new(0);

struct Pool {
    sender: Sender<async_task::Task<Tag>>,
    receiver: Receiver<async_task::Task<Tag>>,
}

lazy_static! {
    static ref POOL: Pool = {
        for _ in 0..2 {
            thread::Builder::new()
                .name("async-blocking-driver".to_string())
                .spawn(|| abort_on_panic(|| {
                    for task in &POOL.receiver {
                        task.run();
                    }
                }))
                .expect("cannot start a thread driving blocking tasks");
        }

        // We want to use an unbuffered channel here to help
        // us drive our dynamic control. In effect, the
        // kernel's scheduler becomes the queue, reducing
        // the number of buffers that work must flow through
        // before being acted on by a core. This helps keep
        // latency snappy in the overall async system by
        // reducing bufferbloat.
        let (sender, receiver) = bounded(0);
        Pool { sender, receiver }
    };
}

// Create up to MAX_THREADS dynamic blocking task worker threads.
// Dynamic threads will terminate themselves if they don't
// receive any work after between one and ten seconds.
fn maybe_create_another_blocking_thread() {
    // We use a `Relaxed` atomic operation because
    // it's just a heuristic, and would not lose correctness
    // even if it's random.
    let workers = DYNAMIC_THREAD_COUNT.load(Ordering::Relaxed);
    if workers >= MAX_THREADS {
        return;
    }

    // We want to avoid having all threads terminate at
    // exactly the same time, causing thundering herd
    // effects. We want to stagger their destruction over
    // 10 seconds or so to make the costs fade into
    // background noise.
    //
    // Generate a simple random number of milliseconds
    let rand_sleep_ms = u64::from(random(10_000));

    thread::Builder::new()
        .name("async-blocking-driver-dynamic".to_string())
        .spawn(move || {
            let wait_limit = Duration::from_millis(1000 + rand_sleep_ms);

            DYNAMIC_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
            while let Ok(task) = POOL.receiver.recv_timeout(wait_limit) {
                abort_on_panic(|| task.run());
            }
            DYNAMIC_THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
        })
        .expect("cannot start a dynamic thread driving blocking tasks");
}

// Enqueues work, attempting to send to the threadpool in a
// nonblocking way and spinning up another worker thread if
// there is not a thread ready to accept the work.
fn schedule(t: async_task::Task<Tag>) {
    if let Err(err) = POOL.sender.try_send(t) {
        // We were not able to send to the channel without
        // blocking. Try to spin up another thread and then
        // retry sending while blocking.
        maybe_create_another_blocking_thread();
        POOL.sender.send(err.into_inner()).unwrap();
    }
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks.
pub(crate) fn spawn<F, R>(f: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let tag = Tag::new(None);
    let future = async move { f() };
    let (task, handle) = async_task::spawn(future, schedule, tag);
    task.schedule();
    JoinHandle::new(handle)
}

/// Generates a random number in `0..n`.
fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = Cell::new(Wrapping(1_406_868_647));
    }

    RNG.with(|rng| {
        // This is the 32-bit variant of Xorshift.
        //
        // Source: https://en.wikipedia.org/wiki/Xorshift
        let mut x = rng.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rng.set(x);

        // This is a fast alternative to `x % n`.
        //
        // Author: Daniel Lemire
        // Source: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((u64::from(x.0)).wrapping_mul(u64::from(n)) >> 32) as u32
    })
}
