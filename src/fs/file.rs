//! Types for working with files.

use std::fs;
use std::io::{Read as _, Seek, SeekFrom, Write as _};
use std::path::Path;
use std::pin::Pin;
use std::sync::Mutex;

use cfg_if::cfg_if;
use futures::future::{self, FutureExt, TryFutureExt};
use futures::io::{AsyncRead, AsyncSeek, AsyncWrite, Initializer};

use crate::future::Future;
use crate::io;
use crate::task::{blocking, Context, Poll};

/// A reference to a file on the filesystem.
///
/// An instance of a `File` can be read and/or written depending on what options it was opened
/// with.
///
/// Files are automatically closed when they go out of scope. Errors detected on closing are
/// ignored by the implementation of `Drop`. Use the method [`sync_all`] if these errors must be
/// manually handled.
///
/// This type is an async version of [`std::fs::File`].
///
/// [`sync_all`]: struct.File.html#method.sync_all
/// [`std::fs::File`]: https://doc.rust-lang.org/std/fs/struct.File.html
///
/// # Examples
///
/// Create a new file and write some bytes to it:
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs::File;
/// use async_std::prelude::*;
///
/// let mut file = File::create("a.txt").await?;
/// file.write_all(b"Hello, world!").await?;
/// #
/// # Ok(()) }) }
/// ```
///
/// Read the contents of a file into a `Vec<u8>`:
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs::File;
/// use async_std::prelude::*;
///
/// let mut file = File::open("a.txt").await?;
/// let mut contents = Vec::new();
/// file.read_to_end(&mut contents).await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug)]
pub struct File {
    mutex: Mutex<State>,

    #[cfg(unix)]
    raw_fd: std::os::unix::io::RawFd,

    #[cfg(windows)]
    raw_handle: UnsafeShared<std::os::windows::io::RawHandle>,
}

/// The state of an asynchronous file.
///
/// The file can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    /// The file is idle.
    ///
    /// If the inner representation is `None`, that means the file is closed.
    Idle(Option<Inner>),

    /// The file is blocked on an asynchronous operation.
    ///
    /// Awaiting this operation will result in the new state of the file.
    Busy(blocking::JoinHandle<State>),
}

/// Inner representation of an asynchronous file.
#[derive(Debug)]
struct Inner {
    /// The blocking file handle.
    file: fs::File,

    /// The read/write buffer.
    buf: Vec<u8>,

    /// The result of the last asynchronous operation on the file.
    last_op: Option<Operation>,
}

/// Possible results of an asynchronous operation on a file.
#[derive(Debug)]
enum Operation {
    Read(io::Result<usize>),
    Write(io::Result<usize>),
    Seek(io::Result<u64>),
    Flush(io::Result<()>),
}

impl File {
    /// Opens a file in read-only mode.
    ///
    /// See the [`OpenOptions::open`] method for more details.
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not already exist.
    /// Other errors may also be returned according to [`OpenOptions::open`].
    ///
    /// [`OpenOptions::open`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    ///
    /// let file = File::open("a.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn open<P: AsRef<Path>>(path: P) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let file = blocking::spawn(async move { fs::File::open(&path) }).await?;

