//! Prints the runtime's execution log on the standard output.

use async_std::task;
use async_std::prelude::*;

fn main() {
    femme::start(log::LevelFilter::Trace).unwrap();

    task::block_on(async {
        let handle = task::spawn(async {
            async_std::println!("hello").await;
            // log::info!("Hello world!");
        });

        handle.await;
    })
}
