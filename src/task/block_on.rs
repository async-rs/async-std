use std::future::Future;

use kv_log_macro::trace;

use crate::task::Task;

/// Spawns a task and blocks the current thread on its result.
///
/// Calling this function is similar to [spawning] a thread and immediately [joining] it, except an
/// asynchronous task will be spawned.
///
/// See also: [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// [spawning]: https://doc.rust-lang.org/std/thread/fn.spawn.html
/// [joining]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
///
/// # Examples
///
/// ```no_run
/// use async_std::task;
///
/// fn main() {
///     task::block_on(async {
///         println!("Hello, world!");
///     })
/// }
/// ```
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    // Create a new task handle.
    let task = Task::new(None);

    // Log this `block_on` operation.
    trace!("block_on", {
        task_id: task.id().0,
        parent_task_id: Task::get_current(|t| t.id().0).unwrap_or(0),
    });

    let wrapped_future = async move {
        // Drop task-locals on exit.
        // defer! {
        //     Task::get_current(|t| unsafe { t.drop_locals() });
        // }

        // Log completion on exit.
        defer! {
            trace!("completed", {
                task_id: Task::get_current(|t| t.id().0),
            });
        }

        future.await
    };

    once_cell::sync::Lazy::force(&crate::rt::RUNTIME);

    // Run the future as a task.
    unsafe { Task::set_current(&task, || smol::block_on(wrapped_future)) }
}
