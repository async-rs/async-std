mod next_back;
mod nth_back;
mod rfind;
mod rfold;
mod try_rfold;

use next_back::NextBackFuture;
use nth_back::NthBackFuture;
use rfind::RFindFuture;
use rfold::RFoldFuture;
use try_rfold::TryRFoldFuture;

extension_trait! {
    use crate::stream::Stream;

    use std::pin::Pin;
    use std::task::{Context, Poll};


    #[doc = r#"
        A stream able to yield elements from both ends.

        Something that implements `DoubleEndedStream` has one extra capability
        over something that implements [`Stream`]: the ability to also take
        `Item`s from the back, as well as the front.

        It is important to note that both back and forth work on the same range,
        and do not cross: iteration is over when they meet in the middle.

        In a similar fashion to the [`Stream`] protocol, once a
        `DoubleEndedStream` returns `None` from a `next_back()`, calling it again
        may or may not ever return `Some` again. `next()` and `next_back()` are
        interchangeable for this purpose.
        ```
    "#]
    pub trait DoubleEndedStream {
        #[doc = r#"
            The type of items yielded by this stream.
        "#]
        type Item;

        #[doc = r#"
            Attempts to receive the next item from the back of the stream.

            There are several possible return values:

            * `Poll::Pending` means this stream's next_back value is not ready yet.
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
                s: impl DoubleEndedStream<Item = i32> + Unpin,
            ) -> impl DoubleEndedStream<Item = i32> + Unpin {
                struct Increment<S>(S);

                impl<S: DoubleEndedStream<Item = i32> + Unpin> Stream for Increment<S> {
                    type Item = S::Item;

                    fn poll_next_back(
                        mut self: Pin<&mut Self>,
                        cx: &mut Context<'_>,
                    ) -> Poll<Option<Self::Item>> {
                        match Pin::new(&mut self.0).poll_next_back(cx) {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(None) => Poll::Ready(None),
                            Poll::Ready(Some(item)) => Poll::Ready(Some(item + 1)),
                        }
                    }
                }

                Increment(s)
            }

            let mut s = increment(stream::once(7)); // will need to implement DoubleEndedStream

            assert_eq!(s.next_back().await, Some(8));
            assert_eq!(s.next_back().await, None);
            #
            # }) }
            ```
        "#]
        fn poll_next_back(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
    }

    #[doc = r#"
        Extension methods for [`DoubleEndedStreamExt`].

        [`Stream`]: ../stream/trait.Stream.html
    "#]
    pub trait DoubleEndedStreamExt: crate::stream::DoubleEndedStream {
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
            use async_std::stream::Sample;
            use async_std::stream::double_ended::DoubleEndedStreamExt;

            let mut s = Sample::from(vec![7u8]);

            assert_eq!(s.next_back().await, Some(7));
            assert_eq!(s.next_back().await, None);
            #
            # }) }
            ```
        "#]
        fn next_back(&mut self) -> impl Future<Output = Option<Self::Item>> + '_ [NextBackFuture<'_, Self>]
            where
                Self: Unpin,
        {
            NextBackFuture { stream: self }
        }

        #[doc = r#"
            Returns the nth element from the back of the stream.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::stream::Sample;
            use async_std::stream::double_ended::DoubleEndedStreamExt;

            let mut s = Sample::from(vec![1u8, 2, 3, 4, 5]);

            let second = s.nth_back(1).await;
            assert_eq!(second, Some(4));
            #
            # }) }
            ```
        "#]
        fn nth_back(
            &mut self,
            n: usize,
        ) -> impl Future<Output = Option<Self::Item>> + '_ [NthBackFuture<'_, Self>]
        where
            Self: Unpin + Sized,
        {
            NthBackFuture::new(self, n)
        }

        #[doc = r#"
            Returns the the frist element from the right that matches the predicate.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::stream::Sample;
            use async_std::stream::double_ended::DoubleEndedStreamExt;

            let mut s = Sample::from(vec![1u8, 2, 3, 4, 5]);

            let second = s.rfind(|v| v % 2 == 0).await;
            assert_eq!(second, Some(4));
            #
            # }) }
            ```
        "#]
        fn rfind<P>(
            &mut self,
            p: P,
        ) -> impl Future<Output = Option<Self::Item>> + '_ [RFindFuture<'_, Self, P>]
        where
            Self: Unpin + Sized,
            P: FnMut(&Self::Item) -> bool,
        {
            RFindFuture::new(self, p)
        }

        #[doc = r#"
            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::stream::Sample;
            use async_std::stream::double_ended::DoubleEndedStreamExt;

            let s = Sample::from(vec![1, 2, 3, 4, 5]);

            let second = s.rfold(0, |acc, v| v + acc).await;

            assert_eq!(second, 15);
            #
            # }) }
            ```
        "#]
        fn rfold<B, F>(
            self,
            accum: B,
            f: F,
        ) -> impl Future<Output = Option<B>> [RFoldFuture<Self, F, B>]
            where
                Self: Sized,
                F: FnMut(B, Self::Item) -> B,
        {
            RFoldFuture::new(self, accum, f)
        }

        #[doc = r#"
            A combinator that applies a function as long as it returns successfully, producing a single, final value.
            Immediately returns the error when the function returns unsuccessfully.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::stream::Sample;
            use async_std::stream::double_ended::DoubleEndedStreamExt;

            let s = Sample::from(vec![1, 2, 3, 4, 5]);
            let sum = s.try_rfold(0, |acc, v| {
                if (acc+v) % 2 == 1 {
                    Ok(v+3)
                } else {
                    Err("fail")
                }
            }).await;

            assert_eq!(sum, Err("fail"));
            #
            # }) }
            ```
        "#]
        fn try_rfold<B, F, E>(
            self,
            accum: B,
            f: F,
        ) -> impl Future<Output = Option<B>> [TryRFoldFuture<Self, F, B>]
            where
                Self: Sized,
                F: FnMut(B, Self::Item) -> Result<B, E>,
       {
           TryRFoldFuture::new(self, accum, f)
       }

    }
}
