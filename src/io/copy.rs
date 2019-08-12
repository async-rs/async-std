use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::io;

/// Copies the entire contents of a reader into a writer.
///
/// This function will continuously read data from `reader` and then
/// write it into `writer` in a streaming fashion until `reader`
/// returns EOF.
///
/// On success, the total number of bytes that were copied from
/// `reader` to `writer` is returned.
///
/// If you’re wanting to copy the contents of one file to another and you’re
/// working with filesystem paths, see the [`fs::copy`] function.
///
/// This function is an async version of [`std::fs::write`].
///
/// [`std::io::copy`]: https://doc.rust-lang.org/std/io/fn.copy.html
/// [`fs::copy`]: ../fs/fn.copy.html
///
/// # Errors
///
/// This function will return an error immediately if any call to `read` or
/// `write` returns an error. All instances of `ErrorKind::Interrupted` are
/// handled by this function and the underlying operation is retried.
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::{io, task};
///
/// let mut reader: &[u8] = b"hello";
/// let mut writer = io::stdout();
///
/// io::copy(&mut reader, &mut writer).await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn copy<R, W>(reader: &mut R, writer: &mut W) -> io::Result<u64>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    let bytes_read = reader.copy_into(writer).await?;
    Ok(bytes_read)
}
