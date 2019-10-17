use std::pin::Pin;

use crate::stream::{IntoStream, Stream};
use crate::task::{Context, Poll};

/// Real logic of both `Flatten` and `FlatMap` which simply delegate to
/// this type.
#[derive(Clone, Debug)]
struct FlattenCompat<S, U> {
    stream: S,
    frontiter: Option<U>,
}
impl<S, U> FlattenCompat<S, U> {
    pin_utils::unsafe_unpinned!(stream: S);
    pin_utils::unsafe_unpinned!(frontiter: Option<U>);

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

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let Some(ref mut inner) = self.as_mut().frontiter() {
                if let item @ Some(_) = futures_core::ready!(Pin::new(inner).poll_next(cx)) {
                    return Poll::Ready(item);
                }
            }

            match futures_core::ready!(Pin::new(&mut self.stream).poll_next(cx)) {
                None => return Poll::Ready(None),
                Some(inner) => *self.as_mut().frontiter() = Some(inner.into_stream()),
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
