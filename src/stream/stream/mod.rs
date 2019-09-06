//! Asynchronous iteration.
//!
//! This module is an async version of [`std::iter`].
//!
//! [`std::iter`]: https://doc.rust-lang.org/std/iter/index.html
//!
//! # Examples
//!
//! ```
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

mod all;
mod any;
mod min_by;
mod next;
mod take;

pub use take::Take;

use all::AllFuture;
use any::AnyFuture;
use min_by::MinByFuture;
use next::NextFuture;

use std::cmp::Ordering;
use std::marker::PhantomData;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "docs")] {
        #[doc(hidden)]
        pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);

        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => (ImplFuture<$a, $o>);
            ($a:lifetime, $f:tt, $o:ty, $t1:ty) => (ImplFuture<$a, $o>);
            ($a:lifetime, $f:tt, $o:ty, $t1:ty, $t2:ty) => (ImplFuture<$a, $o>);

        }
    } else {
        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => ($f<$a, Self>);
            ($a:lifetime, $f:tt, $o:ty, $t1:ty) => ($f<$a, Self, $t1>);
            ($a:lifetime, $f:tt, $o:ty, $t1:ty, $t2:ty) => ($f<$a, Self, $t1, $t2>);
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
    fn next(&mut self) -> ret!('_, NextFuture, Option<Self::Item>)
    where
        Self: Unpin;

    /// Creates a stream that yields its first `n` elements.
    ///
    /// # Examples
    ///
    /// ```
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

    /// Returns the element that gives the minimum value with respect to the
    /// specified comparison function. If several elements are equally minimum,
    /// the first element is returned. If the stream is empty, `None` is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use std::collections::VecDeque;
    /// use async_std::stream::Stream;
    ///
    /// let s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();
    ///
    /// let min = Stream::min_by(s.clone(), |x, y| x.cmp(y)).await;
    /// assert_eq!(min, Some(1));
    ///
    /// let min = Stream::min_by(s, |x, y| y.cmp(x)).await;
    /// assert_eq!(min, Some(3));
    ///
    /// let min = Stream::min_by(VecDeque::<usize>::new(), |x, y| x.cmp(y)).await;
    /// assert_eq!(min, None);
    /// #
    /// # }) }
    /// ```
    fn min_by<F>(self, compare: F) -> MinByFuture<Self, F>
    where
        Self: Sized + Unpin,
        F: FnMut(&Self::Item, &Self::Item) -> Ordering,
    {
        MinByFuture::new(self, compare)
    }

    /// Tests if every element of the stream matches a predicate.
    ///
    /// `all()` takes a closure that returns `true` or `false`. It applies
    /// this closure to each element of the stream, and if they all return
    /// `true`, then so does `all()`. If any of them return `false`, it
    /// returns `false`.
    ///
    /// `all()` is short-circuiting; in other words, it will stop processing
    /// as soon as it finds a `false`, given that no matter what else happens,
    /// the result will also be `false`.
    ///
    /// An empty stream returns `true`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::repeat::<u32>(42).take(3);
    /// assert!(s.all(|x| x ==  42).await);
    ///
    /// #
    /// # }) }
    /// ```
    ///
    /// Empty stream:
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::empty::<u32>();
    /// assert!(s.all(|_| false).await);
    /// #
    /// # }) }
    /// ```
    #[inline]
    fn all<F>(&mut self, f: F) -> ret!('_, AllFuture, bool, F, Self::Item)
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        AllFuture {
            stream: self,
            result: true, // the default if the empty stream
            __item: PhantomData,
            f,
        }
    }

    /// Tests if any element of the stream matches a predicate.
    ///
    /// `any()` takes a closure that returns `true` or `false`. It applies
    /// this closure to each element of the stream, and if any of them return
    /// `true`, then so does `any()`. If they all return `false`, it
    /// returns `false`.
    ///
    /// `any()` is short-circuiting; in other words, it will stop processing
    /// as soon as it finds a `true`, given that no matter what else happens,
    /// the result will also be `true`.
    ///
    /// An empty stream returns `false`.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::repeat::<u32>(42).take(3);
    /// assert!(s.any(|x| x ==  42).await);
    ///
    /// #
    /// # }) }
    /// ```
    ///
    /// Empty stream:
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let mut s = stream::empty::<u32>();
    /// assert!(!s.any(|_| false).await);
    /// #
    /// # }) }
    /// ```
    #[inline]
    fn any<F>(&mut self, f: F) -> ret!('_, AnyFuture, bool, F, Self::Item)
    where
        Self: Sized,
        F: FnMut(Self::Item) -> bool,
    {
        AnyFuture {
            stream: self,
            result: false, // the default if the empty stream
            __item: PhantomData,
            f,
        }
    }
}

impl<T: futures_core::stream::Stream + Unpin + ?Sized> Stream for T {
    type Item = <Self as futures_core::stream::Stream>::Item;

    fn next(&mut self) -> ret!('_, NextFuture, Option<Self::Item>)
    where
        Self: Unpin,
    {
        NextFuture { stream: self }
    }
}
