use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

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
