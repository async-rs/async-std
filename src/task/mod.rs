//! Asynchronous tasks.
//!
//! This module is similar to [`std::thread`], except it uses asynchronous tasks in place of
//! threads.
//!
//! [`std::thread`]: https://doc.rust-lang.org/std/thread/index.html
//!
//! # Examples
//!
//! Spawn a task and await its result:
//!
//! ```
//! # fn main() { async_std::thread::spawn_task(async {
//! #
//! use async_std::task;
//!
//! let handle = task::spawn(async {
//!     1 + 2
//! });
//! assert_eq!(handle.await, 3);
//! #
//! # }) }
//! ```

#[doc(inline)]
pub use std::task::{Context, Poll, Waker};

#[cfg(any(feature = "unstable", feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[doc(inline)]
pub use async_macros::ready;

pub use builder::Builder;
pub use pool::spawn;
pub use sleep::sleep;
pub use task::{JoinHandle, Task, TaskId};
pub use task_local::{AccessError, LocalKey};
pub use worker::current;

mod builder;
mod pool;
mod sleep;
mod sleepers;

pub(crate) mod blocking;
pub(crate) mod task;
pub(crate) mod task_local;
pub(crate) mod worker;

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`thread::spawn_task`].
///
/// [`thread::spawn_task`]: fn.block_on.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # fn main() { async_std::thread::spawn_task(async {
/// #
/// use async_std::thread;
///
/// task::blocking(async {
///     println!("long-running task here");
/// }).await;
/// #
/// # }) }
/// ```
// Once this function stabilizes we should merge `blocking::spawn` into this so
// all code in our crate uses `task::blocking` too.
#[cfg(any(feature = "unstable", feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[inline]
pub fn blocking<F, R>(future: F) -> task::JoinHandle<R>
where
    F: crate::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    blocking::spawn(future)
}
