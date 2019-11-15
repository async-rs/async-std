mod nth_back;
mod rfind;
mod rfold;

use nth_back::NthBackFuture;
use rfind::RFindFuture;
use rfold::RFoldFuture;

extension_trait! {
    use crate::stream::Stream;

    use std::pin::Pin;
    use std::task::{Context, Poll};


    #[doc = r#"
    Something fancy
    "#]
    pub trait DoubleEndedStream {
        type Item;

        fn poll_next_back(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
    }

    #[doc = r#"
        Something else
    "#]
    pub trait DoubleEndedStreamExt: crate::stream::DoubleEndedStream {

        #[doc = r#"
            Returns the nth element from the back of the stream.

            # Examples

            Basic usage:

            ```
            # fn main() { async_std::task::block_on(async {
            #
            use async_std::stream::double_ended::DoubleEndedStreamExt;
            use async_std::stream;

            let mut s = stream::from_iter(vec![1u8, 2, 3, 4, 5]);

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
    }
}
