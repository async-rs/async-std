use core::marker::PhantomData;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that maps value of another stream with a function.
    #[derive(Debug)]
    pub struct Map<S, F, T, Fut, B> {
        #[pin]
        stream: S,
        f: F,
		#[pin]
		future: Option<Fut>,
		__from: PhantomData<T>,
		__to: PhantomData<B>,
	}
}

impl<S, F, T, Fut, B> Map<S, F, T, Fut, B> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        Self {
            stream,
            f,
            future: None,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, Fut, B> Stream for Map<S, F, S::Item, Fut, B>
where
    S: Stream,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = B>,
{
    type Item = B;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            match this.future.is_some() {
                false => {
                    let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));
                    match next {
                        Some(v) => {
                            let fut = (this.f)(v);
                            this.future.as_mut().set(Some(fut));
                        }
                        None => {
                            return Poll::Ready(None);
                        }
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));
                    this.future.as_mut().set(None);
                    return Poll::Ready(Some(res));
                }
            }
        }
    }
}
