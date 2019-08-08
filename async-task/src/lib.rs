//! Task abstraction for building executors.
//!
//! # What is an executor?
//!
//! An async block creates a future and an async function returns one. But futures don't do
//! anything unless they are awaited inside other async blocks or async functions. So the question
//! arises: who or what awaits the main future that awaits others?
//!
//! One solution is to call [`block_on()`] on the main future, which will block
//! the current thread and keep polling the future until it completes. But sometimes we don't want
//! to block the current thread and would prefer to *spawn* the future to let a background thread
//! block on it instead.
//!
//! This is where executors step in - they create a number of threads (typically equal to the
//! number of CPU cores on the system) that are dedicated to polling spawned futures. Each executor
//! thread keeps polling spawned futures in a loop and only blocks when all spawned futures are
//! either sleeping or running.
//!
//! # What is a task?
//!
//! In order to spawn a future on an executor, one needs to allocate the future on the heap and
//! keep some state alongside it, like whether the future is ready for polling, waiting to be woken
//! up, or completed. This allocation is usually called a *task*.
//!
//! The executor then runs the spawned task by polling its future. If the future is pending on a
//! resource, a [`Waker`] associated with the task will be registered somewhere so that the task
//! can be woken up and run again at a later time.
//!
//! For example, if the future wants to read something from a TCP socket that is not ready yet, the
//! networking system will clone the task's waker and wake it up once the socket becomes ready.
//!
//! # Task construction
//!
//! A task is constructed with [`Task::create()`]:
//!
//! ```
//! # #![feature(async_await)]
//! let future = async { 1 + 2 };
//! let schedule = |task| unimplemented!();
//!
//! let (task, handle) = async_task::spawn(future, schedule, ());
//! ```
//!
//! The first argument to the constructor, `()` in this example, is an arbitrary piece of data
//! called a *tag*. This can be a task identifier, a task name, task-local storage, or something
//! of similar nature.
//!
//! The second argument is the future that gets polled when the task is run.
//!
//! The third argument is the schedule function, which is called every time when the task gets
//! woken up. This function should push the received task into some kind of queue of runnable
//! tasks.
//!
//! The constructor returns a runnable [`Task`] and a [`JoinHandle`] that can await the result of
//! the future.
//!
//! # Task scheduling
//!
//! TODO
//!
//! # Join handles
//!
//! TODO
//!
//! # Cancellation
//!
//! TODO
//!
//! # Performance
//!
//! TODO: explain single allocation, etc.
//!
//! Task [construction] incurs a single allocation only. The [`Task`] can then be run and its
//! result awaited through the [`JoinHandle`]. When woken, the task gets automatically rescheduled.
//! It's also possible to cancel the task so that it stops running and can't be awaited anymore.
//!
//! [construction]: struct.Task.html#method.create
//! [`JoinHandle`]: struct.JoinHandle.html
//! [`Task`]: struct.Task.html
//! [`Future`]: https://doc.rust-lang.org/nightly/std/future/trait.Future.html
//! [`Waker`]: https://doc.rust-lang.org/nightly/std/task/struct.Waker.html
//! [`block_on()`]: https://docs.rs/futures-preview/*/futures/executor/fn.block_on.html
//!
//! # Examples
//!
//! A simple single-threaded executor:
//!
//! ```
//! # #![feature(async_await)]
//! use std::future::Future;
//! use std::panic::catch_unwind;
//! use std::thread;
//!
//! use async_task::{JoinHandle, Task};
//! use crossbeam::channel::{unbounded, Sender};
//! use futures::executor;
//! use lazy_static::lazy_static;
//!
//! /// Spawns a future on the executor.
//! fn spawn<F, R>(future: F) -> JoinHandle<R, ()>
//! where
//!     F: Future<Output = R> + Send + 'static,
//!     R: Send + 'static,
//! {
//!     lazy_static! {
//!         // A channel that holds scheduled tasks.
//!         static ref QUEUE: Sender<Task<()>> = {
//!             let (sender, receiver) = unbounded::<Task<()>>();
//!
//!             // Start the executor thread.
//!             thread::spawn(|| {
//!                 for task in receiver {
//!                     // Ignore panics for simplicity.
//!                     let _ignore_panic = catch_unwind(|| task.run());
//!                 }
//!             });
//!
//!             sender
//!         };
//!     }
//!
//!     // Create a task that is scheduled by sending itself into the channel.
//!     let schedule = |t| QUEUE.send(t).unwrap();
//!     let (task, handle) = async_task::spawn(future, schedule, ());
//!
//!     // Schedule the task by sending it into the channel.
//!     task.schedule();
//!
//!     handle
//! }
//!
//! // Spawn a future and await its result.
//! let handle = spawn(async {
//!     println!("Hello, world!");
//! });
//! executor::block_on(handle);
//! ```

#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]

mod header;
mod join_handle;
mod raw;
mod state;
mod task;
mod utils;

pub use crate::join_handle::JoinHandle;
pub use crate::task::{spawn, Task};
