use std::fs;
use std::path::{Path, PathBuf};

use crate::io;
use crate::task::blocking;

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
