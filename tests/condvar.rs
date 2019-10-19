use std::sync::Arc;
use std::time::Duration;

use async_std::sync::{Condvar, Mutex};
use async_std::task;

#[test]
fn wait_timeout() {
    task::block_on(async {
        let m = Mutex::new(());
        let c = Condvar::new();
        let (_, wait_result) = c
            .wait_timeout(m.lock().await, Duration::from_millis(10))
            .await;
        assert!(wait_result.timed_out());
    })
}
