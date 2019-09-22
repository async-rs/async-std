use std::pin::Pin;
use std::sync::Mutex;

use cfg_if::cfg_if;

use crate::future::Future;
use crate::io::{self, Write};
use crate::task::{blocking, Context, Poll};

/// Constructs a new handle to the standard output of the current process.
///
/// This function is an async version of [`std::io::stdout`].
///
/// [`std::io::stdout`]: https://doc.rust-lang.org/std/io/fn.stdout.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::io;
/// use async_std::prelude::*;
///
/// let mut stdout = io::stdout();
/// stdout.write_all(b"Hello, world!").await?;
/// #
/// # Ok(()) }) }
/// ```
pub fn stdout() -> Stdout {
    Stdout(Mutex::new(State::Idle(Some(Inner {
        stdout: std::io::stdout(),
        buf: Vec::new(),
        last_op: None,
    }))))
}

/// A handle to the standard output of the current process.
///
/// Created by the [`stdout`] function.
///
/// This type is an async version of [`std::io::Stdout`].
///
/// [`stdout`]: fn.stdout.html
/// [`std::io::Stdout`]: https://doc.rust-lang.org/std/io/struct.Stdout.html
#[derive(Debug)]
pub struct Stdout(Mutex<State>);

/// The state of the asynchronous stdout.
///
/// The stdout can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    /// The stdout is idle.
    Idle(Option<Inner>),

    /// The stdout is blocked on an asynchronous operation.
    ///
    /// Awaiting this operation will result in the new state of the stdout.
    Busy(blocking::JoinHandle<State>),
}

/// Inner representation of the asynchronous stdout.
#[derive(Debug)]
struct Inner {
    /// The blocking stdout handle.
    stdout: std::io::Stdout,

    /// The write buffer.
    buf: Vec<u8>,

    /// The result of the last asynchronous operation on the stdout.
    last_op: Option<Operation>,
}

/// Possible results of an asynchronous operation on the stdout.
#[derive(Debug)]
enum Operation {
    Write(io::Result<usize>),
    Flush(io::Result<()>),
}

impl Write for Stdout {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let state = &mut *self.0.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    let inner = opt.as_mut().unwrap();

                    // Check if the operation has completed.
                    if let Some(Operation::Write(res)) = inner.last_op.take() {
                        let n = res?;

                        // If more data was written than is available in the buffer, let's retry
                        // the write operation.
                        if n <= buf.len() {
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

                        // Copy the data to write into the inner buffer.
                        inner.buf[..buf.len()].copy_from_slice(buf);

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            let res = std::io::Write::write(&mut inner.stdout, &mut inner.buf);
                            inner.last_op = Some(Operation::Write(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the stdout is currently blocked on.
                State::Busy(task) => *state = futures_core::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = &mut *self.0.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    let inner = opt.as_mut().unwrap();

                    // Check if the operation has completed.
                    if let Some(Operation::Flush(res)) = inner.last_op.take() {
                        return Poll::Ready(res);
                    } else {
                        let mut inner = opt.take().unwrap();

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            let res = std::io::Write::flush(&mut inner.stdout);
                            inner.last_op = Some(Operation::Flush(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the stdout is currently blocked on.
                State::Busy(task) => *state = futures_core::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
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
        impl AsRawFd for Stdout {
            fn as_raw_fd(&self) -> RawFd {
                std::io::stdout().as_raw_fd()
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        impl AsRawHandle for Stdout {
            fn as_raw_handle(&self) -> RawHandle {
                std::io::stdout().as_raw_handle()
            }
        }
    }
}
