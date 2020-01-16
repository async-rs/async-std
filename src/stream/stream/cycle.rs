use core::mem::ManuallyDrop;
use core::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that will repeatedly yield the same list of elements.
#[derive(Debug)]
pub struct Cycle<S> {
    orig: S,
    source: ManuallyDrop<S>,
}

impl<S> Cycle<S>
where
    S: Stream + Clone,
{
    pub(crate) fn new(source: S) -> Self {
        Self {
            orig: source.clone(),
            source: ManuallyDrop::new(source),
        }
    }
}

impl<S> Drop for Cycle<S> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.source);
        }
    }
}

impl<S> Stream for Cycle<S>
where
    S: Stream + Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        unsafe {
            let this = self.get_unchecked_mut();

            match futures_core::ready!(Pin::new_unchecked(&mut *this.source).poll_next(cx)) {
                Some(item) => Poll::Ready(Some(item)),
                None => {
                    ManuallyDrop::drop(&mut this.source);
                    this.source = ManuallyDrop::new(this.orig.clone());
                    Pin::new_unchecked(&mut *this.source).poll_next(cx)
                }
            }
        }
    }
}
