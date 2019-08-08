//! Composable asynchronous iteration.
//!
//! This module is an async version of [`std::iter`].
//!
//! [`std::iter`]: https://doc.rust-lang.org/std/iter/index.html
//!
//! # Examples
//!
//! ```
//! # #![feature(async_await)]
//! # use async_std::prelude::*;
//! # async_std::task::block_on(async {
//! use async_std::stream;
//!
//! let mut stream = stream::repeat(9).take(3);
//! while let Some(num) = stream.next().await {
//!     assert_eq!(num, 9);
//! }
//! # std::io::Result::Ok(())
//! # }).unwrap();
//! ```

#[doc(inline)]
pub use futures::stream::{empty, once, repeat, Empty, Once, Repeat, Stream};
