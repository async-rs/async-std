use std::fs;
use std::path::{Path, PathBuf};

use crate::io;
use crate::task::blocking;

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
