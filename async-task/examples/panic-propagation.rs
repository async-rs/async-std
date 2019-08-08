//! A single-threaded executor where join handles propagate panics from tasks.

#![feature(async_await)]

use std::future::Future;
use std::panic::{resume_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

use crossbeam::channel::{unbounded, Sender};
use futures::executor;
use futures::future::FutureExt;
use lazy_static::lazy_static;

/// Spawns a future on the executor.
fn spawn<F, R>(future: F) -> JoinHandle<R>
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

    // Wrap the handle into one that propagates panics.
    JoinHandle(handle)
}

/// A join handle that propagates panics inside the task.
struct JoinHandle<R>(async_task::JoinHandle<thread::Result<R>, ()>);

impl<R> Future for JoinHandle<R> {
    type Output = Option<R>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Ok(val))) => Poll::Ready(Some(val)),
            Poll::Ready(Some(Err(err))) => resume_unwind(err),
        }
    }
}

fn main() {
    // Spawn a future that panics and block on it.
    let handle = spawn(async {
        panic!("Ooops!");
    });
    executor::block_on(handle);
}
