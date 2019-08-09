//! The async prelude.
//!
//! The prelude re-exports the most commonly used traits in async programming.
//!
//! # Examples
//!
//! Import the prelude to use the [`timeout`] combinator:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::{io, prelude::*};
//! use std::time::Duration;
//!
//! let stdin = io::stdin();
//! let mut line = String::new();
//! let dur = Duration::from_secs(5);
//!
//! stdin.read_line(&mut line).timeout(dur).await??;
//! #
//! # Ok(()) }) }
//! ```
//!
//! [`timeout`]: ../time/trait.Timeout.html#method.timeout

#[doc(no_inline)]
pub use futures::future::FutureExt as _;
#[doc(no_inline)]
pub use futures::future::TryFutureExt as _;
#[doc(no_inline)]
pub use futures::io::AsyncBufReadExt as _;
#[doc(no_inline)]
pub use futures::io::AsyncReadExt as _;
#[doc(no_inline)]
pub use futures::io::AsyncSeekExt as _;
#[doc(no_inline)]
pub use futures::io::AsyncWriteExt as _;
#[doc(no_inline)]
pub use futures::stream::StreamExt as _;
#[doc(no_inline)]
pub use futures::stream::TryStreamExt as _;

#[doc(no_inline)]
pub use crate::time::Timeout as _;
