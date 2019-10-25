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
        inner: FlattenCompat<Map<S, F, T, U>, U>,
    }
}

impl<S, U, F> FlatMap<S, U, S::Item, F>
where
    S: Stream,
    U: IntoStream,
    F: FnMut(S::Item) -> U,
{
    pub fn new(stream: S, f: F) -> FlatMap<S, U, S::Item, F> {
        FlatMap {
            inner: FlattenCompat::new(stream.map(f)),
        }
    }
}

impl<S, U, F> Stream for FlatMap<S, U, S::Item, F>
where
    S: Stream<Item: IntoStream<IntoStream = U, Item = U::Item>> + std::marker::Unpin,
    U: Stream + std::marker::Unpin,
    F: FnMut(S::Item) -> U,
{
    type Item = U::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

pin_project! {
    /// This `struct` is created by the [`flatten`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`flatten`]: trait.Stream.html#method.flatten
    /// [`Stream`]: trait.Stream.html
    #[allow(missing_debug_implementations)]
    pub struct Flatten<S, U> {
        #[pin]
        inner: FlattenCompat<S, U>
    }
}

impl<S: Stream<Item: IntoStream>> Flatten<S, S::Item> {
    pub fn new(stream: S) -> Flatten<S, S::Item> {
        Flatten {
            inner: FlattenCompat::new(stream),
        }
    }
}

impl<S, U> Stream for Flatten<S, <S::Item as IntoStream>::IntoStream>
where
    S: Stream<Item: IntoStream<IntoStream = U, Item = U::Item>> + std::marker::Unpin,
    U: Stream + std::marker::Unpin,
{
    type Item = U::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

pin_project! {
    /// Real logic of both `Flatten` and `FlatMap` which simply delegate to
    /// this type.
    #[derive(Clone, Debug)]
    struct FlattenCompat<S, U> {
        stream: S,
        frontiter: Option<U>,
    }
}

impl<S, U> FlattenCompat<S, U> {
    /// Adapts an iterator by flattening it, for use in `flatten()` and `flat_map()`.
    pub fn new(stream: S) -> FlattenCompat<S, U> {
        FlattenCompat {
            stream,
            frontiter: None,
        }
    }
}

impl<S, U> Stream for FlattenCompat<S, U>
where
    S: Stream<Item: IntoStream<IntoStream = U, Item = U::Item>> + std::marker::Unpin,
    U: Stream + std::marker::Unpin,
{
    type Item = U::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(inner) = this.frontiter {
                if let item @ Some(_) = futures_core::ready!(Pin::new(inner).poll_next(cx)) {
                    return Poll::Ready(item);
                }
            }

            match futures_core::ready!(Pin::new(&mut this.stream).poll_next(cx)) {
                None => return Poll::Ready(None),
                Some(inner) => *this.frontiter = Some(inner.into_stream()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FlattenCompat;

    use crate::prelude::*;
    use crate::task;

    use std::collections::VecDeque;

    #[test]
    fn test_poll_next() -> std::io::Result<()> {
        let inner1: VecDeque<u8> = vec![1, 2, 3].into_iter().collect();
        let inner2: VecDeque<u8> = vec![4, 5, 6].into_iter().collect();

        let s: VecDeque<_> = vec![inner1, inner2].into_iter().collect();

        task::block_on(async move {
            let flat = FlattenCompat::new(s);
            let v: Vec<u8> = flat.collect().await;

            assert_eq!(v, vec![1, 2, 3, 4, 5, 6]);
            Ok(())
        })
    }
}
