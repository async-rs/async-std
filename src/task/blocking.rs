//! A thread pool for running blocking functions asynchronously.
//!
//! Blocking thread pool consists of four elements:
//! * Frequency Detector
//! * Trend Estimator
//! * Predictive Upscaler
//! * Time-based Downscaler
//!
//! ## Frequency Detector
//! Detects how many tasks are submitted from scheduler to thread pool in a given time frame.
//! Pool manager thread does this sampling every 200 milliseconds.
//! This value is going to be used for trend estimation phase.
//!
//! ## Trend Estimator
//! Hold up to the given number of frequencies to create an estimation.
//! This pool holds 10 frequencies at a time.
//! Estimation algorithm and prediction uses Exponentially Weighted Moving Average algorithm.
//!
//! Algorithm is altered and adapted from [A Novel Predictive and Self–Adaptive Dynamic Thread Pool Management](https://doi.org/10.1109/ISPA.2011.61).
//!
//! ## Predictive Upscaler
//! Selects upscaling amount based on estimation or when throughput hogs based on amount of tasks mapped.
//! Throughput hogs determined by a combination of job in / job out frequency and current scheduler task assignment frequency.
//!
//! ## Time-based Downscaler
//! After dynamic tasks spawned with upscaler they will continue working in between 1 second and 10 seconds.
//! When tasks are detached from the channels after this amount they join back.

use std::collections::VecDeque;
use std::fmt;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;

use crate::future::Future;
use crate::io::ErrorKind;
use crate::task::{Context, Poll};
use crate::utils::abort_on_panic;
use std::sync::{Arc, Mutex};

/// Low watermark value, defines the bare minimum of the pool.
/// Spawns initial thread set.
const LOW_WATERMARK: u64 = 2;

/// Pool managers interval time (milliseconds).
/// This is the actual interval which makes adaptation calculation.
const MANAGER_POLL_INTERVAL: u64 = 200;

/// Frequency histogram's sliding window size.
/// Defines how many frequencies will be considered for adaptation.
const FREQUENCY_QUEUE_SIZE: usize = 10;

/// Exponential moving average smoothing coefficient for limited window.
/// Smoothing factor is estimated with: 2 / (N + 1) where N is sample size.
const EMA_COEFFICIENT: f64 = 2_f64 / (FREQUENCY_QUEUE_SIZE as f64 + 1_f64);

/// Pool task frequency variable.
/// Holds scheduled tasks onto the thread pool for the calculation window.
static FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Possible max threads (without OS contract).
static MAX_THREADS: AtomicU64 = AtomicU64::new(10_000);

/// Pool interface between the scheduler and thread pool
struct Pool {
    sender: Sender<async_task::Task<()>>,
    receiver: Receiver<async_task::Task<()>>,
}

