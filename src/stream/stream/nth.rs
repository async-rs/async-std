use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct NthFuture<'a, S> {
    stream: &'a mut S,
    n: usize,
}

impl<'a, S> NthFuture<'a, S> {
    pin_utils::unsafe_pinned!(stream: &'a mut S);
    pin_utils::unsafe_unpinned!(n: usize);

    pub(crate) fn new(stream: &'a mut S, n: usize) -> Self {
        NthFuture { stream, n }
    }
}

impl<'a, S> futures_core::future::Future for NthFuture<'a, S>
where
    S: futures_core::stream::Stream + Unpin + Sized,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use futures_core::stream::Stream;

        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
        match next {
            Some(v) => match self.n {
                0 => Poll::Ready(Some(v)),
                _ => {
                    *self.as_mut().n() -= 1;
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            None => Poll::Ready(None),
        }
    }
}
