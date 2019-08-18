//! Asynchronous iteration.
//!
//! This module is an async version of [`std::iter`].
//!
//! [`std::iter`]: https://doc.rust-lang.org/std/iter/index.html
//!
//! # Examples
//!
//! ```
//! # #![feature(async_await)]
//! # fn main() { async_std::task::block_on(async {
//! #
//! use async_std::prelude::*;
//! use async_std::stream;
//!
//! let mut s = stream::repeat(9).take(3);
//!
//! while let Some(v) = s.next().await {
//!     assert_eq!(v, 9);
//! }
//! #
//! # }) }
//! ```

use std::pin::Pin;

use cfg_if::cfg_if;

use crate::future::Future;
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

/// An asynchronous stream of values.
///
/// This trait is an async version of [`std::iter::Iterator`].
///
/// While it is currently not possible to implement this trait directly, it gets implemented
/// automatically for all types that implement [`futures::stream::Stream`].
///
/// [`std::iter::Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
/// [`futures::stream::Stream`]:
/// https://docs.rs/futures-preview/0.3.0-alpha.17/futures/stream/trait.Stream.html
pub trait Stream {
    /// The type of items yielded by this stream.
    type Item;

    /// Advances the stream and returns the next value.
    ///
    /// Returns [`None`] when iteration is finished. Individual stream implementations may
    /// choose to resume iteration, and so calling `next()` again may or may not eventually
    /// start returning more values.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::once(7);
    ///
    /// assert_eq!(s.next().await, Some(7));
    /// assert_eq!(s.next().await, None);
    /// #
    /// # }) }
    /// ```
    fn next<'a>(&'a mut self) -> ret!('a, NextFuture, Option<Self::Item>)
    where
        Self: Unpin;

    /// Creates a stream that yields its first `n` elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::repeat(9).take(3);
    ///
    /// while let Some(v) = s.next().await {
    ///     assert_eq!(v, 9);
    /// }
    /// #
    /// # }) }
    /// ```
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take {
            stream: self,
            remaining: n,
        }
    }
}

impl<T: futures::Stream + Unpin + ?Sized> Stream for T {
    type Item = <Self as futures::Stream>::Item;

    fn next<'a>(&'a mut self) -> ret!('a, NextFuture, Option<Self::Item>)
    where
        Self: Unpin,
    {
        NextFuture { stream: self }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct NextFuture<'a, T: Unpin + ?Sized> {
    stream: &'a mut T,
}

impl<T: futures::Stream + Unpin + ?Sized> Future for NextFuture<'_, T> {
    type Output = Option<T::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}

/// A stream that yields the first `n` items of another stream.
#[derive(Clone, Debug)]
pub struct Take<S> {
    stream: S,
    remaining: usize,
}

impl<S: Unpin> Unpin for Take<S> {}

impl<S: futures::Stream> Take<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(remaining: usize);
}

impl<S: futures::Stream> futures::Stream for Take<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if self.remaining == 0 {
            Poll::Ready(None)
        } else {
            let next = futures::ready!(self.as_mut().stream().poll_next(cx));
            match next {
                Some(_) => *self.as_mut().remaining() -= 1,
                None => *self.as_mut().remaining() = 0,
            }
            Poll::Ready(next)
        }
    }
}