lazy_static! {
    /// Blocking pool with static starting thread count.
    static ref POOL: Pool = {
        for _ in 0..LOW_WATERMARK {
            thread::Builder::new()
                .name("async-blocking-driver".to_string())
                .spawn(|| abort_on_panic(|| {
                    for task in &POOL.receiver {
                        task.run();
                    }
                }))
                .expect("cannot start a thread driving blocking tasks");
        }

        // Pool manager to check frequency of task rates
        // and take action by scaling the pool accordingly.
        thread::Builder::new()
            .name("async-pool-manager".to_string())
            .spawn(|| abort_on_panic(|| {
                let poll_interval = Duration::from_millis(MANAGER_POLL_INTERVAL);
                loop {
                    scale_pool();
                    thread::sleep(poll_interval);
                }
            }))
            .expect("thread pool manager cannot be started");

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

    /// Sliding window for pool task frequency calculation
    static ref FREQ_QUEUE: Arc<Mutex<VecDeque<u64>>> = {
        Arc::new(Mutex::new(VecDeque::with_capacity(FREQUENCY_QUEUE_SIZE + 1)))
    };

    /// Dynamic pool thread count variable
    static ref POOL_SIZE: Arc<Mutex<u64>> = Arc::new(Mutex::new(LOW_WATERMARK));
}

/// Exponentially Weighted Moving Average calculation
///
/// This allows us to find the EMA value.
/// This value represents the trend of tasks mapped onto the thread pool.
/// Calculation is following:
/// ```
/// +--------+-----------------+----------------------------------+
/// | Symbol |   Identifier    |           Explanation            |
/// +--------+-----------------+----------------------------------+
/// | α      | EMA_COEFFICIENT | smoothing factor between 0 and 1 |
/// | Yt     | freq            | frequency sample at time t       |
/// | St     | acc             | EMA at time t                    |
/// +--------+-----------------+----------------------------------+
/// ```
/// Under these definitions formula is following:
/// ```
/// EMA = α * [ Yt + (1 - α)*Yt-1 + ((1 - α)^2)*Yt-2 + ((1 - α)^3)*Yt-3 ... ] + St
/// ```
/// # Arguments
///
/// * `freq_queue` - Sliding window of frequency samples
#[inline]
fn calculate_ema(freq_queue: &VecDeque<u64>) -> f64 {
    freq_queue.iter().enumerate().fold(0_f64, |acc, (i, freq)| {
        acc + ((*freq as f64) * ((1_f64 - EMA_COEFFICIENT).powf(i as f64) as f64))
    }) * EMA_COEFFICIENT as f64
}

/// Adaptive pool scaling function
///
/// This allows to spawn new threads to make room for incoming task pressure.
/// Works in the background detached from the pool system and scales up the pool based
/// on the request rate.
///
/// It uses frequency based calculation to define work. Utilizing average processing rate.
fn scale_pool() {
    // Fetch current frequency, it does matter that operations are ordered in this approach.
    let current_frequency = FREQUENCY.load(Ordering::SeqCst);
    let freq_queue_arc = FREQ_QUEUE.clone();
    let mut freq_queue = freq_queue_arc.lock().unwrap();

    // Make it safe to start for calculations by adding initial frequency scale
    if freq_queue.len() == 0 {
        freq_queue.push_back(0);
    }

    // Calculate message rate for the given time window
    let frequency = (current_frequency as f64 / MANAGER_POLL_INTERVAL as f64) as u64;

    // Calculates current time window's EMA value (including last sample)
    let prev_ema_frequency = calculate_ema(&freq_queue);

    // Add seen frequency data to the frequency histogram.
    freq_queue.push_back(frequency);
    if freq_queue.len() == FREQUENCY_QUEUE_SIZE + 1 {
        freq_queue.pop_front();
    }

    // Calculates current time window's EMA value (including last sample)
    let curr_ema_frequency = calculate_ema(&freq_queue);

    // Adapts the thread count of pool
    //
    // Sliding window of frequencies visited by the pool manager.
    // Pool manager creates EMA value for previous window and current window.
    // Compare them to determine scaling amount based on the trends.
    // If current EMA value is bigger, we will scale up.
    if curr_ema_frequency > prev_ema_frequency {
        // "Scale by" amount can be seen as "how much load is coming".
        // "Scale" amount is "how many threads we should spawn".
        let scale_by: f64 = curr_ema_frequency - prev_ema_frequency;
        let scale = ((LOW_WATERMARK as f64 * scale_by) + LOW_WATERMARK as f64) as u64;

        // It is time to scale the pool!
        (0..scale).for_each(|_| {
            create_blocking_thread();
        });
    } else if (curr_ema_frequency - prev_ema_frequency).abs() < std::f64::EPSILON
        && current_frequency != 0
    {
        // Throughput is low. Allocate more threads to unblock flow.
        // If we fall to this case, scheduler is congested by longhauling tasks.
        // For unblock the flow we should add up some threads to the pool, but not that many to
        // stagger the program's operation.
        let scale = LOW_WATERMARK * current_frequency + 1;

        // Scale it up!
        (0..scale).for_each(|_| {
            create_blocking_thread();
        });
    }

    FREQUENCY.store(0, Ordering::Release);
}

/// Creates blocking thread to receive tasks
/// Dynamic threads will terminate themselves if they don't
/// receive any work after between one and ten seconds.
fn create_blocking_thread() {
    // Check that thread is spawnable.
    // If it hits to the OS limits don't spawn it.
    {
        let current_arc = POOL_SIZE.clone();
        let pool_size = *current_arc.lock().unwrap();
        if pool_size >= MAX_THREADS.load(Ordering::SeqCst) {
            MAX_THREADS.store(10_000, Ordering::SeqCst);
            return;
        }
    }
    // We want to avoid having all threads terminate at
    // exactly the same time, causing thundering herd
    // effects. We want to stagger their destruction over
    // 10 seconds or so to make the costs fade into
    // background noise.
    //
    // Generate a simple random number of milliseconds
    let rand_sleep_ms = u64::from(random(10_000));

    let _ = thread::Builder::new()
        .name("async-blocking-driver-dynamic".to_string())
        .spawn(move || {
            let wait_limit = Duration::from_millis(1000 + rand_sleep_ms);

            // Adjust the pool size counter before and after spawn
            {
                let current_arc = POOL_SIZE.clone();
                *current_arc.lock().unwrap() += 1;
            }
            while let Ok(task) = POOL.receiver.recv_timeout(wait_limit) {
                abort_on_panic(|| task.run());
            }
            {
                let current_arc = POOL_SIZE.clone();
                *current_arc.lock().unwrap() -= 1;
            }
        })
        .map_err(|err| {
            match err.kind() {
                ErrorKind::WouldBlock => {
                    // Maximum allowed threads per process is varying from system to system.
                    // Also, some systems have it(like macOS), and some don't(Linux).
                    // This case expected not to happen.
                    // But when happened this shouldn't throw a panic.
                    let current_arc = POOL_SIZE.clone();
                    MAX_THREADS.store(*current_arc.lock().unwrap() - 1, Ordering::SeqCst);
                }
                _ => eprintln!(
                    "cannot start a dynamic thread driving blocking tasks: {}",
                    err
                ),
            }
        });
}

/// Enqueues work, attempting to send to the thread pool in a
/// nonblocking way and spinning up needed amount of threads
/// based on the previous statistics without relying on
/// if there is not a thread ready to accept the work or not.
fn schedule(t: async_task::Task<()>) {
    // Add up for every incoming scheduled task
    FREQUENCY.fetch_add(1, Ordering::Acquire);

    if let Err(err) = POOL.sender.try_send(t) {
        // We were not able to send to the channel without
        // blocking.
        POOL.sender.send(err.into_inner()).unwrap();
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
