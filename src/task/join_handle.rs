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
pub struct JoinHandle<T>(async_task::JoinHandle<T, Task>);

unsafe impl<T> Send for JoinHandle<T> {}
unsafe impl<T> Sync for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    /// Creates a new `JoinHandle`.
    pub(crate) fn new(inner: async_task::JoinHandle<T, Task>) -> JoinHandle<T> {
        JoinHandle(inner)
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
        self.0.tag()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => panic!("cannot await the result of a panicked task"),
            Poll::Ready(Some(val)) => Poll::Ready(val),
        }
    }
}
