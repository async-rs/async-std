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
//! # async_std::task::block_on(async {
//! #
//! use async_std::task;
//!
//! let handle = task::spawn(async {
//!     1 + 2
//! });
//! assert_eq!(handle.await, 3);
//! #
//! # })
//! ```

#[doc(inline)]
pub use std::task::{Context, Poll, Waker};

#[doc(inline)]
pub use async_macros::ready;

pub use block_on::block_on;
pub use builder::Builder;
pub use pool::spawn;
pub use sleep::sleep;
pub use task::{JoinHandle, Task, TaskId};
pub use task_local::{AccessError, LocalKey};
pub use worker::current;

mod block_on;
mod builder;
mod pool;
mod sleep;
mod sleepers;
mod task;
mod task_local;
mod worker;

pub(crate) mod blocking;

cfg_if::cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        mod yield_now;
        pub use yield_now::yield_now;
    }
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`task::block_on`].
///
/// [`task::block_on`]: fn.block_on.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// task::blocking(|| {
///     println!("long-running task here");
/// }).await;
/// #
/// # })
/// ```
// Once this function stabilizes we should merge `blocking::spawn` into this so
// all code in our crate uses `task::blocking` too.
#[cfg(any(feature = "unstable", feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[inline]
pub fn blocking<F, R>(f: F) -> task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    blocking::spawn_blocking(future)
}