        #[cfg(unix)]
        let file = File {
            raw_fd: file.as_raw_fd(),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        #[cfg(windows)]
        let file = File {
            raw_handle: UnsafeShared(file.as_raw_handle()),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        Ok(file)
    }

    /// Opens a file in write-only mode.
    ///
    /// This function will create a file if it does not exist, and will truncate it if it does.
    ///
    /// See the [`OpenOptions::open`] function for more details.
    ///
    /// [`OpenOptions::open`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    ///
    /// let file = File::create("a.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn create<P: AsRef<Path>>(path: P) -> io::Result<File> {
        let path = path.as_ref().to_owned();
        let file = blocking::spawn(async move { fs::File::create(&path) }).await?;

        #[cfg(unix)]
        let file = File {
            raw_fd: file.as_raw_fd(),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        #[cfg(windows)]
        let file = File {
            raw_handle: UnsafeShared(file.as_raw_handle()),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        Ok(file)
    }

    /// Attempts to synchronize all OS-internal metadata to disk.
    ///
    /// This function will attempt to ensure that all in-memory data reaches the filesystem before
    /// returning.
    ///
    /// This can be used to handle errors that would otherwise only be caught when the `File` is
    /// closed. Dropping a file will ignore errors in synchronizing this in-memory data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut file = File::create("a.txt").await?;
    /// file.write_all(b"Hello, world!").await?;
    /// file.sync_all().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn sync_all(&self) -> io::Result<()> {
        future::poll_fn(|cx| {
            let state = &mut *self.mutex.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file.sync_all();
                                let _ = s.send(res);
                                State::Idle(Some(inner))
                            }));

                            return Poll::Ready(Some(r));
                        }
                    },
                    // Poll the asynchronous operation the file is currently blocked on.
                    State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .map(|opt| opt.ok_or_else(|| io_error("file closed")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Similar to [`sync_all`], except that it may not synchronize file metadata.
    ///
    /// This is intended for use cases that must synchronize content, but don't need the metadata
    /// on disk. The goal of this method is to reduce disk operations.
    ///
    /// Note that some platforms may simply implement this in terms of [`sync_all`].
    ///
    /// [`sync_all`]: struct.File.html#method.sync_all
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut file = File::create("a.txt").await?;
    /// file.write_all(b"Hello, world!").await?;
    /// file.sync_data().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn sync_data(&self) -> io::Result<()> {
        future::poll_fn(|cx| {
            let state = &mut *self.mutex.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file.sync_data();
                                let _ = s.send(res);
                                State::Idle(Some(inner))
                            }));

                            return Poll::Ready(Some(r));
                        }
                    },
                    // Poll the asynchronous operation the file is currently blocked on.
                    State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .map(|opt| opt.ok_or_else(|| io_error("file closed")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Truncates or extends the underlying file.
    ///
    /// If the `size` is less than the current file's size, then the file will be truncated. If it
    /// is greater than the current file's size, then the file will be extended to `size` and have
    /// all of the intermediate data filled in with zeros.
    ///
    /// The file's cursor isn't changed. In particular, if the cursor was at the end and the file
    /// is truncated using this operation, the cursor will now be past the end.
    ///
    /// # Errors
    ///
    /// This function will return an error if the file is not opened for writing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    ///
    /// let file = File::create("a.txt").await?;
    /// file.set_len(10).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn set_len(&self, size: u64) -> io::Result<()> {
        future::poll_fn(|cx| {
            let state = &mut *self.mutex.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file.set_len(size);
                                let _ = s.send(res);
                                State::Idle(Some(inner))
                            }));

                            return Poll::Ready(Some(r));
                        }
                    },
                    // Poll the asynchronous operation the file is currently blocked on.
                    State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .map(|opt| opt.ok_or_else(|| io_error("file closed")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Queries metadata about the file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    ///
    /// let file = File::open("a.txt").await?;
    /// let metadata = file.metadata().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn metadata(&self) -> io::Result<fs::Metadata> {
        future::poll_fn(|cx| {
            let state = &mut *self.mutex.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file.metadata();
                                let _ = s.send(res);
                                State::Idle(Some(inner))
                            }));

                            return Poll::Ready(Some(r));
                        }
                    },
                    // Poll the asynchronous operation the file is currently blocked on.
                    State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .map(|opt| opt.ok_or_else(|| io_error("file closed")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Changes the permissions on the underlying file.
    ///
    /// # Errors
    ///
    /// This function will return an error if the user lacks permission to change attributes on the
    /// underlying file, but may also return an error in other OS-specific cases.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    ///
    /// let file = File::create("a.txt").await?;
    ///
    /// let mut perms = file.metadata().await?.permissions();
    /// perms.set_readonly(true);
    /// file.set_permissions(perms).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn set_permissions(&self, perm: fs::Permissions) -> io::Result<()> {
        let mut perm = Some(perm);

        future::poll_fn(|cx| {
            let state = &mut *self.mutex.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();
                            let perm = perm.take().unwrap();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file.set_permissions(perm);
                                let _ = s.send(res);
                                State::Idle(Some(inner))
                            }));

                            return Poll::Ready(Some(r));
                        }
                    },
                    // Poll the asynchronous operation the file is currently blocked on.
                    State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
                }
            }
        })
        .map(|opt| opt.ok_or_else(|| io_error("file closed")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }
}

impl AsyncRead for File {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }

    #[inline]
    unsafe fn initializer(&self) -> Initializer {
        Initializer::nop()
    }
}

impl AsyncRead for &File {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let state = &mut *self.mutex.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    // Grab a reference to the inner representation of the file or return an error
                    // if the file is closed.
                    let inner = opt.as_mut().ok_or_else(|| io_error("file closed"))?;
                    let mut offset = 0;

