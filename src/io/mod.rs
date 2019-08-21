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
pub use std::io::SeekFrom;

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
        pub struct Error {
            _private: ()
        }
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
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum ErrorKind {
            /// An entity was not found, often a file.
            NotFound,
            /// The operation lacked the necessary privileges to complete.
            PermissionDenied,
            /// The connection was refused by the remote server.
            ConnectionRefused,
            /// The connection was reset by the remote server.
            ConnectionReset,
            /// The connection was aborted (terminated) by the remote server.
            ConnectionAborted,
            /// The network operation failed because it was not connected yet.
            NotConnected,
            /// A socket address could not be bound because the address is already in
            /// use elsewhere.
            AddrInUse,
            /// A nonexistent interface was requested or the requested address was not
            /// local.
            AddrNotAvailable,
            /// The operation failed because a pipe was closed.
            BrokenPipe,
            /// An entity already exists, often a file.
            AlreadyExists,
            /// The operation needs to block to complete, but the blocking operation was
            /// requested to not occur.
            WouldBlock,
            /// A parameter was incorrect.
            InvalidInput,
            /// Data not valid for the operation were encountered.
            ///
            /// Unlike [`InvalidInput`], this typically means that the operation
            /// parameters were valid, however the error was caused by malformed
            /// input data.
            ///
            /// For example, a function that reads a file into a string will error with
            /// `InvalidData` if the file's contents are not valid UTF-8.
            ///
            /// [`InvalidInput`]: #variant.InvalidInput
            InvalidData,
            /// The I/O operation's timeout expired, causing it to be canceled.
            TimedOut,
            /// An error returned when an operation could not be completed because a
            /// call to [`write`] returned [`Ok(0)`].
            ///
            /// This typically means that an operation could only succeed if it wrote a
            /// particular number of bytes but only a smaller number of bytes could be
            /// written.
            ///
            /// [`write`]: ../io/trait.Write.html#tymethod.write
            /// [`Ok(0)`]: ../io/type.Result.html
            WriteZero,
            /// This operation was interrupted.
            ///
            /// Interrupted operations can typically be retried.
            Interrupted,
            /// Any I/O error not part of this list.
            Other,
            /// An error returned when an operation could not be completed because an
            /// "end of file" was reached prematurely.
            ///
            /// This typically means that an operation could only succeed if it read a
            /// particular number of bytes but only a smaller number of bytes could be
            /// read.
            UnexpectedEof,
        }
    } else {
        #[doc(inline)]
        pub use std::io::ErrorKind;
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// A specialized [`Result`] type for I/O
        /// operations.
        ///
        /// This type is broadly used across [`std::io`] for any operation which may
        /// produce an error.
        ///
        /// This typedef is generally used to avoid writing out [`io::Error`] directly and
        /// is otherwise a direct mapping to [`Result`].
        ///
        /// While usual Rust style is to import types directly, aliases of [`Result`]
        /// often are not, to make it easier to distinguish between them. [`Result`] is
        /// generally assumed to be [`std::result::Result`][`Result`], and so users of this alias
        /// will generally use `io::Result` instead of shadowing the prelude's import
        /// of [`std::result::Result`][`Result`].
        ///
        /// [`std::io`]: ../io/index.html
        /// [`io::Error`]: ../io/struct.Error.html
        ///
        /// # Examples
        ///
        /// A convenience function that bubbles an `io::Result` to its caller:
        ///
        /// ```
        /// use std::io;
        ///
        /// fn get_string() -> io::Result<String> {
        ///     let mut buffer = String::new();
        ///
        ///     io::stdin().read_line(&mut buffer)?;
        ///
        ///     Ok(buffer)
        /// }
        /// ```
        pub type Result<T> = std::result::Result<T, std::io::Error>;
    } else {
        #[doc(inline)]
        pub use std::io::Result;
    }
}
