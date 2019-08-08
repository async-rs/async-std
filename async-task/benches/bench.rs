#![feature(async_await, test)]

extern crate test;

use futures::channel::oneshot;
use futures::executor;
use futures::future::TryFutureExt;
use test::Bencher;

#[bench]
fn task_create(b: &mut Bencher) {
    b.iter(|| {
        async_task::spawn(async {}, drop, ());
    });
}

#[bench]
fn task_run(b: &mut Bencher) {
    b.iter(|| {
        let (task, handle) = async_task::spawn(async {}, drop, ());
        task.run();
        executor::block_on(handle).unwrap();
    });
}

#[bench]
fn oneshot_create(b: &mut Bencher) {
    b.iter(|| {
        let (tx, _rx) = oneshot::channel::<()>();
        let _task = Box::new(async move { tx.send(()).map_err(|_| ()) });
    });
}

#[bench]
fn oneshot_run(b: &mut Bencher) {
    b.iter(|| {
        let (tx, rx) = oneshot::channel::<()>();
        let task = Box::new(async move { tx.send(()).map_err(|_| ()) });

        let future = task.and_then(|_| rx.map_err(|_| ()));
        executor::block_on(future).unwrap();
    });
}
