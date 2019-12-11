use std::pin::Pin;
use std::hash::Hash;
use std::collections::HashSet;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that filters out duplicate items.
    ///
    /// Duplicates are detected by comparing the key they map to with the keying
    /// function `f` by hash and equality. The keys are stored in a hash set in
    /// the stream.
    ///
    /// This `struct` is created by the [`unique`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`unique`]: trait.Stream.html#method.unique
    /// [`Stream`]: trait.Stream.html
    #[cfg(feature="unstable")]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    pub struct UniqueBy<S, V, F> {
        #[pin]
        stream: S,
        used: HashSet<V>,
        f: F,
    }
}

impl<S, V, F> UniqueBy<S, V, F>
where
    V: Eq + Hash
{
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream,
            used: HashSet::new(),
            f
        }
    }
}

impl<S, V, F> Stream for UniqueBy<S, V, F>
where
    S: Stream,
    V: Eq + Hash,
    F: FnMut(&S::Item) -> V,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        while let Some(v) = futures_core::ready!(this.stream.as_mut().poll_next(cx)) {
            let key = (this.f)(&v);
            if this.used.insert(key) {
                return Poll::Ready(Some(v));
            }
        }
        
        Poll::Ready(None)
    }
}
