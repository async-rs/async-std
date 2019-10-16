//! Traits, helpers, and type definitions for core I/O functionality.
//!
//! The `async_std::io` module contains a number of common things you'll need
//! when doing input and output. The most core part of this module is
//! the [`Read`] and [`Write`] traits, which provide the
//! most general interface for reading and writing input and output.
//!
//! This module is an async version of [`std::io`].
//!
//! [`std::io`]: https://doc.rust-lang.org/std/io/index.html
//!
//! # Read and Write
//!
//! Because they are traits, [`Read`] and [`Write`] are implemented by a number
//! of other types, and you can implement them for your types too. As such,
//! you'll see a few different types of I/O throughout the documentation in
//! this module: [`File`]s, [`TcpStream`]s, and sometimes even [`Vec<T>`]s. For
//! example, [`Read`] adds a [`read`][`Read::read`] method, which we can use on
//! [`File`]s:
//!
//! ```no_run
//! use async_std::prelude::*;
//! use async_std::fs::File;
//!
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! let mut f = File::open("foo.txt").await?;
//! let mut buffer = [0; 10];
//!
//! // read up to 10 bytes
//! let n = f.read(&mut buffer).await?;
//!
//! println!("The bytes: {:?}", &buffer[..n]);
//! #
//! # Ok(()) }) }
//! ```
//!
//! [`Read`] and [`Write`] are so important, implementors of the two traits have a
//! nickname: readers and writers. So you'll sometimes see 'a reader' instead
//! of 'a type that implements the [`Read`] trait'. Much easier!
//!
//! ## Seek and BufRead
//!
//! Beyond that, there are two important traits that are provided: [`Seek`]
//! and [`BufRead`]. Both of these build on top of a reader to control
//! how the reading happens. [`Seek`] lets you control where the next byte is
//! coming from:
//!
//! ```no_run
//! use async_std::prelude::*;
//! use async_std::io::SeekFrom;
//! use async_std::fs::File;
//!
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! let mut f = File::open("foo.txt").await?;
//! let mut buffer = [0; 10];
//!
//! // skip to the last 10 bytes of the file
//! f.seek(SeekFrom::End(-10)).await?;
//!
//! // read up to 10 bytes
//! let n = f.read(&mut buffer).await?;
//!
//! println!("The bytes: {:?}", &buffer[..n]);
//! #
//! # Ok(()) }) }
//! ```
//!
//! [`BufRead`] uses an internal buffer to provide a number of other ways to read, but
//! to show it off, we'll need to talk about buffers in general. Keep reading!

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

// For use in the print macros.
#[doc(hidden)]
pub use stdio::{_eprint, _print};

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
mod stdio;
mod stdout;
mod timeout;
