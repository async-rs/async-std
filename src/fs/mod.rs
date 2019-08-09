//! Filesystem manipulation operations.
//!
//! This module is an async version of [`std::fs`].
//!
//! [`std::fs`]: https://doc.rust-lang.org/std/fs/index.html
//!
//! # Examples
//!
//! Create a new file and write some bytes to it:
//!
//! ```no_run
//! # #![feature(async_await)]
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::fs::File;
//! use async_std::prelude::*;
//!
//! let mut file = File::create("foo.txt").await?;
//! file.write_all(b"Hello, world!").await?;
//! #
//! # Ok(()) }) }
//! ```

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::task::blocking;

pub use dir_builder::DirBuilder;
pub use dir_entry::DirEntry;
pub use file::File;
pub use open_options::OpenOptions;
pub use read_dir::ReadDir;

mod dir_builder;
mod dir_entry;
mod file;
mod open_options;
mod read_dir;

#[doc(inline)]
pub use std::fs::{FileType, Metadata, Permissions};

/// Returns the canonical form of a path.
///
/// The returned path is in absolute form with all intermediate components normalized and symbolic
/// links resolved.
///
/// This function is an async version of [`std::fs::canonicalize`].
///
/// [`std::fs::canonicalize`]: https://doc.rust-lang.org/std/fs/fn.canonicalize.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * A non-final component in path is not a directory.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let path = fs::canonicalize(".").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::canonicalize(path) }).await
}

/// Creates a new, empty directory.
///
/// This function is an async version of [`std::fs::create_dir`].
///
/// [`std::fs::create_dir`]: https://doc.rust-lang.org/std/fs/fn.create_dir.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` already exists.
/// * A parent of the given path does not exist.
/// * The current process lacks permissions to create directory at `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::create_dir("./some/dir").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::create_dir(path) }).await
}

/// Creates a new, empty directory and all of its parents if they are missing.
///
/// This function is an async version of [`std::fs::create_dir_all`].
///
/// [`std::fs::create_dir_all`]: https://doc.rust-lang.org/std/fs/fn.create_dir_all.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The parent directories do not exists and couldn't be created.
/// * The current process lacks permissions to create directory at `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::create_dir_all("./some/dir").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::create_dir_all(path) }).await
}

/// Creates a new hard link on the filesystem.
///
/// The `dst` path will be a link pointing to the `src` path. Note that systems often require these
/// two paths to both be located on the same filesystem.
///
/// This function is an async version of [`std::fs::hard_link`].
///
/// [`std::fs::hard_link`]: https://doc.rust-lang.org/std/fs/fn.hard_link.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The `src` path is not a file or doesn't exist.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::hard_link("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    blocking::spawn(async move { fs::hard_link(&from, &to) }).await
}

/// Copies the contents and permissions of one file to another.
///
/// On success, the total number of bytes copied is returned and equals the length of the `from`
/// file.
///
/// The old contents of `to` will be overwritten. If `from` and `to` both point to the same file,
/// then the file will likely get truncated by this operation.
///
/// This function is an async version of [`std::fs::copy`].
///
/// [`std::fs::copy`]: https://doc.rust-lang.org/std/fs/fn.copy.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The `from` path is not a file.
/// * The `from` file does not exist.
/// * The current process lacks permissions to access `from` or write `to`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let bytes_copied = fs::copy("foo.txt", "bar.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    blocking::spawn(async move { fs::copy(&from, &to) }).await
}

/// Queries the metadata for a path.
///
/// This function will traverse symbolic links to query information about the file or directory.
///
/// This function is an async version of [`std::fs::metadata`].
///
/// [`std::fs::metadata`]: https://doc.rust-lang.org/std/fs/fn.metadata.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * The current process lacks permissions to query metadata for `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let perm = fs::metadata("foo.txt").await?.permissions();
/// #
/// # Ok(()) }) }
/// ```
pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::metadata(path) }).await
}

/// Read the entire contents of a file into a bytes vector.
///
/// This is a convenience function for reading entire files. It pre-allocates a buffer based on the
/// file size when available, so it is generally faster than manually opening a file and reading
/// into a `Vec`.
///
/// This function is an async version of [`std::fs::read`].
///
/// [`std::fs::read`]: https://doc.rust-lang.org/std/fs/fn.read.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * The current process lacks permissions to read `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let contents = fs::read("foo.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::read(path) }).await
}

