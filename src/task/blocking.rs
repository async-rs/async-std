//! A thread pool for running blocking functions asynchronously.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use crossbeam::channel::{bounded, Receiver, Sender};
use lazy_static::lazy_static;

use crate::utils::abort_on_panic;

const MAX_THREADS: u64 = 10_000;
const MIN_WAIT_US: u64 = 10;
const MAX_WAIT_US: u64 = 10_000;
const WAIT_SPREAD: u64 = MAX_WAIT_US - MIN_WAIT_US;

static DYNAMIC_THREAD_COUNT: AtomicU64 = AtomicU64::new(0);

struct Pool {
    sender: Sender<async_task::Task<()>>,
    receiver: Receiver<async_task::Task<()>>,
}

lazy_static! {
    static ref POOL: Pool = {
        for _ in 0..2 {
            thread::Builder::new()
                .name("async-blocking-driver".to_string())
                .spawn(|| {
                    for task in &POOL.receiver {
                        abort_on_panic(|| task.run());
                    }
                })
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
// receive any work after a timeout that scales down as the
// total number of threads scales up.
fn maybe_create_another_blocking_thread() {
    // We use a `Relaxed` atomic operation because
    // it's just a heuristic, and would not lose correctness
    // even if it's random.
    let workers = DYNAMIC_THREAD_COUNT.load(Ordering::Relaxed);
    if workers >= MAX_THREADS {
        return;
    }

    // We want to give up earlier when we have more threads
    // to exert backpressure on the system submitting work
    // to do.
    let utilization_percent = (workers * 100) / MAX_THREADS;
    let relative_wait_limit = (WAIT_SPREAD * utilization_percent) / 100;

    // higher utilization -> lower wait time
    let wait_limit_us = MAX_WAIT_US - relative_wait_limit;
    assert!(wait_limit_us >= MIN_WAIT_US);
    let wait_limit = Duration::from_micros(wait_limit_us);

    thread::Builder::new()
        .name("async-blocking-driver-dynamic".to_string())
        .spawn(move || {
            DYNAMIC_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
            while let Ok(task) = POOL.receiver.recv_timeout(wait_limit) {
                abort_on_panic(|| task.run());
            }
            DYNAMIC_THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
        })
        .expect("cannot start a dynamic thread driving blocking tasks");
}

// Enqueues work, blocking on a threadpool for a certain amount of
// time based on the number of worker threads currently active in
// the system. If we cannot send our work to the pool after the
// given timeout, we will attempt to increase the number of
// worker threads active in the system, up to MAX_THREADS. The
// timeout is dynamic, and when we have more threads we block
// for longer before spinning up another thread for backpressure.
fn schedule(t: async_task::Task<()>) {
    // We use a `Relaxed` atomic operation because
    // it's just a heuristic, and would not lose correctness
    // even if it's random.
    let workers = DYNAMIC_THREAD_COUNT.load(Ordering::Relaxed);

    // We want to block for longer when we have more threads to
    // exert backpressure on the system submitting work to do.
    let utilization_percent = (workers * 100) / MAX_THREADS;
    let relative_wait_limit = (WAIT_SPREAD * utilization_percent) / 100;

    // higher utilization -> higher block time
    let wait_limit_us = MIN_WAIT_US + relative_wait_limit;
    assert!(wait_limit_us <= MAX_WAIT_US);
    let wait_limit = Duration::from_micros(wait_limit_us);

    let first_try_result = POOL.sender.send_timeout(t, wait_limit);
    match first_try_result {
        Ok(()) => {
            // NICEEEE
        }
        Err(crossbeam::channel::SendTimeoutError::Timeout(t)) => {
            // We were not able to send to the channel within our
            // budget. Try to spin up another thread, and then
            // block without a time limit on the submission of
            // the task.
            maybe_create_another_blocking_thread();
            POOL.sender.send(t).unwrap()
        }
        Err(crossbeam::channel::SendTimeoutError::Disconnected(_)) => {
            panic!(
                "unable to send to blocking threadpool \
                due to receiver disconnection"
            );
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
