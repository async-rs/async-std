use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

/// Writes a slice of bytes as the entire contents of a file.
///
/// This function will create a file if it does not exist, and will entirely replace its contents
/// if it does.
///
/// This function is an async version of [`std::fs::write`].
///
/// [`std::fs::write`]: https://doc.rust-lang.org/std/fs/fn.write.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * The current process lacks permissions to write into `path`.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// fs::write("a.txt", b"Lorem ipsum").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    let contents = contents.as_ref().to_owned();
    blocking::spawn(async move { fs::write(path, contents) }).await
}
