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
pub use std::io::{Result, SeekFrom};

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

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// The error type for I/O operations of the [`Read`], [`Write`], [`Seek`], and
        /// associated traits.
        ///
        /// Errors mostly originate from the underlying OS, but custom instances of
        /// `Error` can be created with crafted error messages and a particular value of
        /// [`ErrorKind`].
        ///
        /// [`Read`]: ../io/trait.Read.html
        /// [`Write`]: ../io/trait.Write.html
        /// [`Seek`]: ../io/trait.Seek.html
        /// [`ErrorKind`]: enum.ErrorKind.html
        pub struct Error {}
    } else {
        #[doc(inline)]
        pub use std::io::Error;
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// A list specifying general categories of I/O error.
        ///
        /// This list is intended to grow over time and it is not recommended to
        /// exhaustively match against it.
        ///
        /// It is used with the [`io::Error`] type.
        ///
        /// [`io::Error`]: struct.Error.html
        pub enum ErrorKind {}
    } else {
        #[doc(inline)]
        pub use std::io::ErrorKind;
    }
}
