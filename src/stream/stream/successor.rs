use std::pin::Pin;
use std::marker::PhantomData;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[derive(Debug)]
pub struct Successor<F, Fut, T> 
where Fut: Future<Output=T>
{
    successor: F,
    next: T,
    _marker: PhantomData<Fut>
}

pub fn successor<F, Fut, T>(func: F, start: T) -> Successor<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = T>,
    T: Copy,
    {
        Successor {
            successor: func,
            next: start,
            _marker: PhantomData,
        }
    }

impl <F, Fut, T> Successor<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = T>,
    T: Copy,

{
    pin_utils::unsafe_unpinned!(successor: F);
    pin_utils::unsafe_unpinned!(next: T);
}

impl <F, Fut, T> Stream for Successor<F, Fut, T>
where
    Fut: Future<Output = T>,
    F: FnMut(T) -> Fut,
    T: Copy,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {

        match self.as_mut().successor()(self.next).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(val) => {
                self.next = val;
                Poll::Ready(Some(val))
            }
        }
    }
}
