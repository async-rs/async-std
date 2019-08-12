use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
/// fs::remove_file("a.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::remove_file(path) }).await
}
