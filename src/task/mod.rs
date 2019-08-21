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
pub use std::task::{Context, Poll};

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

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// A `Waker` is a handle for waking up a task by notifying its executor
        /// that it is ready to be run.
        ///
        /// This handle encapsulates a [`RawWaker`][`std::task::RawWaker`]
        /// instance, which defines the executor-specific wakeup behavior.
        ///
        /// Implements [`Clone`], [`trait@Send`], and [`trait@Sync`].
        pub struct Waker {
            _private: (),
        }
    } else {
        #[doc(inline)]
        pub use std::task::Waker;
    }
}
