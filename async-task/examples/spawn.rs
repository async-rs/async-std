//! A simple single-threaded executor.

#![feature(async_await)]

use std::future::Future;
use std::panic::catch_unwind;
use std::thread;

use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use lazy_static::lazy_static;

/// Spawns a future on the executor.
fn spawn<F, R>(future: F) -> async_task::JoinHandle<R, ()>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    lazy_static! {
        // A channel that holds scheduled tasks.
        static ref QUEUE: Sender<async_task::Task<()>> = {
            let (sender, receiver) = unbounded::<async_task::Task<()>>();

            // Start the executor thread.
            thread::spawn(|| {
                for task in receiver {
                    // Ignore panics for simplicity.
                    let _ignore_panic = catch_unwind(|| task.run());
                }
            });

            sender
        };
    }

    // Create a task that is scheduled by sending itself into the channel.
    let schedule = |t| QUEUE.send(t).unwrap();
    let (task, handle) = async_task::spawn(future, schedule, ());

    // Schedule the task by sending it into the channel.
    task.schedule();

    handle
}

fn main() {
    // Spawn a future and await its result.
    let handle = spawn(async {
        println!("Hello, world!");
    });
    executor::block_on(handle);
}
