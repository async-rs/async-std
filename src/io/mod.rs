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
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::io;
//!
//! let stdin = io::stdin();
//! let mut line = String::new();
//! stdin.read_line(&mut line).await?;
//! #
//! # Ok(()) }) }
//! ```

#[doc(inline)]
pub use std::io::{empty, sink, Cursor, Empty, Error, ErrorKind, Result, SeekFrom, Sink};

pub use buf_read::{BufRead, Lines};
pub use buf_reader::BufReader;
pub use copy::copy;
pub use read::Read;
pub use seek::Seek;
pub use stderr::{stderr, Stderr};
pub use stdin::{stdin, Stdin};
pub use stdout::{stdout, Stdout};
pub use timeout::timeout;
pub use write::Write;

mod buf_read;
mod buf_reader;
mod copy;
mod read;
mod seek;
mod stderr;
mod stdin;
mod stdout;
mod timeout;
mod write;
