use std::future::Future;
use std::pin::Pin;

use crate::task::{Context, Poll, Task};

/// A handle that awaits the result of a task.
///
/// Dropping a [`JoinHandle`] will detach the task, meaning that there is no longer
/// a handle to the task and no way to `join` on it.
///
/// Created when a task is [spawned].
///
/// [spawned]: fn.spawn.html
#[derive(Debug)]
pub struct JoinHandle<T> {
    handle: Option<async_task::JoinHandle<T, ()>>,
    task: Task,
}

impl<T> JoinHandle<T> {
    /// Creates a new `JoinHandle`.
    pub(crate) fn new(inner: async_task::JoinHandle<T, ()>, task: Task) -> JoinHandle<T> {
        JoinHandle {
            handle: Some(inner),
            task,
        }
    }

    /// Returns a handle to the underlying task.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::task;
    ///
    /// let handle = task::spawn(async {
    ///     1 + 2
    /// });
    /// println!("id = {}", handle.task().id());
    /// #
    /// # })
    pub fn task(&self) -> &Task {
        &self.task
    }

    /// Cancel this task.
    pub async fn cancel(mut self) -> Option<T> {
        let handle = self.handle.take().unwrap();
        handle.cancel();
        handle.await
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.handle.as_mut().unwrap()).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(output) => {
                Poll::Ready(output.expect("cannot await the result of a panicked task"))
            }
        }
    }
}
