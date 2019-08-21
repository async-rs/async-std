use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
