use std::fs;
use std::future::Future;
use std::io;
use std::path::Path;

use cfg_if::cfg_if;

use super::File;
use crate::task::blocking;

/// Options and flags which for configuring how a file is opened.
///
/// This builder exposes the ability to configure how a [`File`] is opened and what operations are
/// permitted on the open file. The [`File::open`] and [`File::create`] methods are aliases for
/// commonly used options with this builder.
///
/// Generally speaking, when using `OpenOptions`, you'll first call [`new`], then chain calls to
/// methods to set each option, then call [`open`], passing the path of the file you're trying to
/// open. This will give you a [`File`] inside that you can further operate on.
///
/// This type is an async version of [`std::fs::OpenOptions`].
///
/// [`new`]: struct.OpenOptions.html#method.new
/// [`open`]: struct.OpenOptions.html#method.open
/// [`File`]: struct.File.html
/// [`File::open`]: struct.File.html#method.open
/// [`File::create`]: struct.File.html#method.create
/// [`std::fs::OpenOptions`]: https://doc.rust-lang.org/std/fs/struct.OpenOptions.html
///
/// # Examples
///
/// Opening a file for reading:
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs::OpenOptions;
///
/// let file = OpenOptions::new()
///     .read(true)
///     .open("foo.txt")
///     .await?;
/// #
/// # Ok(()) }) }
/// ```
///
/// Opening a file for both reading and writing, creating it if it doesn't exist:
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs::OpenOptions;
///
/// let file = OpenOptions::new()
///     .read(true)
///     .write(true)
///     .create(true)
///     .open("foo.txt")
///     .await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Clone, Debug)]
pub struct OpenOptions(fs::OpenOptions);

impl OpenOptions {
    /// Creates a blank new set of options.
    ///
    /// All options are initially set to `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn new() -> OpenOptions {
        OpenOptions(fs::OpenOptions::new())
    }

    /// Sets the option for read access.
    ///
    /// This option, when `true`, will indicate that the file should be readable if opened.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .read(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn read(&mut self, read: bool) -> &mut OpenOptions {
        self.0.read(read);
        self
    }

    /// Sets the option for write access.
    ///
    /// This option, when `true`, will indicate that the file should be writable if opened.
    ///
    /// If the file already exists, any write calls on it will overwrite its contents, without
    /// truncating it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn write(&mut self, write: bool) -> &mut OpenOptions {
        self.0.write(write);
        self
    }

    /// Sets the option for append mode.
    ///
    /// This option, when `true`, means that writes will append to a file instead of overwriting
    /// previous contents. Note that setting `.write(true).append(true)` has the same effect as
    /// setting only `.append(true)`.
    ///
    /// For most filesystems, the operating system guarantees that all writes are atomic: no writes
    /// get mangled because another process writes at the same time.
    ///
    /// One maybe obvious note when using append mode: make sure that all data that belongs
    /// together is written to the file in one operation. This can be done by concatenating strings
    /// before writing them, or using a buffered writer (with a buffer of adequate size), and
    /// flushing when the message is complete.
    ///
    /// If a file is opened with both read and append access, beware that after opening and after
    /// every write, the position for reading may be set at the end of the file. So, before
    /// writing, save the current position by seeking with a zero offset, and restore it before the
    /// next read.
    ///
    /// ## Note
    ///
    /// This function doesn't create the file if it doesn't exist. Use the [`create`] method to do
    /// so.
    ///
    /// [`create`]: #method.create
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .append(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn append(&mut self, append: bool) -> &mut OpenOptions {
        self.0.append(append);
        self
    }

    /// Sets the option for truncating a previous file.
    ///
    /// If a file is successfully opened with this option set, it will truncate the file to 0
    /// length if it already exists.
    ///
    /// The file must be opened with write access for truncation to work.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .truncate(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn truncate(&mut self, truncate: bool) -> &mut OpenOptions {
        self.0.truncate(truncate);
        self
    }

    /// Sets the option for creating a new file.
    ///
    /// This option indicates whether a new file will be created if the file does not yet exist.
    ///
    /// In order for the file to be created, [`write`] or [`append`] access must be used.
    ///
    /// [`write`]: #method.write
    /// [`append`]: #method.append
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .create(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn create(&mut self, create: bool) -> &mut OpenOptions {
        self.0.create(create);
        self
    }

    /// Sets the option to always create a new file.
    ///
    /// This option indicates whether a new file will be created. No file is allowed to exist at
    /// the target location, also no (dangling) symlink.
    ///
    /// This option is useful because it is atomic. Otherwise, between checking whether a file
    /// exists and creating a new one, the file may have been created by another process (a TOCTOU
    /// race condition / attack).
    ///
    /// If `.create_new(true)` is set, [`.create()`] and [`.truncate()`] are ignored.
    ///
    /// The file must be opened with write or append access in order to create a new file.
    ///
    /// [`.create()`]: #method.create
    /// [`.truncate()`]: #method.truncate
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new()
    ///     .write(true)
    ///     .create_new(true)
    ///     .open("foo.txt")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn create_new(&mut self, create_new: bool) -> &mut OpenOptions {
        self.0.create_new(create_new);
        self
    }

    /// Opens a file at specified path with the configured options.
    ///
    /// # Errors
    ///
    /// This function will return an error under a number of different circumstances. Some of these
    /// error conditions are listed here, together with their [`ErrorKind`]. The mapping to
    /// [`ErrorKind`]s is not part of the compatibility contract of the function, especially the
    /// `Other` kind might change to more specific kinds in the future.
    ///
    /// * [`NotFound`]: The specified file does not exist and neither `create` or `create_new` is
    ///   set.
    /// * [`NotFound`]: One of the directory components of the file path does not exist.
    /// * [`PermissionDenied`]: The user lacks permission to get the specified access rights for
    ///   the file.
    /// * [`PermissionDenied`]: The user lacks permission to open one of the directory components
    ///   of the specified path.
    /// * [`AlreadyExists`]: `create_new` was specified and the file already exists.
    /// * [`InvalidInput`]: Invalid combinations of open options (truncate without write access, no
    ///   access mode set, etc.).
    /// * [`Other`]: One of the directory components of the specified file path was not, in fact, a
    ///   directory.
    /// * [`Other`]: Filesystem-level errors: full disk, write permission requested on a read-only
    ///   file system, exceeded disk quota, too many open files, too long filename, too many
    ///   symbolic links in the specified path (Unix-like systems only), etc.
    ///
    /// [`ErrorKind`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html
    /// [`AlreadyExists`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.AlreadyExists
    /// [`InvalidInput`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.InvalidInput
    /// [`NotFound`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.NotFound
    /// [`Other`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.Other
    /// [`PermissionDenied`]: https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.PermissionDenied
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::OpenOptions;
    ///
    /// let file = OpenOptions::new().open("foo.txt").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn open<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<File>> {
        let path = path.as_ref().to_owned();
        let options = self.0.clone();
        async move { blocking::spawn(async move { options.open(path).map(|f| f.into()) }).await }
    }
}

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
        use crate::os::unix::fs::OpenOptionsExt;
    } else if #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
    }
}

#[cfg_attr(feature = "docs.rs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs.rs"))] {
        impl OpenOptionsExt for OpenOptions {
            fn mode(&mut self, mode: u32) -> &mut Self {
                self.0.mode(mode);
                self
            }

            fn custom_flags(&mut self, flags: i32) -> &mut Self {
                self.0.custom_flags(flags);
                self
            }
        }
    }
}
