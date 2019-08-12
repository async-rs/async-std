use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Mutex;

use cfg_if::cfg_if;
use futures::future::{self, FutureExt, TryFutureExt};

use crate::future::Future;
use crate::io;
use crate::task::{blocking, Poll};

/// An entry inside a directory.
///
/// An instance of `DirEntry` represents an entry inside a directory on the filesystem. Each entry
/// carriers additional information like the full path or metadata.
///
/// This type is an async version of [`std::fs::DirEntry`].
///
/// [`std::fs::DirEntry`]: https://doc.rust-lang.org/std/fs/struct.DirEntry.html
#[derive(Debug)]
pub struct DirEntry {
    /// The state of the entry.
    state: Mutex<State>,

    /// The full path to the entry.
    path: PathBuf,

    #[cfg(unix)]
    ino: u64,

    /// The bare name of the entry without the leading path.
    file_name: OsString,
}

/// The state of an asynchronous `DirEntry`.
///
/// The `DirEntry` can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    Idle(Option<fs::DirEntry>),
    Busy(blocking::JoinHandle<State>),
}

impl DirEntry {
    /// Creates an asynchronous `DirEntry` from a synchronous handle.
    pub(crate) fn new(inner: fs::DirEntry) -> DirEntry {
        #[cfg(unix)]
        let dir_entry = DirEntry {
            path: inner.path(),
            file_name: inner.file_name(),
            ino: inner.ino(),
            state: Mutex::new(State::Idle(Some(inner))),
        };

        #[cfg(windows)]
        let dir_entry = DirEntry {
            path: inner.path(),
            file_name: inner.file_name(),
            state: Mutex::new(State::Idle(Some(inner))),
        };

        dir_entry
    }

    /// Returns the full path to this entry.
    ///
    /// The full path is created by joining the original path passed to [`read_dir`] with the name
    /// of this entry.
    ///
    /// [`read_dir`]: fn.read_dir.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs, prelude::*};
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(entry) = dir.next().await {
    ///     let entry = entry?;
    ///     println!("{:?}", entry.path());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    /// Returns the metadata for this entry.
    ///
    /// This function will not traverse symlinks if this entry points at a symlink.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs, prelude::*};
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(entry) = dir.next().await {
    ///     let entry = entry?;
    ///     println!("{:?}", entry.metadata().await?);
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn metadata(&self) -> io::Result<fs::Metadata> {
        future::poll_fn(|cx| {
            let state = &mut *self.state.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.metadata();
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
        .map(|opt| opt.ok_or_else(|| io_error("invalid state")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Returns the file type for this entry.
    ///
    /// This function will not traverse symlinks if this entry points at a symlink.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs, prelude::*};
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(entry) = dir.next().await {
    ///     let entry = entry?;
    ///     println!("{:?}", entry.file_type().await?);
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn file_type(&self) -> io::Result<fs::FileType> {
        future::poll_fn(|cx| {
            let state = &mut *self.state.lock().unwrap();

            loop {
                match state {
                    State::Idle(opt) => match opt.take() {
                        None => return Poll::Ready(None),
                        Some(inner) => {
                            let (s, r) = futures::channel::oneshot::channel();

                            // Start the operation asynchronously.
                            *state = State::Busy(blocking::spawn(async move {
                                let res = inner.file_type();
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
        .map(|opt| opt.ok_or_else(|| io_error("invalid state")))
        .await?
        .map_err(|_| io_error("blocking task failed"))
        .await?
    }

    /// Returns the bare name of this entry without the leading path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs, prelude::*};
    ///
    /// let mut dir = fs::read_dir(".").await?;
    ///
    /// while let Some(entry) = dir.next().await {
    ///     let entry = entry?;
    ///     println!("{:?}", entry.file_name());
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn file_name(&self) -> OsString {
        self.file_name.clone()
    }
}

/// Creates a custom `io::Error` with an arbitrary error type.
fn io_error(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
        use crate::os::unix::fs::DirEntryExt;
    } else if #[cfg(unix)] {
        use std::os::unix::fs::DirEntryExt;
    }
}

#[cfg_attr(feature = "docs.rs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs.rs"))] {
        impl DirEntryExt for DirEntry {
            fn ino(&self) -> u64 {
                self.ino
            }
        }
    }
}
