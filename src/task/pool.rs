use std::iter;
use std::thread;

use crossbeam_deque::{Injector, Stealer, Worker};
use kv_log_macro::trace;
use lazy_static::lazy_static;

use super::sleepers::Sleepers;
use super::task;
use super::task_local;
use super::worker;
use super::{Builder, JoinHandle};
use crate::future::Future;
use crate::utils::abort_on_panic;

/// Spawns a task.
///
/// This function is similar to [`std::thread::spawn`], except it spawns an asynchronous task.
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread/fn.spawn.html
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// let handle = task::spawn(async {
///     1 + 2
/// });
///
/// assert_eq!(handle.await, 3);
/// #
/// # }) }
/// ```
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(future).expect("cannot spawn future")
}

pub(crate) struct Pool {
    pub injector: Injector<task::Runnable>,
    pub stealers: Vec<Stealer<task::Runnable>>,
    pub sleepers: Sleepers,
}

impl Pool {
    /// Spawn a future onto the pool.
    pub fn spawn<F, T>(&self, future: F, builder: Builder) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let tag = task::Tag::new(builder.name);

        // Log this `spawn` operation.
        let child_id = tag.task_id().as_u64();
        let parent_id = worker::get_task(|t| t.id().as_u64()).unwrap_or(0);

        trace!("spawn", {
            parent_id: parent_id,
            child_id: child_id,
        });

        // Wrap the future into one that drops task-local variables on exit.
        let future = unsafe { task_local::add_finalizer(future) };

        // Wrap the future into one that logs completion on exit.
        let future = async move {
            let res = future.await;
            trace!("spawn completed", {
                parent_id: parent_id,
                child_id: child_id,
            });
            res
        };

        let (task, handle) = async_task::spawn(future, worker::schedule, tag);
        task.schedule();
        JoinHandle::new(handle)
    }

    /// Find the next runnable task to run.
    pub fn find_task(&self, local: &Worker<task::Runnable>) -> Option<task::Runnable> {
        // Pop a task from the local queue, if not empty.
        local.pop().or_else(|| {
            // Otherwise, we need to look for a task elsewhere.
            iter::repeat_with(|| {
                // Try stealing a batch of tasks from the injector queue.
                self.injector
                    .steal_batch_and_pop(local)
                    // Or try stealing a bach of tasks from one of the other threads.
                    .or_else(|| {
                        self.stealers
                            .iter()
                            .map(|s| s.steal_batch_and_pop(local))
                            .collect()
                    })
            })
            // Loop while no task was stolen and any steal operation needs to be retried.
            .find(|s| !s.is_retry())
            // Extract the stolen task, if there is one.
            .and_then(|s| s.success())
        })
    }
}

#[inline]
pub(crate) fn get() -> &'static Pool {
    lazy_static! {
        static ref POOL: Pool = {
            let num_threads = num_cpus::get().max(1);
            let mut stealers = Vec::new();

            // Spawn worker threads.
            for _ in 0..num_threads {
                let worker = Worker::new_fifo();
                stealers.push(worker.stealer());

                thread::Builder::new()
                    .name("async-task-driver".to_string())
                    .spawn(|| abort_on_panic(|| worker::main_loop(worker)))
                    .expect("cannot start a thread driving tasks");
            }

            Pool {
                injector: Injector::new(),
                stealers,
                sleepers: Sleepers::new(),
            }
        };
    }
    &*POOL
}
