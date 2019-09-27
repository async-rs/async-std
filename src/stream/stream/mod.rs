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
mod chain;
mod enumerate;
mod filter;
mod filter_map;
mod find;
mod find_map;
mod fold;
mod fuse;
mod inspect;
mod min_by;
mod next;
mod nth;
mod scan;
mod skip;
mod skip_while;
mod step_by;
mod take;
mod zip;

use all::AllFuture;
use any::AnyFuture;
use enumerate::Enumerate;
use filter_map::FilterMap;
use find::FindFuture;
use find_map::FindMapFuture;
use fold::FoldFuture;
use min_by::MinByFuture;
use next::NextFuture;
use nth::NthFuture;

pub use chain::Chain;
pub use filter::Filter;
pub use fuse::Fuse;
pub use inspect::Inspect;
pub use scan::Scan;
pub use skip::Skip;
pub use skip_while::SkipWhile;
pub use step_by::StepBy;
pub use take::Take;
pub use zip::Zip;

use std::cmp::Ordering;
use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::utils::extension_trait;

cfg_if! {
    if #[cfg(feature = "docs")] {
        use std::ops::{Deref, DerefMut};

        use crate::task::{Context, Poll};
    }
}

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        use std::pin::Pin;

        use crate::future::Future;
        use crate::stream::FromStream;
    }
}

