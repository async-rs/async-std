use std::fs::{self, Metadata};
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
