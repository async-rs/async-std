use std::pin::Pin;

use crate::future::Future;
use crate::io::{self, BufRead, BufReader, Read, Write};
use crate::task::{Context, Poll};

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
/// This function is an async version of [`std::io::copy`].
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
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::io;
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
    R: Read + Unpin + ?Sized,
    W: Write + Unpin + ?Sized,
{
    pub struct CopyFuture<'a, R, W: ?Sized> {
        reader: R,
        writer: &'a mut W,
        amt: u64,
    }

    impl<R, W: Unpin + ?Sized> CopyFuture<'_, R, W> {
        fn project(self: Pin<&mut Self>) -> (Pin<&mut R>, Pin<&mut W>, &mut u64) {
            unsafe {
                let this = self.get_unchecked_mut();
                (
                    Pin::new_unchecked(&mut this.reader),
                    Pin::new(&mut *this.writer),
                    &mut this.amt,
                )
            }
        }
    }

    impl<R, W> Future for CopyFuture<'_, R, W>
    where
        R: BufRead,
        W: Write + Unpin + ?Sized,
    {
        type Output = io::Result<u64>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let (mut reader, mut writer, amt) = self.project();
            loop {
                let buffer = futures_core::ready!(reader.as_mut().poll_fill_buf(cx))?;
                if buffer.is_empty() {
                    futures_core::ready!(writer.as_mut().poll_flush(cx))?;
                    return Poll::Ready(Ok(*amt));
                }

                let i = futures_core::ready!(writer.as_mut().poll_write(cx, buffer))?;
                if i == 0 {
                    return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
                }
                *amt += i as u64;
                reader.as_mut().consume(i);
            }
        }
    }

    let future = CopyFuture {
        reader: BufReader::new(reader),
        writer,
        amt: 0,
    };
    future.await
}
