//! An executor that assigns an ID to every spawned task.

#![feature(async_await)]

use std::cell::Cell;
use std::future::Future;
use std::panic::catch_unwind;
use std::thread;

use crossbeam::atomic::AtomicCell;
use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use lazy_static::lazy_static;

#[derive(Clone, Copy, Debug)]
struct TaskId(usize);

thread_local! {
    /// The ID of the current task.
    static TASK_ID: Cell<Option<TaskId>> = Cell::new(None);
}

/// Returns the ID of the currently executing task.
///
/// Returns `None` if called outside the runtime.
fn task_id() -> Option<TaskId> {
    TASK_ID.with(|id| id.get())
}

/// Spawns a future on the executor.
fn spawn<F, R>(future: F) -> async_task::JoinHandle<R, TaskId>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    lazy_static! {
        // A channel that holds scheduled tasks.
        static ref QUEUE: Sender<async_task::Task<TaskId>> = {
            let (sender, receiver) = unbounded::<async_task::Task<TaskId>>();

            // Start the executor thread.
            thread::spawn(|| {
                TASK_ID.with(|id| {
                    for task in receiver {
                        // Store the task ID into the thread-local before running.
                        id.set(Some(*task.tag()));

                        // Ignore panics for simplicity.
                        let _ignore_panic = catch_unwind(|| task.run());
                    }
                })
            });

            sender
        };

        // A counter that assigns IDs to spawned tasks.
        static ref COUNTER: AtomicCell<usize> = AtomicCell::new(0);
    }

    // Reserve an ID for the new task.
    let id = TaskId(COUNTER.fetch_add(1));

    // Create a task that is scheduled by sending itself into the channel.
    let schedule = |task| QUEUE.send(task).unwrap();
    let (task, handle) = async_task::spawn(future, schedule, id);

    // Schedule the task by sending it into the channel.
    task.schedule();

    handle
}

fn main() {
    let mut handles = vec![];

    // Spawn a bunch of tasks.
    for _ in 0..10 {
        handles.push(spawn(async move {
            println!("Hello from task with {:?}", task_id());
        }));
    }

    // Wait for the tasks to finish.
    for handle in handles {
        executor::block_on(handle);
    }
}
