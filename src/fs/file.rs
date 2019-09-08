//! Async file implementation.

use std::cell::UnsafeCell;
use std::cmp;
use std::fs;
use std::io::{Read as _, Seek, SeekFrom, Write as _};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use cfg_if::cfg_if;
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite, Initializer};

use crate::future;
use crate::io::{self, Write};
use crate::task::{self, blocking, Context, Poll, Waker};

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
    file: Arc<fs::File>,
    lock: Lock<State>,
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
        Ok(file.into())
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
        Ok(file.into())
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
        // Flush the write cache before calling `sync_all()`.
        let state = future::poll_fn(|cx| {
            let state = futures_core::ready!(self.lock.poll_lock(cx));
            state.poll_flush(cx)
        })
        .await?;

        blocking::spawn(async move { state.file.sync_all() }).await
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
        // Flush the write cache before calling `sync_data()`.
        let state = future::poll_fn(|cx| {
            let state = futures_core::ready!(self.lock.poll_lock(cx));
            state.poll_flush(cx)
        })
        .await?;

        blocking::spawn(async move { state.file.sync_data() }).await
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
        // Invalidate the read cache and flush the write cache before calling `set_len()`.
        let state = future::poll_fn(|cx| {
            let state = futures_core::ready!(self.lock.poll_lock(cx));
            let state = futures_core::ready!(state.poll_unread(cx))?;
            state.poll_flush(cx)
        })
        .await?;

        blocking::spawn(async move { state.file.set_len(size) }).await
    }

    /// Queries metadata about the file.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        let file = self.file.clone();
        blocking::spawn(async move { file.metadata() }).await
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
        let file = self.file.clone();
        blocking::spawn(async move { file.set_permissions(perm) }).await
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // We need to flush the file on drop. Unfortunately, that is not possible to do in a
        // non-blocking fashion, but our only other option here is losing data remaining in the
        // write cache. Good task schedulers should be resilient to occasional blocking hiccups in
        // file destructors so we don't expect this to be a common problem in practice.
        let _ = task::block_on(self.flush());
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
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let state = futures_core::ready!(self.lock.poll_lock(cx));
        state.poll_read(cx, buf)
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
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let state = futures_core::ready!(self.lock.poll_lock(cx));
        state.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = futures_core::ready!(self.lock.poll_lock(cx));
        state.poll_flush(cx).map(|res| res.map(drop))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let state = futures_core::ready!(self.lock.poll_lock(cx));
        state.poll_close(cx)
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
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        let state = futures_core::ready!(self.lock.poll_lock(cx));
        state.poll_seek(cx, pos)
    }
}

