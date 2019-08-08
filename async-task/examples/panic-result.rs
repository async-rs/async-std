//! A single-threaded executor where join handles catch panics inside tasks.

#![feature(async_await)]

use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::thread;

use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use futures::future::FutureExt;
use lazy_static::lazy_static;

/// Spawns a future on the executor.
fn spawn<F, R>(future: F) -> async_task::JoinHandle<thread::Result<R>, ()>
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
                    // No need for `catch_unwind()` here because panics are already caught.
                    task.run();
                }
            });

            sender
        };
    }

    // Create a future that catches panics within itself.
    let future = AssertUnwindSafe(future).catch_unwind();

    // Create a task that is scheduled by sending itself into the channel.
    let schedule = |t| QUEUE.send(t).unwrap();
    let (task, handle) = async_task::spawn(future, schedule, ());

    // Schedule the task by sending it into the channel.
    task.schedule();

    handle
}

fn main() {
    // Spawn a future that completes succesfully.
    let handle = spawn(async {
        println!("Hello, world!");
    });

    // Block on the future and report its result.
    match executor::block_on(handle) {
        None => println!("The task was cancelled."),
        Some(Ok(val)) => println!("The task completed with {:?}", val),
        Some(Err(_)) => println!("The task has panicked"),
    }

    // Spawn a future that panics.
    let handle = spawn(async {
        panic!("Ooops!");
    });

    // Block on the future and report its result.
    match executor::block_on(handle) {
        None => println!("The task was cancelled."),
        Some(Ok(val)) => println!("The task completed with {:?}", val),
        Some(Err(_)) => println!("The task has panicked"),
    }
}
