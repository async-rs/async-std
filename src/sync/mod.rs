//! Synchronization primitives.
//!
//! This module is an async version of [`std::sync`].
//!
//! [`std::sync`]: https://doc.rust-lang.org/std/sync/index.html
//!
//! # Examples
//!
//! Spawn a task that updates an integer protected by a mutex:
//!
//! ```
//! # fn main() { async_std::task::block_on(async {
//! #
//! use std::sync::Arc;
//!
//! use async_std::sync::Mutex;
//! use async_std::task;
//!
//! let m1 = Arc::new(Mutex::new(0));
//! let m2 = m1.clone();
//!
//! task::spawn(async move {
//!     *m2.lock().await = 1;
//! })
//! .await;
//!
//! assert_eq!(*m1.lock().await, 1);
//! #
//! # }) }
//! ```

#[doc(inline)]
pub use std::sync::{Arc, Weak};

#[cfg(any(feature = "unstable", feature = "docs"))]
pub use barrier::{Barrier, BarrierWaitResult};

pub use mutex::{Mutex, MutexGuard};
pub use rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

#[cfg(any(feature = "unstable", feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
mod barrier;
mod mutex;
mod rwlock;
