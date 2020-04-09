use core::pin::Pin;
use core::marker::PhantomData;
use core::future::Future;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct ForEachFuture<S, F, Fut, T> {
        #[pin]
        stream: S,
        f: F,
		#[pin]
		future: Option<Fut>,
		__t: PhantomData<T>,
    }
}

impl<S, F, Fut, T> ForEachFuture<S, F, Fut, T> {
    pub(super) fn new(stream: S, f: F) -> Self {
        Self {
            stream,
            f,
            future: None,
            __t: PhantomData,
        }
    }
}

impl<S, F, Fut> Future for ForEachFuture<S, F, Fut, S::Item>
where
    S: Stream + Sized,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = ()>,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
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
                        None => return Poll::Ready(()),
                    }
                }
                true => {
                    let _ =
                        futures_core::ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));

                    this.future.as_mut().set(None);
                }
            }
        }
    }
}
