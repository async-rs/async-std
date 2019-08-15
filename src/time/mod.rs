//! Timeouts for async operations.
//!
//! This module is an async extension of [`std::time`].
//!
//! [`std::time`]: https://doc.rust-lang.org/std/time/index.html
//!
//! # Examples
//!
//! Read a line from stdin with a timeout of 5 seconds.
//!
//! ```no_run
//! # #![feature(async_await)]
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use std::time::Duration;
//!
//! use async_std::io;
//! use async_std::prelude::*;
//!
//! let stdin = io::stdin();
//! let mut line = String::new();
//!
//! let n = stdin
//!     .read_line(&mut line)
//!     .timeout(Duration::from_secs(5))
//!     .await??;
//! #
//! # Ok(()) }) }
//! ```

pub use timeout::{Timeout, TimeoutError};

mod timeout;
