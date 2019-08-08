//! A function that runs a future to completion on a dedicated thread.

#![feature(async_await)]

use std::future::Future;
use std::sync::Arc;
use std::thread;

use crossbeam::channel;
use futures::executor;

/// Spawns a future on a new dedicated thread.
///
/// The returned handle can be used to await the output of the future.
fn spawn_on_thread<F, R>(future: F) -> async_task::JoinHandle<R, ()>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    // Create a channel that holds the task when it is scheduled for running.
    let (sender, receiver) = channel::unbounded();
    let sender = Arc::new(sender);
    let s = Arc::downgrade(&sender);

    // Wrap the future into one that disconnects the channel on completion.
    let future = async move {
        // When the inner future completes, the sender gets dropped and disconnects the channel.
        let _sender = sender;
        future.await
    };

    // Create a task that is scheduled by sending itself into the channel.
    let schedule = move |t| s.upgrade().unwrap().send(t).unwrap();
    let (task, handle) = async_task::spawn(future, schedule, ());

    // Schedule the task by sending it into the channel.
    task.schedule();

    // Spawn a thread running the task to completion.
    thread::spawn(move || {
        // Keep taking the task from the channel and running it until completion.
        for task in receiver {
            task.run();
        }
    });

    handle
}

fn main() {
    // Spawn a future on a dedicated thread.
    executor::block_on(spawn_on_thread(async {
        println!("Hello, world!");
    }));
}
