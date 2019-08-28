use std::io;
use std::pin::Pin;
use std::sync::Arc;
use futures::lock::Mutex;

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
    Stdin(Arc::new(Mutex::new(io::stdin())))
}

/// A handle to the standard input of the current process.
///
/// Created by the [`stdin`] function.
///
/// This type is an async version of [`std::io::Stdin`].
///
/// [`stdin`]: fn.stdin.html
/// [`std::io::Stdin`]: https://doc.rust-lang.org/std/io/struct.Stdin.html
#[derive(Debug, Clone)]
pub struct Stdin(Arc<Mutex<io::Stdin>>);

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
        let this = self.clone();

        let handle = blocking::spawn(async move {
            let io = this.0.lock().await;

            let mut line = String::new();
            let res = io.read_line(&mut line);
            (res, line)
        });
        let (res, line) = handle.await;

        let n = res?;
        buf.push_str(&line);

        Ok(n)
    }
}

impl AsyncRead for Stdin {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.clone();
        let len = buf.len();

        let handle = blocking::spawn(async move {
            let mut io = this.0.lock().await;

            let mut inner_buf: Vec<u8> = Vec::with_capacity(len);
            unsafe {
                inner_buf.set_len(len);
            }
            let res = io::Read::read(&mut *io, &mut inner_buf);
            res.and_then(|n| {
                unsafe {
                    inner_buf.set_len(n);
                }
                Ok((n, inner_buf))
            })
        });
        pin_utils::pin_mut!(handle);
        handle.poll(cx).map_ok(|(n, inner_buf)| {
            buf[..n].copy_from_slice(&inner_buf[..n]);
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
