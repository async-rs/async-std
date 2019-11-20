use std::cell::Cell;
use std::iter;
use std::thread;
use std::time::Duration;

use crossbeam_deque::{Injector, Stealer, Worker};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

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

        let proc = Processor {
            worker,
            slot: Cell::new(None),
            slot_runs: Cell::new(0),
        };

        thread::Builder::new()
            .name("async-std/executor".to_string())
            .spawn(|| {
                let _ = PROCESSOR.with(|p| p.set(proc));
                abort_on_panic(main_loop);
            })
            .expect("cannot start a thread driving tasks");
    }

    Pool {
        injector: Injector::new(),
        stealers,
        sleepers: Sleepers::new(),
    }
});

/// The state of a worker thread.
struct Processor {
    /// The local task queue.
    worker: Worker<Runnable>,

    /// Contains the next task to run as an optimization that skips queues.
    slot: Cell<Option<Runnable>>,

    /// How many times in a row tasks have been taked from the slot rather than the queue.
    slot_runs: Cell<u32>,
}

thread_local! {
    /// Worker thread state.
    static PROCESSOR: OnceCell<Processor> = OnceCell::new();
}

/// Schedules a new runnable task for execution.
pub(crate) fn schedule(task: Runnable) {
    PROCESSOR.with(|proc| {
        // If the current thread is a worker thread, store it into its task slot or push it into
        // its local task queue. Otherwise, push it into the global task queue.
        match proc.get() {
            Some(proc) => {
                // Replace the task in the slot.
                if let Some(task) = proc.slot.replace(Some(task)) {
                    // If the slot already contained a task, push it into the local task queue.
                    proc.worker.push(task);
                    POOL.sleepers.notify_one();
                }
            }
            None => {
                POOL.injector.push(task);
                POOL.sleepers.notify_one();
            }
        }
    })
}

/// Main loop running a worker thread.
fn main_loop() {
    /// Number of yields when no runnable task is found.
    const YIELDS: u32 = 3;
    /// Number of short sleeps when no runnable task in found.
    const SLEEPS: u32 = 1;

    // The number of times the thread didn't find work in a row.
    let mut fails = 0;

    loop {
        // Try to find a runnable task.
        match find_runnable() {
            Some(task) => {
                fails = 0;

                // Run the found task.
                task.run();
            }
            None => {
                fails += 1;

                // Yield the current thread or put it to sleep.
                if fails <= YIELDS {
                    thread::yield_now();
                } else if fails <= YIELDS + SLEEPS {
                    thread::sleep(Duration::from_micros(10));
                } else {
                    POOL.sleepers.wait();
                    fails = 0;
                }
            }
        }
    }
}

/// Find the next runnable task.
fn find_runnable() -> Option<Runnable> {
    /// Maximum number of times the slot can be used in a row.
    const SLOT_LIMIT: u32 = 16;

    PROCESSOR.with(|proc| {
        let proc = proc.get().unwrap();

        // Try taking a task from the slot.
        let runs = proc.slot_runs.get();
        if runs < SLOT_LIMIT {
            if let Some(task) = proc.slot.take() {
                proc.slot_runs.set(runs + 1);
                return Some(task);
            }
        }
        proc.slot_runs.set(0);

        // Pop a task from the local queue, if not empty.
        proc.worker.pop().or_else(|| {
            // Otherwise, we need to look for a task elsewhere.
            iter::repeat_with(|| {
                // Try stealing a batch of tasks from the global queue.
                POOL.injector
                    .steal_batch_and_pop(&proc.worker)
                    // Or try stealing a batch of tasks from one of the other threads.
                    .or_else(|| {
                        // First, pick a random starting point in the list of local queues.
                        let len = POOL.stealers.len();
                        let start = random(len as u32) as usize;

                        // Try stealing a batch of tasks from each local queue starting from the
                        // chosen point.
                        let (l, r) = POOL.stealers.split_at(start);
                        let stealers = r.iter().chain(l.iter());
                        stealers
                            .map(|s| s.steal_batch_and_pop(&proc.worker))
                            .collect()
                    })
            })
            // Loop while no task was stolen and any steal operation needs to be retried.
            .find(|s| !s.is_retry())
            // Extract the stolen task, if there is one.
            .and_then(|s| s.success())
        })
    })
}
