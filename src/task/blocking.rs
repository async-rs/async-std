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
// receive any work after one second.
fn maybe_create_another_blocking_thread() {
    // We use a `Relaxed` atomic operation because
    // it's just a heuristic, and would not lose correctness
    // even if it's random.
    let workers = DYNAMIC_THREAD_COUNT.load(Ordering::Relaxed);
    if workers >= MAX_THREADS {
        return;
    }

    thread::Builder::new()
        .name("async-blocking-driver-dynamic".to_string())
        .spawn(|| {
            let wait_limit = Duration::from_secs(1);

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
fn schedule(t: async_task::Task<()>) {
    let first_try_result = POOL.sender.try_send(t);
    match first_try_result {
        Ok(()) => {
            // NICEEEE
        }
        Err(crossbeam::channel::TrySendError::Full(t)) => {
            // We were not able to send to the channel without
            // blocking. Try to spin up another thread and then
            // retry sending while blocking.
            maybe_create_another_blocking_thread();
            POOL.sender.send(t).unwrap()
        }
        Err(crossbeam::channel::TrySendError::Disconnected(_)) => {
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
