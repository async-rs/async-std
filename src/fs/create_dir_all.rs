use std::fs;
use std::path::Path;

use crate::task::blocking;
use crate::io;

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
