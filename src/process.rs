//! A module for working with processes.
//!
//! This module is mostly concerned with spawning and interacting with child processes, but it also
//! provides abort and exit for terminating the current process.
//!
//! This is an async version of [`std::process`].
//!
//! [`std::process`]: https://doc.rust-lang.org/std/process/index.html

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[doc(inline)]
pub use async_process::*;
