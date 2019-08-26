//! A thread pool for running blocking functions asynchronously.

use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;

use crate::future::Future;
use crate::task::{Context, Poll};
use crate::utils::abort_on_panic;

const LOW_WATERMARK: u64 = 2;
const MAX_THREADS: u64 = 10_000;

// Pool task frequency calculation variables
static AVR_FREQUENCY: AtomicU64 = AtomicU64::new(0);
static FREQUENCY: AtomicU64 = AtomicU64::new(0);

// Pool speedup calculation variables
static SPEEDUP: AtomicU64 = AtomicU64::new(0);

// Pool size variables
static EXPECTED_POOL_SIZE: AtomicU64 = AtomicU64::new(LOW_WATERMARK);
static CURRENT_POOL_SIZE: AtomicU64 = AtomicU64::new(LOW_WATERMARK);

struct Pool {
    sender: Sender<async_task::Task<()>>,
    receiver: Receiver<async_task::Task<()>>,
}

lazy_static! {
    static ref POOL: Pool = {
        for _ in 0..LOW_WATERMARK {
            thread::Builder::new()
                .name("async-blocking-driver".to_string())
                .spawn(|| abort_on_panic(|| {
                    for task in &POOL.receiver {
                        task.run();
                        calculate_dispatch_frequency();
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

fn calculate_dispatch_frequency() {
    // Calculate current message processing rate here
    let previous_freq = FREQUENCY.fetch_sub(1, Ordering::Relaxed);
    let avr_freq = AVR_FREQUENCY.load(Ordering::Relaxed);
    let current_pool_size = CURRENT_POOL_SIZE.load(Ordering::Relaxed);
    let frequency = (avr_freq as f64 + previous_freq as f64 / current_pool_size as f64) as u64;
    AVR_FREQUENCY.store(frequency, Ordering::Relaxed);

    // Adapt the thread count of pool
    let speedup = SPEEDUP.load(Ordering::Relaxed);
    if frequency > speedup {
        // Speedup can be gained. Scale the pool up here.
        SPEEDUP.store(frequency, Ordering::Relaxed);
        EXPECTED_POOL_SIZE.store(current_pool_size + 1, Ordering::Relaxed);
    } else {
        // There is no need for the extra threads, schedule them to be closed.
        let expected = EXPECTED_POOL_SIZE.load(Ordering::Relaxed);
        if 1 + LOW_WATERMARK < expected {
            // Substract amount of low watermark
            EXPECTED_POOL_SIZE.fetch_sub(LOW_WATERMARK, Ordering::Relaxed);
        }
    }
}

// Creates yet another thread to receive tasks.
// Dynamic threads will terminate themselves if they don't
// receive any work after between one and ten seconds.
fn create_blocking_thread() {
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

            CURRENT_POOL_SIZE.fetch_add(1, Ordering::Relaxed);
            while let Ok(task) = POOL.receiver.recv_timeout(wait_limit) {
                abort_on_panic(|| task.run());
                calculate_dispatch_frequency();
            }
            CURRENT_POOL_SIZE.fetch_sub(1, Ordering::Relaxed);
        })
        .expect("cannot start a dynamic thread driving blocking tasks");
}

// Enqueues work, attempting to send to the threadpool in a
// nonblocking way and spinning up needed amount of threads
// based on the previous statistics without relying on
// if there is not a thread ready to accept the work or not.
fn schedule(t: async_task::Task<()>) {
    // Add up for every incoming task schedule
    FREQUENCY.fetch_add(1, Ordering::Relaxed);

    // Calculate the amount of threads needed to spin up
    // then retry sending while blocking. It doesn't spin if
    // expected pool size is above the MAX_THREADS (which is a
    // case won't happen)
    let pool_size = EXPECTED_POOL_SIZE.load(Ordering::Relaxed);
    let current_pool_size = CURRENT_POOL_SIZE.load(Ordering::Relaxed);
    let reward = (AVR_FREQUENCY.load(Ordering::Relaxed) as f64 / 2.0_f64) as u64;

    if pool_size > current_pool_size && pool_size <= MAX_THREADS {
        let needed = pool_size.saturating_sub(current_pool_size);

        // For safety, check boundaries before spawning threads.
        // This also won't be expected to happen. But better safe than sorry.
        if needed > 0 && (needed < pool_size || needed < current_pool_size) {
            (0..needed).for_each(|_| {
                create_blocking_thread();
            });
        }
    }

    if let Err(err) = POOL.sender.try_send(t) {
        // We were not able to send to the channel without
        // blocking.
        POOL.sender.send(err.into_inner()).unwrap();
    } else {
        // Every successful dispatch, rewarded with negative
        if reward + LOW_WATERMARK < pool_size {
            EXPECTED_POOL_SIZE.fetch_sub(reward, Ordering::Relaxed);
        }
    }
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks.
pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let (task, handle) = async_task::spawn(future, schedule, ());
    task.schedule();
    JoinHandle(handle)
}

/// A handle to a blocking task.
pub struct JoinHandle<R>(async_task::JoinHandle<R, ()>);

impl<R> Unpin for JoinHandle<R> {}

impl<R> Future for JoinHandle<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|out| out.unwrap())
    }
}

impl<R> fmt::Debug for JoinHandle<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle")
            .field("handle", &self.0)
            .finish()
    }
}

/// Generates a random number in `0..n`.
fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = Cell::new(Wrapping(1406868647));
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
        ((x.0 as u64).wrapping_mul(n as u64) >> 32) as u32
    })
}
