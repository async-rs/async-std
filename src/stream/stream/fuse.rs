use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A `Stream` that is permanently closed once a single call to `poll` results in
    /// `Poll::Ready(None)`, returning `Poll::Ready(None)` for all future calls to `poll`.
    #[derive(Clone, Debug)]
    pub struct Fuse<S> {
        #[pin]
        pub(crate) stream: S,
        pub(crate) done: bool,
    }
}

impl<S: Stream> Stream for Fuse<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let this = self.project();
        if *this.done {
            Poll::Ready(None)
        } else {
            let next = futures_core::ready!(this.stream.poll_next(cx));
            if next.is_none() {
                *this.done = true;
            }
            Poll::Ready(next)
        }
    }
}
