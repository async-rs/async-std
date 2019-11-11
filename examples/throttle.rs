//! Spawns a timed task which gets throttled.

fn main() {
    #[cfg(feature = "unstable")]
    {
        use async_std::prelude::*;
        use async_std::task;

        task::block_on(async {
            use async_std::stream;
            use std::time::Duration;

            // emit value every 1 second
            let s = stream::interval(Duration::from_nanos(1000000)).enumerate();

            // throttle for 2 seconds
            let s = s.throttle(Duration::from_secs(2));

            s.for_each(|(n, _)| {
                dbg!(n);
            })
            .await;
            // => 0 .. 1 .. 2 .. 3
            // with a pause of 2 seconds between each print
        })
    }
}
