use std::pin::Pin;

use futures_channel::mpsc::Sender;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pub(crate) struct UnzipSendFuture<S, A, B> {
    stream: S,
    a: (Option<A>, Sender<A>),
    b: (Option<B>, Sender<B>),
}

impl<S, A, B> UnzipSendFuture<S, A, B> {
    pub(crate) fn new(stream: S, tx_a: Sender<A>, tx_b: Sender<B>) -> Self {
        UnzipSendFuture {
            stream,
            a: (None, tx_a),
            b: (None, tx_b),
        }
    }

    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(a: (Option<A>, Sender<A>));
    pin_utils::unsafe_unpinned!(b: (Option<B>, Sender<B>));
}

impl<S, A, B> crate::future::Future for UnzipSendFuture<S, A, B>
where
    S: Stream<Item = (A, B)>,
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // hand-written send-join
        let (item, tx) = self.as_mut().a();
        if item.is_some() {
            let poll_result = tx.poll_ready(cx);
            if poll_result.is_ready() {
                let item = item.take().unwrap();
                let _ = tx.start_send(item);
            }
        }
        let (item, tx) = self.as_mut().b();
        if item.is_some() {
            let poll_result = tx.poll_ready(cx);
            if poll_result.is_ready() {
                let item = item.take().unwrap();
                let _ = tx.start_send(item);
            }
        }
        if self.as_mut().a().0.is_some() || self.as_mut().b().0.is_some() {
            return Poll::Pending;
        }

        let item = futures_core::ready!(self.as_mut().stream().poll_next(cx));
        if let Some((a, b)) = item {
            self.as_mut().a().0 = Some(a);
            self.as_mut().b().0 = Some(b);
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
