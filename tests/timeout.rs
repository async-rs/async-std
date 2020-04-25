use std::time::Duration;

use async_std::future::timeout;
use async_std::task;

#[test]
fn timeout_future_many() {
    task::block_on(async {
        let futures = (0..100)
            .map(|i| {
                timeout(Duration::from_millis(i * 10), async move {
                    task::sleep(Duration::from_millis(i)).await;
                    Ok::<(), async_std::future::TimeoutError>(())
                })
            })
            .collect::<Vec<_>>();

        for future in futures {
            future.await.unwrap().unwrap();
        }
    });
}
