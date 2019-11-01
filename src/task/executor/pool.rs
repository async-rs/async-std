use std::cell::UnsafeCell;
use std::iter;
use std::thread;
use std::time::Duration;

use crossbeam_deque::{Injector, Stealer, Worker};
use once_cell::sync::Lazy;

use crate::task::executor::Sleepers;
use crate::task::Runnable;
use crate::utils::{abort_on_panic, random};

/// The state of an executor.
struct Pool {
    /// The global queue of tasks.
    injector: Injector<Runnable>,

    /// Handles to local queues for stealing work from worker threads.
    stealers: Vec<Stealer<Runnable>>,

    /// Used for putting idle workers to sleep and notifying them when new tasks come in.
    sleepers: Sleepers,
}

/// Global executor that runs spawned tasks.
static POOL: Lazy<Pool> = Lazy::new(|| {
    let num_threads = num_cpus::get().max(1);
    let mut stealers = Vec::new();

    // Spawn worker threads.
    for _ in 0..num_threads {
        let worker = Worker::new_fifo();
        stealers.push(worker.stealer());

        thread::Builder::new()
            .name("async-std/executor".to_string())
            .spawn(|| abort_on_panic(|| main_loop(worker)))
            .expect("cannot start a thread driving tasks");
    }

    Pool {
        injector: Injector::new(),
        stealers,
        sleepers: Sleepers::new(),
    }
});

thread_local! {
    /// Local task queue associated with the current worker thread.
    static QUEUE: UnsafeCell<Option<Worker<Runnable>>> = UnsafeCell::new(None);
}

/// Schedules a new runnable task for execution.
pub(crate) fn schedule(task: Runnable) {
    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref() };

        // If the current thread is a worker thread, push the task into its local task queue.
        // Otherwise, push it into the global task queue.
        match local {
            None => POOL.injector.push(task),
            Some(q) => q.push(task),
        }
    });

    // Notify a sleeping worker that new work just came in.
    POOL.sleepers.notify_one();
}

/// Main loop running a worker thread.
fn main_loop(local: Worker<Runnable>) {
    // Initialize the local task queue.
    QUEUE.with(|queue| unsafe { *queue.get() = Some(local) });

    // The number of times the thread didn't find work in a row.
    let mut step = 0;

    loop {
        // Try to find a runnable task.
        match find_runnable() {
            Some(task) => {
                // Found. Now run the task.
                task.run();
                step = 0;
            }
            None => {
                // Yield the current thread or put it to sleep.
                match step {
                    0..=2 => {
                        thread::yield_now();
                        step += 1;
                    }
                    3 => {
                        thread::sleep(Duration::from_micros(10));
                        step += 1;
                    }
                    _ => {
                        POOL.sleepers.wait();
                        step = 0;
                    }
                }
            }
        }
    }
}

/// Find the next runnable task.
fn find_runnable() -> Option<Runnable> {
    let pool = &*POOL;

    QUEUE.with(|queue| {
        let local = unsafe { (*queue.get()).as_ref().unwrap() };

        // Pop a task from the local queue, if not empty.
        local.pop().or_else(|| {
            // Otherwise, we need to look for a task elsewhere.
            iter::repeat_with(|| {
                // Try stealing a batch of tasks from the global queue.
                pool.injector
                    .steal_batch_and_pop(&local)
                    // Or try stealing a batch of tasks from one of the other threads.
                    .or_else(|| {
                        // First, pick a random starting point in the list of local queues.
                        let len = pool.stealers.len();
                        let start = random(len as u32) as usize;

                        // Try stealing a batch of tasks from each local queue starting from the
                        // chosen point.
                        let (l, r) = pool.stealers.split_at(start);
                        let rotated = r.iter().chain(l.iter());
                        rotated.map(|s| s.steal_batch_and_pop(&local)).collect()
                    })
            })
            // Loop while no task was stolen and any steal operation needs to be retried.
            .find(|s| !s.is_retry())
            // Extract the stolen task, if there is one.
            .and_then(|s| s.success())
        })
    })
}