/// Returns a stream over the entries within a directory.
///
/// The stream yields items of type [`io::Result`]`<`[`DirEntry`]`>`. New errors may be encountered
/// after a stream is initially constructed.
///
/// This function is an async version of [`std::fs::read_dir`].
///
/// [`io::Result`]: https://doc.rust-lang.org/std/io/type.Result.html
/// [`DirEntry`]: struct.DirEntry.html
/// [`std::fs::read_dir`]: https://doc.rust-lang.org/std/fs/fn.read_dir.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * `path` does not point at a directory.
/// * The current process lacks permissions to view the contents of `path`.
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
pub async fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<ReadDir> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::read_dir(path) })
        .await
        .map(ReadDir::new)
}

/// Reads a symbolic link, returning the path it points to.
///
/// This function is an async version of [`std::fs::read_link`].
///
/// [`std::fs::read_link`]: https://doc.rust-lang.org/std/fs/fn.read_link.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` is not a symbolic link.
/// * `path` does not exist.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let path = fs::read_link("foo.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn read_link<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::read_link(path) }).await
}

/// Read the entire contents of a file into a string.
///
/// This function is an async version of [`std::fs::read_to_string`].
///
/// [`std::fs::read_to_string`]: https://doc.rust-lang.org/std/fs/fn.read_to_string.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` is not a file.
/// * The current process lacks permissions to read `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// use async_std::fs::read_to_string;
///
/// # futures::executor::block_on(async {
/// let contents = read_to_string("foo.txt").await?;
/// # std::io::Result::Ok(())
/// # }).unwrap();
/// ```
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::read_to_string(path) }).await
}

/// Removes an existing, empty directory.
///
/// This function is an async version of [`std::fs::remove_dir`].
///
/// [`std::fs::remove_dir`]: https://doc.rust-lang.org/std/fs/fn.remove_dir.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` is not an empty directory.
/// * The current process lacks permissions to remove directory at `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::remove_dir("./some/dir").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::remove_dir(path) }).await
}

/// Removes an directory and all of its contents.
///
/// This function is an async version of [`std::fs::remove_dir_all`].
///
/// [`std::fs::remove_dir_all`]: https://doc.rust-lang.org/std/fs/fn.remove_dir_all.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` is not a directory.
/// * The current process lacks permissions to remove directory at `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::remove_dir_all("./some/dir").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::remove_dir_all(path) }).await
}

/// Removes a file from the filesystem.
///
/// This function is an async version of [`std::fs::remove_file`].
///
/// [`std::fs::remove_file`]: https://doc.rust-lang.org/std/fs/fn.remove_file.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` is not a file.
/// * The current process lacks permissions to remove file at `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::remove_file("foo.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::remove_file(path) }).await
}

/// Renames a file or directory to a new name, replacing the original if it already exists.
///
/// This function is an async version of [`std::fs::rename`].
///
/// [`std::fs::rename`]: https://doc.rust-lang.org/std/fs/fn.rename.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `from` does not exist.
/// * `from` and `to` are on different filesystems.
/// * The current process lacks permissions to rename `from` to `to`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::rename("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    blocking::spawn(async move { fs::rename(&from, &to) }).await
}

/// Changes the permissions on a file or directory.
///
/// This function is an async version of [`std::fs::set_permissions`].
///
/// [`std::fs::set_permissions`]: https://doc.rust-lang.org/std/fs/fn.set_permissions.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * The current process lacks permissions to change attributes of `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let mut perm = fs::metadata("foo.txt").await?.permissions();
/// perm.set_readonly(true);
///
/// fs::set_permissions("foo.txt", perm).await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn set_permissions<P: AsRef<Path>>(path: P, perm: fs::Permissions) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::set_permissions(path, perm) }).await
}

/// Queries the metadata for a path without following symlinks.
///
/// This function is an async version of [`std::fs::symlink_metadata`].
///
/// [`std::fs::symlink_metadata`]: https://doc.rust-lang.org/std/fs/fn.symlink_metadata.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * The current process lacks permissions to query metadata for `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let perm = fs::symlink_metadata("foo.txt").await?.permissions();
/// #
/// # Ok(()) }) }
/// ```
pub async fn symlink_metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::symlink_metadata(path) }).await
}

/// Writes a slice of bytes as the entire contents of a file.
///
/// This function will create a file if it does not exist, and will entirely replace its contents
/// if it does.
///
/// This function is an async version of [`std::fs::write`].
///
/// [`std::fs::write`]: https://doc.rust-lang.org/std/fs/fn.write.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The current process lacks permissions to write into `path`.
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::write("foo.txt", b"Lorem ipsum").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    let contents = contents.as_ref().to_owned();
    blocking::spawn(async move { fs::write(path, contents) }).await
}
