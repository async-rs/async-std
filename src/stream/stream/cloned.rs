use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Cloned<S> {
    stream: S,
}

impl<S> Cloned<S> {
    pub fn new(stream: S) -> Cloned<S> {
        Cloned { stream }
    }
}

impl<S, T> Stream for Cloned<S>
where
    S: Stream<Item = T> + Unpin,
    T: Clone,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.stream).poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(item)) => Poll::Ready(Some(item.clone())),
        }
    }
}
