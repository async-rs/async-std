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

pub mod prelude;

#[doc(inline)]
pub use std::io::{Error, ErrorKind, Result, SeekFrom};

pub use buf_read::{BufRead, Lines};
pub use buf_reader::BufReader;
pub use copy::copy;
pub use empty::{empty, Empty};
pub use read::Read;
pub use seek::Seek;
pub use sink::{sink, Sink};
pub use stderr::{stderr, Stderr};
pub use stdin::{stdin, Stdin};
pub use stdout::{stdout, Stdout};
pub use timeout::timeout;
pub use write::Write;

mod buf_read;
mod buf_reader;
mod copy;
mod empty;
mod read;
mod seek;
mod sink;
mod stderr;
mod stdin;
mod stdout;
mod timeout;
mod write;
