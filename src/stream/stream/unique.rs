use std::pin::Pin;
use std::hash::Hash;
use std::collections::HashSet;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that filters out duplicate items.
    ///
    /// Duplicates are detected using hash and equality. Produced values are
    /// stored in a hash set in the stream.
    ///
    /// This `struct` is created by the [`unique`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`unique`]: trait.Stream.html#method.unique
    /// [`Stream`]: trait.Stream.html
    #[cfg(feature="unstable")]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    pub struct Unique<S: Stream> {
        #[pin]
        stream: S,
        used: HashSet<S::Item>
    }
}

impl<S> Unique<S>
where
    S: Stream,
    S::Item: Eq + Hash
{
    pub(crate) fn new(stream: S) -> Self {
        Self {
            stream,
            used: HashSet::new()
        }
    }
}

impl<S> Stream for Unique<S>
where
    S: Stream,
    S::Item: Eq + Hash + Clone
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        while let Some(v) = futures_core::ready!(this.stream.as_mut().poll_next(cx)) {
            if this.used.insert(v.clone()) {
                return Poll::Ready(Some(v));
            }
        }
        
        Poll::Ready(None)
    }
}
