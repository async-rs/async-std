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
//! # fn main() { async_std::task::block_on(async {
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

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
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
