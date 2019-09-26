use std::sync::Arc;

use futures_channel::mpsc::unbounded;
use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

use async_std::sync::Barrier;
use async_std::task;

#[test]
fn test_barrier() {
    // Based on the test in std, I was seeing some race conditions, so running it in a loop to make sure
    // things are solid.

    for _ in 0..1_000 {
        task::block_on(async move {
            const N: usize = 10;

            let barrier = Arc::new(Barrier::new(N));
            let (tx, mut rx) = unbounded();

            for _ in 0..N - 1 {
                let c = barrier.clone();
                let mut tx = tx.clone();
                task::spawn(async move {
                    let res = c.wait().await;

                    tx.send(res.is_leader()).await.unwrap();
                });
            }

            // At this point, all spawned threads should be blocked,
            // so we shouldn't get anything from the port
            let res = rx.try_next();
            assert!(match res {
                Err(_err) => true,
                _ => false,
            });

            let mut leader_found = barrier.wait().await.is_leader();

            // Now, the barrier is cleared and we should get data.
            for _ in 0..N - 1 {
                if rx.next().await.unwrap() {
                    assert!(!leader_found);
                    leader_found = true;
                }
            }
            assert!(leader_found);
        });
    }
}
