#![feature(test)]

extern crate test;

use async_std::future::Future;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use futures::task::noop_waker;
use test::Bencher;

async fn test(task: usize, iter: usize) {
    let mutex = Arc::new(Mutex::new(()));
    let mut vec = Vec::new();
    for _ in 0..task {
        let mutex_clone = mutex.clone();
        let handle = async_std::task::spawn(async move {
            for _ in 0..iter {
                let _ = mutex_clone.lock().await;
            }
        });
        vec.push(handle);
    }
    for i in vec {
        i.await
    }
}

#[bench]
fn mutex_contention(b: &mut Bencher) {
    b.iter(|| task::block_on(test(10, 1000)));
}

#[bench]
fn mutex_no_contention(b: &mut Bencher) {
    b.iter(|| task::block_on(test(1, 10000)));
}

#[bench]
fn mutex_unused(b: &mut Bencher) {
    b.iter(|| Mutex::new(()));
}

#[bench]
fn mutex_mimick_contention(b: &mut Bencher) {
    let noop_waker = noop_waker();
    let mut context = task::Context::from_waker(&noop_waker);

    b.iter(|| {
        let mutex = Mutex::new(());
        let mut vec = Vec::with_capacity(10);

        // Mimick 10 tasks concurrently trying to acquire the lock.
        for _ in 0..10 {
            let mut lock_future = Box::pin(mutex.lock());
            let poll_result = lock_future.as_mut().poll(&mut context);
            vec.push((lock_future, poll_result));
        }

        // Go through all 10 tasks and release the lock.
        for (mut future, mut poll) in vec {
            while let task::Poll::Pending = poll {
                poll = future.as_mut().poll(&mut context);
            }
        }
    });
}
