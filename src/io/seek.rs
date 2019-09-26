use std::pin::Pin;

use cfg_if::cfg_if;

use crate::future::Future;
use crate::io::{self, SeekFrom};
use crate::task::{Context, Poll};
use crate::utils::extension_trait;

cfg_if! {
    if #[cfg(feature = "docs")] {
        use std::ops::{Deref, DerefMut};
    }
}

extension_trait! {
    #[doc = r#"
        Allows seeking through a byte stream.

        This trait is a re-export of [`futures::io::AsyncSeek`] and is an async version of
        [`std::io::Seek`].

        The [provided methods] do not really exist in the trait itself, but they become
        available when the prelude is imported:

        ```
        # #[allow(unused_imports)]
        use async_std::prelude::*;
        ```

        [`std::io::Seek`]: https://doc.rust-lang.org/std/io/trait.Seek.html
        [`futures::io::AsyncSeek`]:
        https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncSeek.html
        [provided methods]: #provided-methods
    "#]
    pub trait Seek [SeekExt: futures_io::AsyncSeek] {
        #[doc = r#"
            Attempt to seek to an offset, in bytes, in a stream.
        "#]
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<io::Result<u64>>;

        #[doc = r#"
            Seeks to a new position in a byte stream.

            Returns the new position in the byte stream.

            A seek beyond the end of stream is allowed, but behavior is defined by the
            implementation.

            # Examples

            ```no_run
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::fs::File;
            use async_std::io::SeekFrom;
            use async_std::prelude::*;

            let mut file = File::open("a.txt").await?;

            let file_len = file.seek(SeekFrom::End(0)).await?;
            #
            # Ok(()) }) }
            ```
        "#]
        fn seek(
            &mut self,
            pos: SeekFrom,
        ) -> impl Future<Output = io::Result<u64>> [SeekFuture<'_, Self>]
        where
            Self: Unpin,
        {
            SeekFuture { seeker: self, pos }
        }
    }

    impl<T: Seek + Unpin + ?Sized> Seek for Box<T> {
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<io::Result<u64>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<T: Seek + Unpin + ?Sized> Seek for &mut T {
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<io::Result<u64>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<P> Seek for Pin<P>
    where
        P: DerefMut + Unpin,
        <P as Deref>::Target: Seek,
    {
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: SeekFrom,
        ) -> Poll<io::Result<u64>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct SeekFuture<'a, T: Unpin + ?Sized> {
    seeker: &'a mut T,
    pos: SeekFrom,
}

impl<T: SeekExt + Unpin + ?Sized> Future for SeekFuture<'_, T> {
    type Output = io::Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let pos = self.pos;
        Pin::new(&mut *self.seeker).poll_seek(cx, pos)
    }
}
