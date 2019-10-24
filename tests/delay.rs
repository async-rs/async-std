use async_std::future;
use async_std::task;
use async_std::prelude::*;

use std::time::Duration;
use std::time::Instant;

#[test]
fn delay() {
    task::block_on( async {
        let start = Instant::now();
        let a = future::ready("a");
        let b = future::ready("b");
        let c = future::ready("c").delay(Duration::from_secs(1));
        dbg!(future::join!(a, b, c).await);
        let elapsed = start.elapsed().as_secs();
        assert_eq!(elapsed, 1);
    })
}
