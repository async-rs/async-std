use pin_project_lite::pin_project;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::stream::map::Map;
use crate::stream::{IntoStream, Stream};
use crate::task::{Context, Poll};

pin_project! {
    /// This `struct` is created by the [`flat_map`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`flat_map`]: trait.Stream.html#method.flat_map
    /// [`Stream`]: trait.Stream.html
    #[allow(missing_debug_implementations)]
    pub struct FlatMap<S, U, T, F> {
        #[pin]
        stream: Map<S, F, T, U>,
        #[pin]
        inner_stream: Option<U>,
    }
}

impl<S, U, F> FlatMap<S, U, S::Item, F>
where
    S: Stream,
    U: IntoStream,
    F: FnMut(S::Item) -> U,
{
    pub(super) fn new(stream: S, f: F) -> FlatMap<S, U, S::Item, F> {
        FlatMap {
            stream: stream.map(f),
            inner_stream: None,
        }
    }
}

impl<S, U, F> Stream for FlatMap<S, U, S::Item, F>
where
    S: Stream,
    S::Item: IntoStream<IntoStream = U, Item = U::Item>,
    U: Stream,
    F: FnMut(S::Item) -> U,
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
