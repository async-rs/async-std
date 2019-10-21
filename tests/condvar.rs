use std::sync::Arc;
use std::time::Duration;

use async_std::sync::{Condvar, Mutex};
use async_std::task;

#[test]
fn wait_timeout() {
    task::block_on(async {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();

        task::spawn(async move {
            let (m, c) = &*pair2;
            let _g = m.lock().await;
            task::sleep(Duration::from_millis(20)).await;
            c.notify_one();
        });

        let (m, c) = &*pair;
        let (_, wait_result) = c
            .wait_timeout(m.lock().await, Duration::from_millis(10))
            .await;
        assert!(wait_result.timed_out());
    })
}

