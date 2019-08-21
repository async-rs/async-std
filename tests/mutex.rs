use std::sync::Arc;

use async_std::prelude::*;
use async_std::sync::Mutex;
use async_std::task;
use futures::channel::mpsc;

#[test]
fn smoke() {
    task::block_on(async {
        let m = Mutex::new(());
        drop(m.lock().await);
        drop(m.lock().await);
    })
}

#[test]
fn try_lock() {
    let m = Mutex::new(());
    *m.try_lock().unwrap() = ();
}

#[test]
fn into_inner() {
    let m = Mutex::new(10);
    assert_eq!(m.into_inner(), 10);
}

#[test]
fn get_mut() {
    let mut m = Mutex::new(10);
    *m.get_mut() = 20;
    assert_eq!(m.into_inner(), 20);
}

#[test]
fn contention() {
    task::block_on(async {
        let (tx, mut rx) = mpsc::unbounded();

        let tx = Arc::new(tx);
        let mutex = Arc::new(Mutex::new(0));
        let num_tasks = 10000;

        for _ in 0..num_tasks {
            let tx = tx.clone();
            let mutex = mutex.clone();

            task::spawn(async move {
                let mut lock = mutex.lock().await;
                *lock += 1;
                tx.unbounded_send(()).unwrap();
                drop(lock);
            });
        }

        for _ in 0..num_tasks {
            rx.next().await.unwrap();
        }

        let lock = mutex.lock().await;
        assert_eq!(num_tasks, *lock);
    });
}
