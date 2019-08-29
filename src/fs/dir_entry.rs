use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use cfg_if::cfg_if;

use crate::io;
use crate::task::blocking;

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
    /// The inner synchronous `DirEntry`.
    inner: Arc<fs::DirEntry>,

    #[cfg(unix)]
    ino: u64,
}

impl DirEntry {
    /// Creates an asynchronous `DirEntry` from a synchronous handle.
    pub(crate) fn new(inner: fs::DirEntry) -> DirEntry {
        #[cfg(unix)]
        let dir_entry = DirEntry {
            ino: inner.ino(),
            inner: Arc::new(inner),
        };

        #[cfg(windows)]
        let dir_entry = DirEntry {
            inner: Arc::new(inner),
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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs;
    /// use async_std::prelude::*;
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
        self.inner.path()
    }

    /// Returns the metadata for this entry.
    ///
    /// This function will not traverse symlinks if this entry points at a symlink.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs;
    /// use async_std::prelude::*;
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
        let inner = self.inner.clone();
        blocking::spawn(async move { inner.metadata() }).await
    }

    /// Returns the file type for this entry.
    ///
    /// This function will not traverse symlinks if this entry points at a symlink.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs;
    /// use async_std::prelude::*;
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
        let inner = self.inner.clone();
        blocking::spawn(async move { inner.file_type() }).await
    }

    /// Returns the bare name of this entry without the leading path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs;
    /// use async_std::prelude::*;
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
        self.inner.file_name()
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        use crate::os::unix::fs::DirEntryExt;
    } else if #[cfg(unix)] {
        use std::os::unix::fs::DirEntryExt;
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs"))] {
        impl DirEntryExt for DirEntry {
            fn ino(&self) -> u64 {
                self.ino
            }
        }
    }
}