impl From<std::fs::File> for File {
    /// Converts a `std::fs::File` into its asynchronous equivalent.
    fn from(file: fs::File) -> File {
        let file = Arc::new(file);
        File {
            file: file.clone(),
            lock: Lock::new(State {
                file,
                mode: Mode::Idle,
                cache: Vec::new(),
                is_flushed: false,
                last_read_err: None,
                last_write_err: None,
            }),
        }
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
                self.file.as_raw_fd()
            }
        }

        impl FromRawFd for File {
            unsafe fn from_raw_fd(fd: RawFd) -> File {
                fs::File::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for File {
            fn into_raw_fd(self) -> RawFd {
                self.file.as_raw_fd()
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        impl AsRawHandle for File {
            fn as_raw_handle(&self) -> RawHandle {
                self.file.as_raw_handle()
            }
        }

        impl FromRawHandle for File {
            unsafe fn from_raw_handle(handle: RawHandle) -> File {
                fs::File::from_raw_handle(handle).into()
            }
        }

        impl IntoRawHandle for File {
            fn into_raw_handle(self) -> RawHandle {
                self.file.as_raw_handle()
            }
        }
    }
}

/// An async mutex with non-borrowing lock guards.
#[derive(Debug)]
struct Lock<T>(Arc<LockState<T>>);

unsafe impl<T: Send> Send for Lock<T> {}
unsafe impl<T: Send> Sync for Lock<T> {}

#[derive(Debug)]
/// The state of a lock.
struct LockState<T> {
    /// Set to `true` when locked.
    locked: AtomicBool,

    /// The inner value.
    value: UnsafeCell<T>,

    /// A list of tasks interested in locking.
    wakers: Mutex<Vec<Waker>>,
}

impl<T> Lock<T> {
    /// Creates a new lock with the given value.
    fn new(value: T) -> Lock<T> {
        Lock(Arc::new(LockState {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
            wakers: Mutex::new(Vec::new()),
        }))
    }

    /// Attempts to acquire the lock.
    fn poll_lock(&self, cx: &mut Context<'_>) -> Poll<LockGuard<T>> {
        // Try acquiring the lock.
        if self.0.locked.swap(true, Ordering::Acquire) {
            // Lock the list of wakers.
            let mut list = self.0.wakers.lock().unwrap();

            // Try acquiring the lock again.
            if self.0.locked.swap(true, Ordering::Acquire) {
                // If failed again, add the current task to the list and return.
                if list.iter().all(|w| !w.will_wake(cx.waker())) {
                    list.push(cx.waker().clone());
                }
                return Poll::Pending;
            }
        }

        // The lock was successfully acquired.
        Poll::Ready(LockGuard(self.0.clone()))
    }
}

/// A lock guard.
///
/// When dropped, ownership of the inner value is returned back to the lock.
#[derive(Debug)]
struct LockGuard<T>(Arc<LockState<T>>);

unsafe impl<T: Send> Send for LockGuard<T> {}
unsafe impl<T: Sync> Sync for LockGuard<T> {}

impl<T> LockGuard<T> {
    /// Registers a task interested in locking.
    ///
    /// When this lock guard gets dropped, all registered tasks will be woken up.
    fn register(&self, cx: &Context<'_>) {
        let mut list = self.0.wakers.lock().unwrap();

        if list.iter().all(|w| !w.will_wake(cx.waker())) {
            list.push(cx.waker().clone());
        }
    }
}

impl<T> Drop for LockGuard<T> {
    fn drop(&mut self) {
        self.0.locked.store(false, Ordering::Release);

        for w in self.0.wakers.lock().unwrap().drain(..) {
            w.wake();
        }
    }
}

impl<T> Deref for LockGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for LockGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}

/// Modes a file can be in.
///
/// The file can either be in idle mode, reading mode, or writing mode.
#[derive(Debug)]
enum Mode {
    /// The cache is empty.
    Idle,

    /// The cache contains data read from the inner file.
    ///
    /// This `usize` represents how many bytes from the beginning of cache have been consumed.
    Reading(usize),

    /// The cache contains data that needs to be written to the inner file.
    Writing,
}

/// The current state of a file.
///
/// The `File` struct puts this state behind a lock.
///
/// Filesystem operations that get spawned as blocking tasks will take ownership of the state and
/// return it back once the operation completes.
#[derive(Debug)]
struct State {
    /// The inner file.
    file: Arc<fs::File>,

    /// The current mode (idle, reading, or writing).
    mode: Mode,

    /// The read/write cache.
    ///
    /// If in reading mode, the cache contains a chunk of data that has been read from the file.
    /// If in writing mode, the cache contains data that will eventually be written into the file.
    cache: Vec<u8>,

    /// `true` if the file is flushed.
    ///
    /// When a file is flushed, the write cache and the inner file's buffer are empty.
    is_flushed: bool,

    /// The last read error that came from an async operation.
    last_read_err: Option<io::Error>,

    /// The last write error that came from an async operation.
    last_write_err: Option<io::Error>,
}

impl LockGuard<State> {
    /// Seeks to a new position in the file.
    fn poll_seek(mut self, cx: &mut Context<'_>, pos: SeekFrom) -> Poll<io::Result<u64>> {
        // If this operation doesn't move the cursor, then poll the current position inside the
        // file. This call will hopefully not block.
        if pos == SeekFrom::Current(0) {
            return Poll::Ready((&*self.file).seek(pos));
        }

        // Invalidate the read cache and flush the write cache before calling `seek()`.
        self = futures_core::ready!(self.poll_unread(cx))?;
        self = futures_core::ready!(self.poll_flush(cx))?;

        // Seek to the new position. This call is hopefully not blocking because it should just
        // change the internal offset into the file and not touch the actual file.
        Poll::Ready((&*self.file).seek(pos))
    }

    /// Reads some bytes from the file into a buffer.
    fn poll_read(mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
        // If an async operation has left a read error, return it now.
        if let Some(err) = self.last_read_err.take() {
            return Poll::Ready(Err(err));
        }

        match self.mode {
            Mode::Idle => {}
            Mode::Reading(start) => {
                // How many bytes in the cache are available for reading.
                let available = self.cache.len() - start;

                // If there is cached unconsumed data or if the cache is empty, we can read from
                // it. Empty cache in reading mode indicates that the last operation didn't read
                // any bytes, i.e. it reached the end of the file.
                if available > 0 || self.cache.is_empty() {
                    // Copy data from the cache into the buffer.
                    let n = cmp::min(available, buf.len());
                    buf[..n].copy_from_slice(&self.cache[start..n]);

                    // Move the read cursor forward.
                    self.mode = Mode::Reading(start + n);

                    return Poll::Ready(Ok(n));
                }
            }
            Mode::Writing => {
                // If we're in writing mode, flush the write cache.
                self = futures_core::ready!(self.poll_flush(cx))?;
            }
        }

        // Make the cache as long as `buf`.
        if self.cache.len() < buf.len() {
            let diff = buf.len() - self.cache.len();
            self.cache.reserve(diff);
        }
        unsafe {
            self.cache.set_len(buf.len());
        }

        // Register current task's interest in the file lock.
        self.register(cx);

        // Start a read operation asynchronously.
        blocking::spawn(async move {
            // Read some data from the file into the cache.
            let res = {
                let State { file, cache, .. } = &mut *self;
                (&**file).read(cache)
            };

            match res {
                Ok(n) => {
                    // Update cache length and switch to reading mode, starting from index 0.
                    unsafe {
                        self.cache.set_len(n);
                    }
                    self.mode = Mode::Reading(0);
                }
                Err(err) => {
                    // Save the error and switch to idle mode.
                    self.cache.clear();
                    self.mode = Mode::Idle;
                    self.last_read_err = Some(err);
                }
            }
        });

        Poll::Pending
    }

    /// Invalidates the read cache.
    ///
    /// This method will also move the internal file's cursor backwards by the number of unconsumed
    /// bytes in the read cache.
    fn poll_unread(mut self, _: &mut Context<'_>) -> Poll<io::Result<Self>> {
        match self.mode {
            Mode::Idle | Mode::Writing => Poll::Ready(Ok(self)),
            Mode::Reading(start) => {
                // Number of unconsumed bytes in the read cache.
                let n = self.cache.len() - start;

                if n > 0 {
                    // Seek `n` bytes backwards. This call is hopefully not blocking because it
                    // should just change the internal offset into the file and not touch the
                    // actual file.
                    (&*self.file).seek(SeekFrom::Current(-(n as i64)))?;
                }

                // Switch to idle mode.
                self.cache.clear();
                self.mode = Mode::Idle;

                Poll::Ready(Ok(self))
            }
        }
    }

    /// Writes some data from a buffer into the file.
    fn poll_write(mut self, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        // If an async operation has left a write error, return it now.
        if let Some(err) = self.last_write_err.take() {
            return Poll::Ready(Err(err));
        }

        // If we're in reading mode, invalidate the read buffer.
        self = futures_core::ready!(self.poll_unread(cx))?;

        // Make the cache have as much capacity as `buf`.
        if self.cache.capacity() < buf.len() {
            let diff = buf.len() - self.cache.capacity();
            self.cache.reserve(diff);
        }

        // How many bytes can be written into the cache before filling up.
        let available = self.cache.capacity() - self.cache.len();

        // If there is available space in the cache or if the buffer is empty, we can write data
        // into the cache.
        if available > 0 || buf.is_empty() {
            let n = cmp::min(available, buf.len());
            let start = self.cache.len();

            // Copy data from the buffer into the cache.
            unsafe {
                self.cache.set_len(start + n);
            }
            self.cache[start..start + n].copy_from_slice(&buf[..n]);

            // Mark the file as not flushed and switch to writing mode.
            self.is_flushed = false;
            self.mode = Mode::Writing;
            Poll::Ready(Ok(n))
        } else {
            // Drain the write cache because it's full.
            futures_core::ready!(self.poll_drain(cx))?;
            Poll::Pending
        }
    }

    /// Drains the write cache.
    fn poll_drain(mut self, cx: &mut Context<'_>) -> Poll<io::Result<Self>> {
        // If an async operation has left a write error, return it now.
        if let Some(err) = self.last_write_err.take() {
            return Poll::Ready(Err(err));
        }

        match self.mode {
            Mode::Idle | Mode::Reading(..) => Poll::Ready(Ok(self)),
            Mode::Writing => {
                // Register current task's interest in the file lock.
                self.register(cx);

                // Start a write operation asynchronously.
                blocking::spawn(async move {
                    match (&*self.file).write_all(&self.cache) {
                        Ok(_) => {
                            // Switch to idle mode.
                            self.cache.clear();
                            self.mode = Mode::Idle;
                        }
                        Err(err) => {
                            // Save the error.
                            self.last_write_err = Some(err);
                        }
                    };
                });

                Poll::Pending
            }
        }
    }

    /// Flushes the write cache into the file.
    fn poll_flush(mut self, cx: &mut Context<'_>) -> Poll<io::Result<Self>> {
        // If the file is already in flushed state, return.
        if self.is_flushed {
            return Poll::Ready(Ok(self));
        }

        // If there is data in the write cache, drain it.
        self = futures_core::ready!(self.poll_drain(cx))?;

        // Register current task's interest in the file lock.
        self.register(cx);

        // Start a flush operation asynchronously.
        blocking::spawn(async move {
            match (&*self.file).flush() {
                Ok(()) => {
                    // Mark the file as flushed.
                    self.is_flushed = true;
                }
                Err(err) => {
                    // Save the error.
                    self.last_write_err = Some(err);
                }
            }
        });

        Poll::Pending
    }

    // This function does nothing because we're not sure about `AsyncWrite::poll_close()`'s exact
    // semantics nor whether it will stay in the `AsyncWrite` trait.
    fn poll_close(self, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
