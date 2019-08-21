use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

/// Creates a new hard link on the filesystem.
///
/// The `dst` path will be a link pointing to the `src` path. Note that systems often require these
/// two paths to both be located on the same filesystem.
///
/// This function is an async version of [`std::fs::hard_link`].
///
/// [`std::fs::hard_link`]: https://doc.rust-lang.org/std/fs/fn.hard_link.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The `src` path is not a file or doesn't exist.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::hard_link("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn hard_link<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let from = from.as_ref().to_owned();
    let to = to.as_ref().to_owned();
    blocking::spawn(async move { fs::hard_link(&from, &to) }).await
}
