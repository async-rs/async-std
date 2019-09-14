use std::cell::Cell;
use std::ptr;
use std::thread;

use crossbeam_channel::{unbounded, Sender};
use lazy_static::lazy_static;

use super::log_utils;
use super::task;
use super::{JoinHandle, Task};
use crate::future::Future;
use crate::io;
use crate::utils::abort_on_panic;

/// Returns a handle to the current task.
///
/// # Panics
///
/// This function will panic if not called within the context of a task created by [`block_on`],
/// [`spawn`], or [`Builder::spawn`].
///
/// [`block_on`]: fn.block_on.html
/// [`spawn`]: fn.spawn.html
/// [`Builder::spawn`]: struct.Builder.html#method.spawn
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// println!("The name of this task is {:?}", task::current().name());
/// #
/// # }) }
/// ```
pub fn current() -> Task {
    get_task(|task| task.clone()).expect("`task::current()` called outside the context of a task")
}

/// Spawns a task.
///
/// This function is similar to [`std::thread::spawn`], except it spawns an asynchronous task.
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread/fn.spawn.html
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// let handle = task::spawn(async {
///     1 + 2
/// });
///
/// assert_eq!(handle.await, 3);
/// #
/// # }) }
/// ```
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    spawn_with_builder(Builder::new(), future)
}

/// Task builder that configures the settings of a new task.
#[derive(Debug)]
pub struct Builder {
    pub(crate) name: Option<String>,
}

impl Builder {
    /// Creates a new builder.
    pub fn new() -> Builder {
        Builder { name: None }
    }

    /// Configures the name of the task.
    pub fn name(mut self, name: String) -> Builder {
        self.name = Some(name);
        self
    }

    /// Spawns a task with the configured settings.
    pub fn spawn<F, T>(self, future: F) -> io::Result<JoinHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        Ok(spawn_with_builder(self, future))
    }
}

pub(crate) fn spawn_with_builder<F, T>(builder: Builder, future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let Builder { name } = builder;

    type Job = async_task::Task<task::Tag>;

    lazy_static! {
        static ref QUEUE: Sender<Job> = {
            let (sender, receiver) = unbounded::<Job>();

            for _ in 0..num_cpus::get().max(1) {
                let receiver = receiver.clone();
                thread::Builder::new()
                    .name("async-task-driver".to_string())
                    .spawn(|| {
                        for job in receiver {
                            set_tag(job.tag(), || abort_on_panic(|| job.run()))
                        }
                    })
                    .expect("cannot start a thread driving tasks");
            }

            sender
        };
    }

    let tag = task::Tag::new(name);
    let schedule = |job| QUEUE.send(job).unwrap();

    // Log this `spawn` operation.
    let child_id = tag.task_id().as_u64();
    let parent_id = get_task(|t| t.id().as_u64()).unwrap_or(0);
    log_utils::print(
        format_args!("spawn"),
        log_utils::LogData {
            parent_id,
            child_id,
        },
    );

    // Wrap the future into one that drops task-local variables on exit.
    let future = async move {
        let res = future.await;

        // Abort on panic because thread-local variables behave the same way.
        abort_on_panic(|| get_task(|task| task.metadata().local_map.clear()));

        log_utils::print(
            format_args!("spawn completed"),
            log_utils::LogData {
                parent_id,
                child_id,
            },
        );
        res
    };

    let (task, handle) = async_task::spawn(future, schedule, tag);
    task.schedule();
    JoinHandle::new(handle)
}

thread_local! {
    static TAG: Cell<*const task::Tag> = Cell::new(ptr::null_mut());
}

pub(crate) fn set_tag<F, R>(tag: *const task::Tag, f: F) -> R
where
    F: FnOnce() -> R,
{
    struct ResetTag<'a>(&'a Cell<*const task::Tag>);

    impl Drop for ResetTag<'_> {
        fn drop(&mut self) {
            self.0.set(ptr::null());
        }
    }

    TAG.with(|t| {
        t.set(tag);
        let _guard = ResetTag(t);

        f()
    })
}

pub(crate) fn get_task<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Task) -> R,
{
    let res = TAG.try_with(|tag| unsafe { tag.get().as_ref().map(task::Tag::task).map(f) });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}
