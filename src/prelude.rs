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
pub use crate::future::Future;
#[doc(no_inline)]
pub use crate::io::BufRead as _;
#[doc(no_inline)]
pub use crate::io::Read as _;
#[doc(no_inline)]
pub use crate::io::Seek as _;
#[doc(no_inline)]
pub use crate::io::Write as _;
#[doc(no_inline)]
pub use crate::stream::Stream;
#[doc(no_inline)]
pub use crate::time::Timeout as _;
