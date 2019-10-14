//! Prints the runtime's execution log on the standard output.

use async_std::task;

fn main() {
    femme::start(log::LevelFilter::Trace).unwrap();

    task::block_on(async {
        let handle = task::spawn(async {
            log::info!("Hello world!");
        });

        handle.await;
    })
}
