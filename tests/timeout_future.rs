#![cfg(feature = "unstable")]

use std::time::Duration;

use async_std::prelude::*;
use async_std::future;
use async_std::task;

#[test]
fn should_timeout() {
    task::block_on(async {        
        let fut = future::pending::<()>();
        let dur = Duration::from_millis(100);
        let res = fut.timeout(dur).await;
        assert!(res.is_err());
    });
}

#[test]
fn should_not_timeout() {
    task::block_on(async {        
        let fut = future::ready(0);
        let dur = Duration::from_millis(100);
        let res = fut.timeout(dur).await;
        assert!(res.is_ok());
    });
}