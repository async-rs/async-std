use std::future::Future;

use crate::task::{Builder, JoinHandle};

/// Spawns a task.
///
/// This function is similar to [`std::thread::spawn`], except it spawns an asynchronous task.
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread/fn.spawn.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// let handle = task::spawn(async {
///     1 + 2
/// });
///
/// assert_eq!(handle.await, 3);
/// #
/// # })
/// ```
///
/// # Blocking
///
/// It is fine to use `spawn` to start tasks that may carry out long-running
/// computations, use blocking I/O primitives, or otherwise engage in activities
/// that mean that polling the task's future may not always return quickly.
///
/// This function begins executing the given future on a thread pool shared with
/// other asynchronous tasks. If polling a particular task takes too long, that
/// thread is removed from the general pool and assigned to run that task
/// exclusively. A new thread is created to take its place in the general pool.
///
/// Although the usual expectation in asynchronous programming is that polling a
/// future should return quickly, in practice it can be hard to anticipate how a
/// given task will behave. A computation that has the potential to take a long
/// time might return quickly in practice, and vice versa. The `spawn` function
/// observes each task's behavior and chooses dynamically whether to assign it a
/// dedicated thread or let it share the thread pool with other tasks.
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(future).expect("cannot spawn task")
}
