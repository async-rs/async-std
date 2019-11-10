use std::pin::Pin;
use std::sync::Mutex;
use std::future::Future;

use crate::io::{self, Write};
use crate::task::{spawn_blocking, Context, JoinHandle, Poll};

cfg_unstable! {
    use once_cell::sync::Lazy;
    use std::io::Write as _;
}

/// Constructs a new handle to the standard error of the current process.
///
/// This function is an async version of [`std::io::stderr`].
///
/// [`std::io::stderr`]: https://doc.rust-lang.org/std/io/fn.stderr.html
///
/// ### Note: Windows Portability Consideration
///
/// When operating in a console, the Windows implementation of this stream does not support
/// non-UTF-8 byte sequences. Attempting to write bytes that are not valid UTF-8 will return
/// an error.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::io;
/// use async_std::prelude::*;
///
/// let mut stderr = io::stderr();
/// stderr.write_all(b"Hello, world!").await?;
/// #
/// # Ok(()) }) }
/// ```
pub fn stderr() -> Stderr {
    Stderr(Mutex::new(State::Idle(Some(Inner {
        stderr: std::io::stderr(),
        buf: Vec::new(),
        last_op: None,
    }))))
}

/// A handle to the standard error of the current process.
///
/// This writer is created by the [`stderr`] function. See its documentation for
/// more.
///
/// ### Note: Windows Portability Consideration
///
/// When operating in a console, the Windows implementation of this stream does not support
/// non-UTF-8 byte sequences. Attempting to write bytes that are not valid UTF-8 will return
/// an error.
///
/// [`stderr`]: fn.stderr.html
#[derive(Debug)]
pub struct Stderr(Mutex<State>);

/// A locked reference to the Stderr handle.
///
/// This handle implements the [`Write`] traits, and is constructed via the [`Stderr::lock`]
/// method.
///
/// [`Write`]: trait.Read.html
/// [`Stderr::lock`]: struct.Stderr.html#method.lock
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[derive(Debug)]
pub struct StderrLock<'a>(std::io::StderrLock<'a>);

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
unsafe impl Send for StderrLock<'_> {}

/// The state of the asynchronous stderr.
///
/// The stderr can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    /// The stderr is idle.
    Idle(Option<Inner>),

    /// The stderr is blocked on an asynchronous operation.
    ///
    /// Awaiting this operation will result in the new state of the stderr.
    Busy(JoinHandle<State>),
}

/// Inner representation of the asynchronous stderr.
#[derive(Debug)]
struct Inner {
    /// The blocking stderr handle.
    stderr: std::io::Stderr,

    /// The write buffer.
    buf: Vec<u8>,

    /// The result of the last asynchronous operation on the stderr.
    last_op: Option<Operation>,
}

/// Possible results of an asynchronous operation on the stderr.
#[derive(Debug)]
enum Operation {
    Write(io::Result<usize>),
    Flush(io::Result<()>),
}

impl Stderr {
    /// Locks this handle to the standard error stream, returning a writable guard.
    ///
    /// The lock is released when the returned lock goes out of scope. The returned guard also implements the Write trait for writing data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::io;
    /// use async_std::prelude::*;
    ///
    /// let stderr = io::stderr();
    /// let mut handle = stderr.lock().await;
    ///
    /// handle.write_all(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    #[cfg(any(feature = "unstable", feature = "docs"))]
    pub async fn lock(&self) -> StderrLock<'static> {
        static STDERR: Lazy<std::io::Stderr> = Lazy::new(std::io::stderr);

        spawn_blocking(move || StderrLock(STDERR.lock())).await
    }
}

impl Write for Stderr {
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
                        *state = State::Busy(spawn_blocking(move || {
                            let res = std::io::Write::write(&mut inner.stderr, &inner.buf);
                            inner.last_op = Some(Operation::Write(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the stderr is currently blocked on.
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
                        *state = State::Busy(spawn_blocking(move || {
                            let res = std::io::Write::flush(&mut inner.stderr);
                            inner.last_op = Some(Operation::Flush(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the stderr is currently blocked on.
                State::Busy(task) => *state = futures_core::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

cfg_unix! {
    use crate::os::unix::io::{AsRawFd, RawFd};

    impl AsRawFd for Stderr {
        fn as_raw_fd(&self) -> RawFd {
            std::io::stderr().as_raw_fd()
        }
    }
}

cfg_windows! {
    use crate::os::windows::io::{AsRawHandle, RawHandle};

    impl AsRawHandle for Stderr {
        fn as_raw_handle(&self) -> RawHandle {
            std::io::stderr().as_raw_handle()
        }
    }
}

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
impl io::Write for StderrLock<'_> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(self.0.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(self.0.flush())
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
