use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use async_std::prelude::*;
use async_std::stream;
use async_std::sync::channel;
use async_std::task;

#[test]
/// Checks that streams are merged fully even if one of the components
/// experiences delay.
fn merging_delayed_streams_work() {
    let (sender, receiver) = channel::<i32>(10);

    let mut s = receiver.merge(stream::empty());
    let t = task::spawn(async move {
        let mut xs = Vec::new();
        while let Some(x) = s.next().await {
            xs.push(x);
        }
        xs
    });

    task::block_on(async move {
        task::sleep(std::time::Duration::from_millis(500)).await;
        sender.send(92).await;
        drop(sender);
        let xs = t.await;
        assert_eq!(xs, vec![92])
    });

    let (sender, receiver) = channel::<i32>(10);

    let mut s = stream::empty().merge(receiver);
    let t = task::spawn(async move {
        let mut xs = Vec::new();
        while let Some(x) = s.next().await {
            xs.push(x);
        }
        xs
    });

    task::block_on(async move {
        task::sleep(std::time::Duration::from_millis(500)).await;
        sender.send(92).await;
        drop(sender);
        let xs = t.await;
        assert_eq!(xs, vec![92])
    });
}

pin_project! {
    /// The opposite of `Fuse`: makes the stream panic if polled after termination.
    struct Explode<S> {
        #[pin]
        done: bool,
        #[pin]
        inner: S,
    }
}

impl<S: Stream> Stream for Explode<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        if *this.done {
            panic!("KABOOM!")
        }
        let res = this.inner.poll_next(cx);
        if let Poll::Ready(None) = &res {
            *this.done = true;
        }
        res
    }
}

fn explode<S: Stream>(s: S) -> Explode<S> {
    Explode {
        done: false,
        inner: s,
    }
}

#[test]
fn merge_works_with_unfused_streams() {
    let s1 = explode(stream::once(92));
    let s2 = explode(stream::once(92));
    let mut s = s1.merge(s2);
    let xs = task::block_on(async move {
        let mut xs = Vec::new();
        while let Some(x) = s.next().await {
            xs.push(x)
        }
        xs
    });
    assert_eq!(xs, vec![92, 92]);
}
