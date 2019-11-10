use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;

use crate::task::{JoinHandle, Task};
use crate::utils::{abort_on_panic, random};

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
    let schedule = |task| POOL.sender.send(task).unwrap();
    let (task, handle) = async_task::spawn(async { f() }, schedule, Task::new(None));
    task.schedule();
    JoinHandle::new(handle)
}

type Runnable = async_task::Task<Task>;

/// The number of sleeping worker threads.
static SLEEPING: AtomicUsize = AtomicUsize::new(0);

struct Pool {
    sender: Sender<Runnable>,
    receiver: Receiver<Runnable>,
}

static POOL: Lazy<Pool> = Lazy::new(|| {
    // Start a single worker thread waiting for the first task.
    start_thread();

    let (sender, receiver) = unbounded();
    Pool { sender, receiver }
});

fn start_thread() {
    SLEEPING.fetch_add(1, Ordering::SeqCst);

    // Generate a random duration of time between 1 second and 10 seconds. If the thread doesn't
    // receive the next task in this duration of time, it will stop running.
    let timeout = Duration::from_millis(1000 + u64::from(random(9_000)));

    thread::Builder::new()
        .name("async-std/blocking".to_string())
        .spawn(move || {
            loop {
                let task = match POOL.receiver.recv_timeout(timeout) {
                    Ok(task) => task,
                    Err(_) => {
                        // Check whether this is the last sleeping thread.
                        if SLEEPING.fetch_sub(1, Ordering::SeqCst) == 1 {
                            // If so, then restart the thread to make sure there is always at least
                            // one sleeping thread.
                            if SLEEPING.compare_and_swap(0, 1, Ordering::SeqCst) == 0 {
                                continue;
                            }
                        }

                        // Stop the thread.
                        return;
                    }
                };

                // If there are no sleeping threads, then start one to make sure there is always at
                // least one sleeping thread.
                if SLEEPING.fetch_sub(1, Ordering::SeqCst) == 1 {
                    start_thread();
                }

                // Run the task.
                abort_on_panic(|| task.run());

                SLEEPING.fetch_add(1, Ordering::SeqCst);
            }
        })
        .expect("cannot start a blocking thread");
}