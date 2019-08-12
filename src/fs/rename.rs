use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
