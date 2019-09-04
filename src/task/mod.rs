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
//! #
//! # }) }
//! ```

#[doc(inline)]
pub use std::task::{Context, Poll, Waker};

pub use block_on::block_on;
pub use local::{AccessError, LocalKey};
pub use pool::{current, spawn, Builder};
pub use sleep::sleep;
pub use task::{JoinHandle, Task, TaskId};

mod block_on;
mod local;
mod pool;
mod sleep;
mod task;

pub(crate) mod blocking;
