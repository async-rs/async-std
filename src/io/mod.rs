//! Basic input and output.
//!
//! This module is an async version of [`std::io`].
//!
//! [`std::io`]: https://doc.rust-lang.org/std/io/index.html
//!
//! # Examples
//!
//! Read a line from the standard input:
//!
//! ```no_run
//! # #![feature(async_await)]
//! use async_std::io;
//!
//! # futures::executor::block_on(async {
//! let stdin = io::stdin();
//! let mut line = String::new();
//! stdin.read_line(&mut line).await?;
//! # std::io::Result::Ok(())
//! # }).unwrap();
//! ```

#[doc(inline)]
pub use futures::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, SeekFrom};

pub use copy::copy;
pub use stderr::{stderr, Stderr};
pub use stdin::{stdin, Stdin};
pub use stdout::{stdout, Stdout};

mod copy;
mod stderr;
mod stdin;
mod stdout;

#[doc(inline)]
pub use std::io::{empty, sink, Cursor, Empty, Error, ErrorKind, Result, Sink};
