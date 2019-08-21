use std::fs;
use std::path::Path;

use crate::io;
use crate::task::blocking;

/// Changes the permissions on a file or directory.
///
/// This function is an async version of [`std::fs::set_permissions`].
///
/// [`std::fs::set_permissions`]: https://doc.rust-lang.org/std/fs/fn.set_permissions.html
///
/// # Errors
///
/// An error will be returned in the following situations (not an exhaustive list):
///
/// * `path` does not exist.
/// * The current process lacks permissions to change attributes of `path`.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::fs;
///
/// let mut perm = fs::metadata("a.txt").await?.permissions();
/// perm.set_readonly(true);
/// fs::set_permissions("a.txt", perm).await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn set_permissions<P: AsRef<Path>>(path: P, perm: fs::Permissions) -> io::Result<()> {
    let path = path.as_ref().to_owned();
    blocking::spawn(async move { fs::set_permissions(path, perm) }).await
}
