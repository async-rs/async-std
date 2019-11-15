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
    }
}

