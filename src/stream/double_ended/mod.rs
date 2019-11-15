mod nth_back;

use nth_back::NthBackFuture;

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
    }
}
