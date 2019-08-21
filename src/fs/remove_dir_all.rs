use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
