//! A thread pool for running blocking functions asynchronously.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;

use crossbeam::channel::{unbounded, Receiver, Sender};
use lazy_static::lazy_static;

use crate::utils::abort_on_panic;

struct Pool {
    sender: Sender<async_task::Task<()>>,
    receiver: Receiver<async_task::Task<()>>,
}

lazy_static! {
    static ref POOL: Pool = {
        for _ in 0..2 {
            thread::Builder::new()
                .name("async-blocking-driver".to_string())
                .spawn(|| {
                    for task in &POOL.receiver {
                        abort_on_panic(|| task.run());
                    }
                })
                .expect("cannot start a thread driving blocking tasks");
        }

        let (sender, receiver) = unbounded();
        Pool { sender, receiver }
    };
}

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks.
pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    let schedule = |t| POOL.sender.send(t).unwrap();
    let (task, handle) = async_task::spawn(future, schedule, ());
    task.schedule();
    JoinHandle(handle)
}

/// A handle to a blocking task.
pub struct JoinHandle<R>(async_task::JoinHandle<R, ()>);

impl<R> Unpin for JoinHandle<R> {}

impl<R> Future for JoinHandle<R> {
    type Output = R;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|out| out.unwrap())
    }
}

impl<R> fmt::Debug for JoinHandle<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("JoinHandle")
            .field("handle", &self.0)
            .finish()
    }
}
