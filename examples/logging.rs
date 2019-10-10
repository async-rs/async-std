//! Prints the runtime's execution log on the standard output.

use async_std::thread;

fn main() {
    femme::start(log::LevelFilter::Trace).unwrap();

    thread::spawn_task(async {
        let handle = task::spawn(async {
            log::info!("Hello world!");
        });

        handle.await;
    })
}
