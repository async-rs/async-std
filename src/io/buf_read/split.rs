use std::mem;
use std::pin::Pin;

use super::read_until_internal;
use crate::io::{self, BufRead};
use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream over the contents of an instance of [`BufRead`] split on a particular byte.
///
/// This stream is created by the [`split`] method on types that implement [`BufRead`].
///
/// This type is an async version of [`std::io::Split`].
///
/// [`split`]: trait.BufRead.html#method.lines
/// [`BufRead`]: trait.BufRead.html
/// [`std::io::Split`]: https://doc.rust-lang.org/std/io/struct.Split.html
#[derive(Debug)]
pub struct Split<R> {
    pub(crate) reader: R,
    pub(crate) buf: Vec<u8>,
    pub(crate) read: usize,
    pub(crate) delim: u8,
}

impl<R: BufRead> Stream for Split<R> {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            reader,
            buf,
            read,
            delim,
        } = unsafe { self.get_unchecked_mut() };
        let reader = unsafe { Pin::new_unchecked(reader) };
        let n = futures_core::ready!(read_until_internal(reader, cx, *delim, buf, read))?;
        if n == 0 && buf.is_empty() {
            return Poll::Ready(None);
        }
        if buf[buf.len() - 1] == *delim {
            buf.pop();
        }
        Poll::Ready(Some(Ok(mem::replace(buf, vec![]))))
    }
}
