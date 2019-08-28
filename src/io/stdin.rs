use futures::lock::Mutex;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use cfg_if::cfg_if;
use futures::io::{AsyncRead, Initializer};

use crate::future::Future;
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
    Stdin(Mutex::new(Arc::new(StdMutex::new(Inner {
        stdin: io::stdin(),
        line: Default::default(),
        buf: Default::default(),
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
pub struct Stdin(Mutex<Arc<StdMutex<Inner>>>);

/// Inner representation of the asynchronous stdin.
#[derive(Debug)]
struct Inner {
    /// The blocking stdin handle.
    stdin: io::Stdin,

    /// The line buffer.
    line: String,

    /// The write buffer.
    buf: Vec<u8>,
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
        let future_lock = self.0.lock().await;

        let mutex = future_lock.clone();
        // Start the operation asynchronously.
        let handle = blocking::spawn(async move {
            let mut guard = mutex.lock().unwrap();
            let inner: &mut Inner = &mut guard;

            inner.line.clear();
            inner.stdin.read_line(&mut inner.line)
        });

        let res = handle.await;
        let n = res?;

        let mutex = future_lock.clone();
        let inner = mutex.lock().unwrap();

        // Copy the read data into the buffer and return.
        buf.push_str(&inner.line);

        Ok(n)
    }
}

impl AsyncRead for Stdin {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let len = buf.len();

        let future_lock = self.0.lock();
        pin_utils::pin_mut!(future_lock);
        let future_lock = futures::ready!(future_lock.poll(cx));

        let mutex = future_lock.clone();
        let handle = blocking::spawn(async move {
            let mut guard = mutex.lock().unwrap();
            let inner: &mut Inner = &mut guard;

            // Set the length of the inner buffer to the length of the provided buffer.
            if inner.buf.len() < len {
                inner.buf.reserve(len - inner.buf.len());
            }
            unsafe {
                inner.buf.set_len(len);
            }

            io::Read::read(&mut inner.stdin, &mut inner.buf)
        });
        pin_utils::pin_mut!(handle);
        handle.poll(cx).map_ok(|n| {
            let mutex = future_lock.clone();
            let inner = mutex.lock().unwrap();

            // Copy the read data into the buffer and return.
            buf[..n].copy_from_slice(&inner.buf[..n]);
            n
        })
    }

    #[inline]
    unsafe fn initializer(&self) -> Initializer {
        Initializer::nop()
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
                io::stdin().as_raw_fd()
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        impl AsRawHandle for Stdin {
            fn as_raw_handle(&self) -> RawHandle {
                io::stdin().as_raw_handle()
            }
        }
    }
}
