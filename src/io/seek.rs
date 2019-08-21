use std::pin::Pin;

use cfg_if::cfg_if;
use futures::io::AsyncSeek;

use crate::future::Future;
use crate::io::{self, SeekFrom};
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(feature = "docs")] {
        #[doc(hidden)]
        pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);

        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => (ImplFuture<$a, $o>);
        }
    } else {
        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => ($f<$a, Self>);
        }
    }
}

/// Allows seeking through a byte stream.
///
/// This trait is an async version of [`std::io::Seek`].
///
/// While it is currently not possible to implement this trait directly, it gets implemented
/// automatically for all types that implement [`futures::io::AsyncSeek`].
///
/// [`std::io::Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
/// [`futures::io::AsyncSeek`]:
/// https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncSeek.html
pub trait Seek {
    /// Seeks to a new position in a byte stream.
    ///
    /// Returns the new position in the byte stream.
    ///
    /// A seek beyond the end of stream is allowed, but behavior is defined by the
    /// implementation.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::io::SeekFrom;
    /// use async_std::prelude::*;
    ///
    /// let mut f = File::open("a.txt").await?;
    ///
    /// let file_len = f.seek(SeekFrom::End(0)).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn seek(&mut self, pos: SeekFrom) -> ret!('_, SeekFuture, io::Result<u64>)
    where
        Self: Unpin;
}

impl<T: AsyncSeek + Unpin + ?Sized> Seek for T {
    fn seek(&mut self, pos: SeekFrom) -> ret!('_, SeekFuture, io::Result<u64>) {
        SeekFuture { seeker: self, pos }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct SeekFuture<'a, T: Unpin + ?Sized> {
    seeker: &'a mut T,
    pos: SeekFrom,
}

impl<T: AsyncSeek + Unpin + ?Sized> Future for SeekFuture<'_, T> {
    type Output = io::Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pos = self.pos;
        Pin::new(&mut *self.seeker).poll_seek(cx, pos)
    }
}
