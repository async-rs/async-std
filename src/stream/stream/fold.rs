use core::marker::PhantomData;
use core::future::Future;
use core::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[derive(Debug)]
    pub struct FoldFuture<S, F, Fut, T, B> {
        #[pin]
        stream: S,
        f: F,
		#[pin]
		future: Option<Fut>,
        acc: Option<B>,
		__t: PhantomData<T>,
    }
}

impl<S, F, Fut, T, B> FoldFuture<S, F, Fut, T, B> {
    pub(super) fn new(stream: S, init: B, f: F) -> Self {
        Self {
            stream,
            f,
            future: None,
            acc: Some(init),
			__t: PhantomData,
        }
    }
}

impl<S, F, Fut, B> Future for FoldFuture<S, F, Fut, S::Item, B>
where
    S: Stream + Sized,
    F: FnMut(B, S::Item) -> Fut,
    Fut: Future<Output = B>,
{
    type Output = B;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            match this.future.is_some() {
                false => {
                    let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

                    match next {
                        Some(v) => {
                            let old = this.acc.take().unwrap();
                            let fut = (this.f)(old, v);
                            this.future.as_mut().set(Some(fut));
                        }
                        None => return Poll::Ready(this.acc.take().unwrap()),
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));
                    this.future.as_mut().set(None);
                    *this.acc = Some(res);
                }
            }
        }
    }
}
