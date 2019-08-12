use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
/// let bytes_copied = fs::copy("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    blocking::spawn(async move { fs::copy(&from, &to) }).await
}
