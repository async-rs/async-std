//! Repeats given stream values and sum them

#![feature(async_await)]

use async_std::io;
use async_std::prelude::*;
use async_std::stream;
use async_std::task;

fn main() -> io::Result<()> {
    task::block_on(async {
        let mut s = stream::cycle(vec![6, 7, 8]);
        let mut total = 0;

        while let Some(v) = s.next().await {
            total += v;
            if total == 42 {
                println!("Found {} the meaning of life!", total);
                break;
            }
        }

        Ok(())
    })
}