                    // Check if the operation has completed.
                    if let Some(Operation::Read(res)) = inner.last_op.take() {
                        let n = res?;

                        if n <= buf.len() {
                            // Copy the read data into the buffer and return.
                            buf[..n].copy_from_slice(&inner.buf[..n]);
                            return Poll::Ready(Ok(n));
                        }

                        // If more data was read than fits into the buffer, let's retry the read
                        // operation, but first move the cursor where it was before the previous
                        // read.
                        offset = n;
                    }

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
                        if offset > 0 {
                            let pos = SeekFrom::Current(-(offset as i64));
                            let _ = Seek::seek(&mut inner.file, pos);
                        }

                        let res = inner.file.read(&mut inner.buf);
                        inner.last_op = Some(Operation::Read(res));
                        State::Idle(Some(inner))
                    }));
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    #[inline]
    unsafe fn initializer(&self) -> Initializer {
        Initializer::nop()
    }
}

impl AsyncWrite for File {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

impl AsyncWrite for &File {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let state = &mut *self.mutex.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    // Grab a reference to the inner representation of the file or return an error
                    // if the file is closed.
                    let inner = opt.as_mut().ok_or_else(|| io_error("file closed"))?;

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
                            let res = inner.file.write(&mut inner.buf);
                            inner.last_op = Some(Operation::Write(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = &mut *self.mutex.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    // Grab a reference to the inner representation of the file or return if the
                    // file is closed.
                    let inner = match opt.as_mut() {
                        None => return Poll::Ready(Ok(())),
                        Some(s) => s,
                    };

                    // Check if the operation has completed.
                    if let Some(Operation::Flush(res)) = inner.last_op.take() {
                        return Poll::Ready(res);
                    } else {
                        let mut inner = opt.take().unwrap();

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            let res = inner.file.flush();
                            inner.last_op = Some(Operation::Flush(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = &mut *self.mutex.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    // Grab a reference to the inner representation of the file or return if the
                    // file is closed.
                    let inner = match opt.take() {
                        None => return Poll::Ready(Ok(())),
                        Some(s) => s,
                    };

                    // Start the operation asynchronously.
                    *state = State::Busy(blocking::spawn(async move {
                        drop(inner);
                        State::Idle(None)
                    }));
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }
}

impl AsyncSeek for File {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        Pin::new(&mut &*self).poll_seek(cx, pos)
    }
}

impl AsyncSeek for &File {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        let state = &mut *self.mutex.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    // Grab a reference to the inner representation of the file or return an error
                    // if the file is closed.
                    let inner = opt.as_mut().ok_or_else(|| io_error("file closed"))?;

                    // Check if the operation has completed.
                    if let Some(Operation::Seek(res)) = inner.last_op.take() {
                        return Poll::Ready(res);
                    } else {
                        let mut inner = opt.take().unwrap();

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            let res = inner.file.seek(pos);
                            inner.last_op = Some(Operation::Seek(res));
                            State::Idle(Some(inner))
                        }));
                    }
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }
}

/// Creates a custom `io::Error` with an arbitrary error type.
fn io_error(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

impl From<std::fs::File> for File {
    /// Converts a `std::fs::File` into its asynchronous equivalent.
    fn from(file: fs::File) -> File {
        #[cfg(unix)]
        let file = File {
            raw_fd: file.as_raw_fd(),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        #[cfg(windows)]
        let file = File {
            raw_handle: UnsafeShared(file.as_raw_handle()),
            mutex: Mutex::new(State::Idle(Some(Inner {
                file,
                buf: Vec::new(),
                last_op: None,
            }))),
        };

        file
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
        use crate::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    } else if #[cfg(unix)] {
        use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
    } else if #[cfg(windows)] {
        use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs"))] {
        impl AsRawFd for File {
            fn as_raw_fd(&self) -> RawFd {
                self.raw_fd
            }
        }

        impl FromRawFd for File {
            unsafe fn from_raw_fd(fd: RawFd) -> File {
                fs::File::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for File {
            fn into_raw_fd(self) -> RawFd {
                self.raw_fd
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        impl AsRawHandle for File {
            fn as_raw_handle(&self) -> RawHandle {
                self.raw_handle.0
            }
        }

        impl FromRawHandle for File {
            unsafe fn from_raw_handle(handle: RawHandle) -> File {
                fs::File::from_raw_handle(handle).into()
            }
        }

        impl IntoRawHandle for File {
            fn into_raw_handle(self) -> RawHandle {
                self.raw_handle.0
            }
        }

        #[derive(Debug)]
        struct UnsafeShared<T>(T);

        unsafe impl<T> Send for UnsafeShared<T> {}
        unsafe impl<T> Sync for UnsafeShared<T> {}
    }
}
