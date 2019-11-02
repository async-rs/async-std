use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;

use crate::task::{JoinHandle, Task};
use crate::utils::{abort_on_panic, random};

type Runnable = async_task::Task<Task>;

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`task::block_on`], [`task::spawn`].
///
/// [`task::block_on`]: fn.block_on.html
/// [`task::spawn`]: fn.spawn.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # #[cfg(feature = "unstable")]
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// task::spawn_blocking(|| {
///     println!("long-running task here");
/// }).await;
/// #
/// # })
/// ```
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[inline]
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let (task, handle) = async_task::spawn(async { f() }, schedule, Task::new(None));
    task.schedule();
    JoinHandle::new(handle)
}

const MAX_THREADS: u64 = 10_000;

static DYNAMIC_THREAD_COUNT: AtomicU64 = AtomicU64::new(0);

struct Pool {
    sender: Sender<Runnable>,
    receiver: Receiver<Runnable>,
}

static POOL: Lazy<Pool> = Lazy::new(|| {
    for _ in 0..2 {
        thread::Builder::new()
            .name("async-std/blocking".to_string())
            .spawn(|| {
                abort_on_panic(|| {
                    for task in &POOL.receiver {
                        task.run();
                    }
                })
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
    let (sender, receiver) = unbounded();
    Pool { sender, receiver }
});

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

    let n_to_spawn = std::cmp::min(2 + (workers / 10), 10);

    for _ in 0..n_to_spawn {
        // We want to avoid having all threads terminate at
        // exactly the same time, causing thundering herd
        // effects. We want to stagger their destruction over
        // 10 seconds or so to make the costs fade into
        // background noise.
        //
        // Generate a simple random number of milliseconds
        let rand_sleep_ms = u64::from(random(10_000));

        thread::Builder::new()
            .name("async-std/blocking".to_string())
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
}

// Enqueues work, attempting to send to the threadpool in a
// nonblocking way and spinning up another worker thread if
// there is not a thread ready to accept the work.
pub(crate) fn schedule(task: Runnable) {
    if let Err(err) = POOL.sender.try_send(task) {
        // We were not able to send to the channel without
        // blocking. Try to spin up another thread and then
        // retry sending while blocking.
        maybe_create_another_blocking_thread();
        POOL.sender.send(err.into_inner()).unwrap();
    }
}
