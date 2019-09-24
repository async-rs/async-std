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

#[doc(inline)]
pub use std::io::{Error, ErrorKind, IoSlice, IoSliceMut, Result, SeekFrom};

pub use buf_read::{BufRead, Lines};
pub use buf_reader::BufReader;
pub use buf_writer::BufWriter;
pub use copy::copy;
pub use cursor::Cursor;
pub use empty::{empty, Empty};
pub use read::Read;
pub use repeat::{repeat, Repeat};
pub use seek::Seek;
pub use sink::{sink, Sink};
pub use stderr::{stderr, Stderr};
pub use stdin::{stdin, Stdin};
pub use stdout::{stdout, Stdout};
pub use timeout::timeout;
pub use write::Write;

pub mod prelude;

pub(crate) mod buf_read;
pub(crate) mod read;
pub(crate) mod seek;
pub(crate) mod write;

mod buf_reader;
mod buf_writer;
mod copy;
mod cursor;
mod empty;
mod repeat;
mod sink;
mod stderr;
mod stdin;
mod stdout;
mod timeout;
