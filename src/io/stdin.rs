use std::pin::Pin;
use std::sync::Mutex;

use cfg_if::cfg_if;

use crate::future::{self, Future};
use crate::io::{self, Read};
use crate::task::{blocking, Context, Poll};

/// Constructs a new handle to the standard input of the current process.
///
/// This function is an async version of [`std::io::stdin`].
///
/// [`std::io::stdin`]: https://doc.rust-lang.org/std/io/fn.stdin.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::io;
///
/// let stdin = io::stdin();
/// let mut line = String::new();
/// stdin.read_line(&mut line).await?;
/// #
/// # Ok(()) }) }
/// ```
pub fn stdin() -> Stdin {
    Stdin(Mutex::new(State::Idle(Some(Inner {
        stdin: std::io::stdin(),
        line: String::new(),
        buf: Vec::new(),
        last_op: None,
    }))))
}

/// A handle to the standard input of the current process.
///
/// Created by the [`stdin`] function.
///
/// This type is an async version of [`std::io::Stdin`].
///
/// [`stdin`]: fn.stdin.html
/// [`std::io::Stdin`]: https://doc.rust-lang.org/std/io/struct.Stdin.html
#[derive(Debug)]
pub struct Stdin(Mutex<State>);

/// The state of the asynchronous stdin.
///
/// The stdin can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    /// The stdin is idle.
    Idle(Option<Inner>),

    /// The stdin is blocked on an asynchronous operation.
    ///
    /// Awaiting this operation will result in the new state of the stdin.
    Busy(blocking::JoinHandle<State>),
}

/// Inner representation of the asynchronous stdin.
#[derive(Debug)]
struct Inner {
    /// The blocking stdin handle.
    stdin: std::io::Stdin,

    /// The line buffer.
    line: String,

    /// The write buffer.
    buf: Vec<u8>,

    /// The result of the last asynchronous operation on the stdin.
    last_op: Option<Operation>,
}

/// Possible results of an asynchronous operation on the stdin.
#[derive(Debug)]
enum Operation {
    ReadLine(io::Result<usize>),
    Read(io::Result<usize>),
}

impl Stdin {
    /// Reads a line of input into the specified buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::io;
    ///
    /// let stdin = io::stdin();
    /// let mut line = String::new();
    /// stdin.read_line(&mut line).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn read_line(&self, buf: &mut String) -> io::Result<usize> {
        future::poll_fn(|cx| {
            let state = &mut *self.0.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => {
                        let inner = opt.as_mut().unwrap();

                        // Check if the operation has completed.
                        if let Some(Operation::ReadLine(res)) = inner.last_op.take() {
                            let n = res?;

                            // Copy the read data into the buffer and return.
                            buf.push_str(&inner.line);
                            return Poll::Ready(Ok(n));
                        } else {
                            let mut inner = opt.take().unwrap();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                inner.line.clear();
                                let res = inner.stdin.read_line(&mut inner.line);
                                inner.last_op = Some(Operation::ReadLine(res));
                                State::Idle(Some(inner))
                            }));
                        }
                    }
                    // Poll the asynchronous operation the stdin is currently blocked on.
                    State::Busy(task) => *state = futures_core::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .await
    }
}

impl Read for Stdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let state = &mut *self.0.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    let inner = opt.as_mut().unwrap();

                    // Check if the operation has completed.
                    if let Some(Operation::Read(res)) = inner.last_op.take() {
                        let n = res?;

                        // If more data was read than fits into the buffer, let's retry the read
                        // operation.
                        if n <= buf.len() {
                            // Copy the read data into the buffer and return.
                            buf[..n].copy_from_slice(&inner.buf[..n]);
                            return Poll::Ready(Ok(n));
                        }
                    } else {
                        let mut inner = opt.take().unwrap();

                        // Set the length of the inner buffer to the length of the provided buffer.
                        if inner.buf.len() < buf.len() {
                            inner.buf.reserve(buf.len() - inner.buf.len());
                        }
                        unsafe {
                            inner.buf.set_len(buf.len());
                        }

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            let res = std::io::Read::read(&mut inner.stdin, &mut inner.buf);
                            inner.last_op = Some(Operation::Read(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the stdin is currently blocked on.
                State::Busy(task) => *state = futures_core::ready!(Pin::new(task).poll(cx)),
            }
        }
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        use crate::os::unix::io::{AsRawFd, RawFd};
        use crate::os::windows::io::{AsRawHandle, RawHandle};
    } else if #[cfg(unix)] {
        use std::os::unix::io::{AsRawFd, RawFd};
    } else if #[cfg(windows)] {
        use std::os::windows::io::{AsRawHandle, RawHandle};
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs"))] {
        impl AsRawFd for Stdin {
            fn as_raw_fd(&self) -> RawFd {
                std::io::stdin().as_raw_fd()
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        impl AsRawHandle for Stdin {
            fn as_raw_handle(&self) -> RawHandle {
                std::io::stdin().as_raw_handle()
            }
        }
    }
}
