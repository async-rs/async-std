use pin_project_lite::pin_project;
use std::pin::Pin;

use crate::stream::{IntoStream, Stream};
use crate::task::{Context, Poll};

pin_project! {
    /// This `struct` is created by the [`flatten`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`flatten`]: trait.Stream.html#method.flatten
    /// [`Stream`]: trait.Stream.html
    #[allow(missing_debug_implementations)]
    pub struct Flatten<S, U> {
        #[pin]
        stream: S,
        #[pin]
        inner_stream: Option<U>,
    }
}

impl<S> Flatten<S, S::Item>
where
    S: Stream,
    S::Item: IntoStream,
{
    pub(super) fn new(stream: S) -> Flatten<S, S::Item> {
        Flatten {
            stream,
            inner_stream: None,
        }
    }
}

impl<S, U> Stream for Flatten<S, <S::Item as IntoStream>::IntoStream>
where
    S: Stream,
    S::Item: IntoStream<IntoStream = U, Item = U::Item>,
    U: Stream,
{
    type Item = U::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(inner) = this.inner_stream.as_mut().as_pin_mut() {
                if let item @ Some(_) = futures_core::ready!(inner.poll_next(cx)) {
                    return Poll::Ready(item);
                }
            }

            match futures_core::ready!(this.stream.as_mut().poll_next(cx)) {
                None => return Poll::Ready(None),
                Some(inner) => this.inner_stream.set(Some(inner.into_stream())),
            }
        }
    }
}