extension_trait! {
    #[doc = r#"
        An asynchronous stream of values.

        This trait is a re-export of [`futures::stream::Stream`] and is an async version of
        [`std::iter::Iterator`].

        The [provided methods] do not really exist in the trait itself, but they become
        available when the prelude is imported:

        ```
        # #[allow(unused_imports)]
        use async_std::prelude::*;
        ```

        [`std::iter::Iterator`]: https://doc.rust-lang.org/std/iter/trait.Iterator.html
        [`futures::stream::Stream`]:
        https://docs.rs/futures-preview/0.3.0-alpha.17/futures/stream/trait.Stream.html
        [provided methods]: #provided-methods
    "#]
    pub trait Stream [StreamExt: futures_core::stream::Stream] {
        #[doc = r#"
            The type of items yielded by this stream.
        "#]
        type Item;

        #[doc = r#"
            Attempts to receive the next item from the stream.

            There are several possible return values:

            * `Poll::Pending` means this stream's next value is not ready yet.
            * `Poll::Ready(None)` means this stream has been exhausted.
            * `Poll::Ready(Some(item))` means `item` was received out of the stream.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::pin::Pin;

            use async_std::prelude::*;
            use async_std::stream;
            use async_std::task::{Context, Poll};

            fn increment(
                s: impl Stream<Item = i32> + Unpin,
            ) -> impl Stream<Item = i32> + Unpin {
                struct Increment<S>(S);

                impl<S: Stream<Item = i32> + Unpin> Stream for Increment<S> {
                    type Item = S::Item;

                    fn poll_next(
                        mut self: Pin<&mut Self>,
                        cx: &mut Context<'_>,
                    ) -> Poll<Option<Self::Item>> {
                        match Pin::new(&mut self.0).poll_next(cx) {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(None) => Poll::Ready(None),
                            Poll::Ready(Some(item)) => Poll::Ready(Some(item + 1)),
                        }
                    }
                }

                Increment(s)
            }

            let mut s = increment(stream::once(7));

            assert_eq!(s.next().await, Some(8));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;

        #[doc = r#"
            Advances the stream and returns the next value.

            Returns [`None`] when iteration is finished. Individual stream implementations may
            choose to resume iteration, and so calling `next()` again may or may not eventually
            start returning more values.

            [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::once(7);

            assert_eq!(s.next().await, Some(7));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        fn next(&mut self) -> impl Future<Output = Option<Self::Item>> + '_ [NextFuture<'_, Self>]
        where
            Self: Unpin,
        {
            NextFuture { stream: self }
        }

        #[doc = r#"
            Creates a stream that yields its first `n` elements.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::repeat(9).take(3);

            while let Some(v) = s.next().await {
                assert_eq!(v, 9);
            }
            #
            # }) }
            ```
        "#]
        fn take(self, n: usize) -> Take<Self>
        where
            Self: Sized,
        {
            Take {
                stream: self,
                remaining: n,
            }
        }

        #[doc = r#"
            Creates a stream that yields each `step`th element.

            # Panics

            This method will panic if the given step is `0`.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let s: VecDeque<_> = vec![0u8, 1, 2, 3, 4].into_iter().collect();
            let mut stepped = s.step_by(2);

            assert_eq!(stepped.next().await, Some(0));
            assert_eq!(stepped.next().await, Some(2));
            assert_eq!(stepped.next().await, Some(4));
            assert_eq!(stepped.next().await, None);

            #
            # }) }
            ```
        "#]
        fn step_by(self, step: usize) -> StepBy<Self>
        where
            Self: Sized,
        {
            StepBy::new(self, step)
        }

        #[doc = r#"
            Takes two streams and creates a new stream over both in sequence.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let first: VecDeque<_> = vec![0u8, 1].into_iter().collect();
            let second: VecDeque<_> = vec![2, 3].into_iter().collect();
            let mut c = first.chain(second);

            assert_eq!(c.next().await, Some(0));
            assert_eq!(c.next().await, Some(1));
            assert_eq!(c.next().await, Some(2));
            assert_eq!(c.next().await, Some(3));
            assert_eq!(c.next().await, None);

            #
            # }) }
            ```
        "#]
        fn chain<U>(self, other: U) -> Chain<Self, U>
        where
            Self: Sized,
            U: Stream<Item = Self::Item> + Sized,
        {
            Chain::new(self, other)
        }

        #[doc = r#"
            Creates a stream that gives the current element's count as well as the next value.

            # Overflow behaviour.

            This combinator does no guarding against overflows.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let s: VecDeque<_> = vec!['a', 'b', 'c'].into_iter().collect();
            let mut s = s.enumerate();

            assert_eq!(s.next().await, Some((0, 'a')));
            assert_eq!(s.next().await, Some((1, 'b')));
            assert_eq!(s.next().await, Some((2, 'c')));
            assert_eq!(s.next().await, None);

            #
            # }) }
            ```
        "#]
        fn enumerate(self) -> Enumerate<Self>
        where
            Self: Sized,
        {
            Enumerate::new(self)
        }

        #[doc = r#"
            A combinator that does something with each element in the stream, passing the value
            on.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let a: VecDeque<_> = vec![1u8, 2, 3, 4, 5].into_iter().collect();
            let sum = a
                    .inspect(|x| println!("about to filter {}", x))
                    .filter(|x| x % 2 == 0)
                    .inspect(|x| println!("made it through filter: {}", x))
                    .fold(0, |sum, i| sum + i).await;

            assert_eq!(sum, 6);
            #
            # }) }
            ```
        "#]
        fn inspect<F>(self, f: F) -> Inspect<Self, F, Self::Item>
        where
            Self: Sized,
            F: FnMut(&Self::Item),
        {
            Inspect::new(self, f)
        }

        #[doc = r#"
            Transforms this `Stream` into a "fused" `Stream` such that after the first time
            `poll` returns `Poll::Ready(None)`, all future calls to `poll` will also return
            `Poll::Ready(None)`.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::once(1).fuse();
            assert_eq!(s.next().await, Some(1));
            assert_eq!(s.next().await, None);
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        fn fuse(self) -> Fuse<Self>
        where
            Self: Sized,
        {
            Fuse {
                stream: self,
                done: false,
            }
        }

        #[doc = r#"
            Creates a stream that uses a predicate to determine if an element should be yielded.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let s: VecDeque<usize> = vec![1, 2, 3, 4].into_iter().collect();
            let mut s = s.filter(|i| i % 2 == 0);

            assert_eq!(s.next().await, Some(2));
            assert_eq!(s.next().await, Some(4));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        fn filter<P>(self, predicate: P) -> Filter<Self, P, Self::Item>
        where
            Self: Sized,
            P: FnMut(&Self::Item) -> bool,
        {
            Filter::new(self, predicate)
        }

        #[doc = r#"
            Both filters and maps a stream.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let s: VecDeque<&str> = vec!["1", "lol", "3", "NaN", "5"].into_iter().collect();

            let mut parsed = s.filter_map(|a| a.parse::<u32>().ok());

            let one = parsed.next().await;
            assert_eq!(one, Some(1));

            let three = parsed.next().await;
            assert_eq!(three, Some(3));

            let five = parsed.next().await;
            assert_eq!(five, Some(5));

            let end = parsed.next().await;
            assert_eq!(end, None);
            #
            # }) }
            ```
        "#]
        fn filter_map<B, F>(self, f: F) -> FilterMap<Self, F, Self::Item, B>
        where
            Self: Sized,
            F: FnMut(Self::Item) -> Option<B>,
        {
            FilterMap::new(self, f)
        }

        #[doc = r#"
            Returns the element that gives the minimum value with respect to the
            specified comparison function. If several elements are equally minimum,
            the first element is returned. If the stream is empty, `None` is returned.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();

            let min = s.clone().min_by(|x, y| x.cmp(y)).await;
            assert_eq!(min, Some(1));

            let min = s.min_by(|x, y| y.cmp(x)).await;
            assert_eq!(min, Some(3));

            let min = VecDeque::<usize>::new().min_by(|x, y| x.cmp(y)).await;
            assert_eq!(min, None);
            #
            # }) }
            ```
        "#]
        fn min_by<F>(
            self,
            compare: F,
        ) -> impl Future<Output = Option<Self::Item>> [MinByFuture<Self, F, Self::Item>]
        where
            Self: Sized,
            F: FnMut(&Self::Item, &Self::Item) -> Ordering,
        {
            MinByFuture::new(self, compare)
        }

        #[doc = r#"
            Returns the nth element of the stream.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let mut s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();

            let second = s.nth(1).await;
            assert_eq!(second, Some(2));
            #
            # }) }
            ```
            Calling `nth()` multiple times:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let mut s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();

            let second = s.nth(0).await;
            assert_eq!(second, Some(1));

            let second = s.nth(0).await;
            assert_eq!(second, Some(2));
            #
            # }) }
            ```
            Returning `None` if the stream finished before returning `n` elements:
            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let mut s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();

            let fourth = s.nth(4).await;
            assert_eq!(fourth, None);
            #
            # }) }
            ```
        "#]
        fn nth(
            &mut self,
            n: usize,
        ) -> impl Future<Output = Option<Self::Item>> + '_ [NthFuture<'_, Self>]
        where
            Self: Sized,
        {
            NthFuture::new(self, n)
        }

        #[doc = r#"
            Tests if every element of the stream matches a predicate.

            `all()` takes a closure that returns `true` or `false`. It applies
            this closure to each element of the stream, and if they all return
            `true`, then so does `all()`. If any of them return `false`, it
            returns `false`.

            `all()` is short-circuiting; in other words, it will stop processing
            as soon as it finds a `false`, given that no matter what else happens,
            the result will also be `false`.

            An empty stream returns `true`.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::repeat::<u32>(42).take(3);
            assert!(s.all(|x| x ==  42).await);

            #
            # }) }
            ```

            Empty stream:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::empty::<u32>();
            assert!(s.all(|_| false).await);
            #
            # }) }
            ```
        "#]
        #[inline]
        fn all<F>(
            &mut self,
            f: F,
        ) -> impl Future<Output = bool> + '_ [AllFuture<'_, Self, F, Self::Item>]
        where
            Self: Unpin + Sized,
            F: FnMut(Self::Item) -> bool,
        {
            AllFuture {
                stream: self,
                result: true, // the default if the empty stream
                _marker: PhantomData,
                f,
            }
        }

        #[doc = r#"
            Searches for an element in a stream that satisfies a predicate.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let mut s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();
            let res = s.find(|x| *x == 2).await;
            assert_eq!(res, Some(2));
            #
            # }) }
            ```

            Resuming after a first find:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let mut s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();
            let res = s.find(|x| *x == 2).await;
            assert_eq!(res, Some(2));

            let next = s.next().await;
            assert_eq!(next, Some(3));
            #
            # }) }
            ```
        "#]
        fn find<P>(
            &mut self,
            p: P,
        ) -> impl Future<Output = Option<Self::Item>> + '_ [FindFuture<'_, Self, P, Self::Item>]
        where
            Self: Sized,
            P: FnMut(&Self::Item) -> bool,
        {
            FindFuture::new(self, p)
        }

        #[doc = r#"
            Applies function to the elements of stream and returns the first non-none result.

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let mut s: VecDeque<&str> = vec!["lol", "NaN", "2", "5"].into_iter().collect();
            let first_number = s.find_map(|s| s.parse().ok()).await;

            assert_eq!(first_number, Some(2));
            #
            # }) }
            ```
        "#]
        fn find_map<F, B>(
            &mut self,
            f: F,
        ) -> impl Future<Output = Option<B>> + '_ [FindMapFuture<'_, Self, F, Self::Item, B>]
        where
            Self: Sized,
            F: FnMut(Self::Item) -> Option<B>,
        {
            FindMapFuture::new(self, f)
        }

        #[doc = r#"
            A combinator that applies a function to every element in a stream
            producing a single, final value.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use std::collections::VecDeque;

            let s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();
            let sum = s.fold(0, |acc, x| acc + x).await;

            assert_eq!(sum, 6);
            #
            # }) }
            ```
        "#]
        fn fold<B, F>(
            self,
            init: B,
            f: F,
        ) -> impl Future<Output = B> [FoldFuture<Self, F, Self::Item, B>]
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            FoldFuture::new(self, init, f)
        }

        #[doc = r#"
            Tests if any element of the stream matches a predicate.

            `any()` takes a closure that returns `true` or `false`. It applies
            this closure to each element of the stream, and if any of them return
            `true`, then so does `any()`. If they all return `false`, it
            returns `false`.

            `any()` is short-circuiting; in other words, it will stop processing
            as soon as it finds a `true`, given that no matter what else happens,
            the result will also be `true`.

            An empty stream returns `false`.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::repeat::<u32>(42).take(3);
            assert!(s.any(|x| x ==  42).await);
            #
            # }) }
            ```

            Empty stream:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let mut s = stream::empty::<u32>();
            assert!(!s.any(|_| false).await);
            #
            # }) }
            ```
        "#]
        #[inline]
        fn any<F>(
            &mut self,
            f: F,
        ) -> impl Future<Output = bool> + '_ [AnyFuture<'_, Self, F, Self::Item>]
        where
            Self: Unpin + Sized,
            F: FnMut(Self::Item) -> bool,
        {
            AnyFuture {
                stream: self,
                result: false, // the default if the empty stream
                _marker: PhantomData,
                f,
            }
        }

        #[doc = r#"
            A stream adaptor similar to [`fold`] that holds internal state and produces a new
            stream.

            [`fold`]: #method.fold

            `scan()` takes two arguments: an initial value which seeds the internal state, and
            a closure with two arguments, the first being a mutable reference to the internal
            state and the second a stream element. The closure can assign to the internal state
            to share state between iterations.

            On iteration, the closure will be applied to each element of the stream and the
            return value from the closure, an `Option`, is yielded by the stream.

            ## Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let s: VecDeque<isize> = vec![1, 2, 3].into_iter().collect();
            let mut s = s.scan(1, |state, x| {
                *state = *state * x;
                Some(-*state)
            });

            assert_eq!(s.next().await, Some(-1));
            assert_eq!(s.next().await, Some(-2));
            assert_eq!(s.next().await, Some(-6));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        #[inline]
        fn scan<St, B, F>(self, initial_state: St, f: F) -> Scan<Self, St, F>
        where
            Self: Sized,
            F: FnMut(&mut St, Self::Item) -> Option<B>,
        {
            Scan::new(self, initial_state, f)
        }

        #[doc = r#"
            Combinator that `skip`s elements based on a predicate.

            Takes a closure argument. It will call this closure on every element in
            the stream and ignore elements until it returns `false`.

            After `false` is returned, `SkipWhile`'s job is over and all further
            elements in the strem are yielded.

            ## Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let a: VecDeque<_> = vec![-1i32, 0, 1].into_iter().collect();
            let mut s = a.skip_while(|x| x.is_negative());

            assert_eq!(s.next().await, Some(0));
            assert_eq!(s.next().await, Some(1));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        fn skip_while<P>(self, predicate: P) -> SkipWhile<Self, P, Self::Item>
        where
            Self: Sized,
            P: FnMut(&Self::Item) -> bool,
        {
            SkipWhile::new(self, predicate)
        }

        #[doc = r#"
            Creates a combinator that skips the first `n` elements.

            ## Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let s: VecDeque<usize> = vec![1, 2, 3].into_iter().collect();
            let mut skipped = s.skip(2);

            assert_eq!(skipped.next().await, Some(3));
            assert_eq!(skipped.next().await, None);
            #
            # }) }
            ```
        "#]
        fn skip(self, n: usize) -> Skip<Self>
        where
            Self: Sized,
        {
            Skip::new(self, n)
        }

        #[doc = r#"
            'Zips up' two streams into a single stream of pairs.

            `zip()` returns a new stream that will iterate over two other streams, returning a
            tuple where the first element comes from the first stream, and the second element
            comes from the second stream.

            In other words, it zips two streams together, into a single one.

            If either stream returns [`None`], [`poll_next`] from the zipped stream will return
            [`None`]. If the first stream returns [`None`], `zip` will short-circuit and
            `poll_next` will not be called on the second stream.

            [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
            [`poll_next`]: #tymethod.poll_next

            ## Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use std::collections::VecDeque;

            use async_std::prelude::*;

            let l: VecDeque<isize> = vec![1, 2, 3].into_iter().collect();
            let r: VecDeque<isize> = vec![4, 5, 6, 7].into_iter().collect();
            let mut s = l.zip(r);

            assert_eq!(s.next().await, Some((1, 4)));
            assert_eq!(s.next().await, Some((2, 5)));
            assert_eq!(s.next().await, Some((3, 6)));
            assert_eq!(s.next().await, None);
            #
            # }) }
            ```
        "#]
        #[inline]
        fn zip<U>(self, other: U) -> Zip<Self, U>
        where
            Self: Sized + Stream,
            U: Stream,
        {
            Zip::new(self, other)
        }

        #[doc = r#"
            Transforms a stream into a collection.

            `collect()` can take anything streamable, and turn it into a relevant
            collection. This is one of the more powerful methods in the async
            standard library, used in a variety of contexts.

            The most basic pattern in which `collect()` is used is to turn one
            collection into another. You take a collection, call [`stream`] on it,
            do a bunch of transformations, and then `collect()` at the end.

            Because `collect()` is so general, it can cause problems with type
            inference. As such, `collect()` is one of the few times you'll see
            the syntax affectionately known as the 'turbofish': `::<>`. This
            helps the inference algorithm understand specifically which collection
            you're trying to collect into.

            # Examples

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::stream;

            let s = stream::repeat(9u8).take(3);
            let buf: Vec<u8> = s.collect().await;

            assert_eq!(buf, vec![9; 3]);

            // You can also collect streams of Result values
            // into any collection that implements FromStream
            let s = stream::repeat(Ok(9)).take(3);
            // We are using Vec here, but other collections
            // are supported as well
            let buf: Result<Vec<u8>, ()> = s.collect().await;

            assert_eq!(buf, Ok(vec![9; 3]));

            // The stream will stop on the first Err and
            // return that instead
            let s = stream::repeat(Err(5)).take(3);
            let buf: Result<Vec<u8>, u8> = s.collect().await;

            assert_eq!(buf, Err(5));
            #
            # }) }
            ```

            [`stream`]: trait.Stream.html#tymethod.next
        "#]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        #[must_use = "if you really need to exhaust the iterator, consider `.for_each(drop)` instead (TODO)"]
        fn collect<'a, B>(
            self,
        ) -> impl Future<Output = B> + 'a [Pin<Box<dyn Future<Output = B> + 'a>>]
        where
            Self: Sized + 'a,
            B: FromStream<Self::Item>,
        {
            FromStream::from_stream(self)
        }
    }

    impl<S: Stream + Unpin + ?Sized> Stream for Box<S> {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<S: Stream + Unpin + ?Sized> Stream for &mut S {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<P> Stream for Pin<P>
    where
        P: DerefMut + Unpin,
        <P as Deref>::Target: Stream,
    {
        type Item = <<P as Deref>::Target as Stream>::Item;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<T: Unpin> Stream for std::collections::VecDeque<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<S: Stream> Stream for std::panic::AssertUnwindSafe<S> {
        type Item = S::Item;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }
}
