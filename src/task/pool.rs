use std::cell::{Cell, UnsafeCell};
use std::fmt::Arguments;
use std::mem;
use std::panic::{self, AssertUnwindSafe};
use std::pin::Pin;
use std::ptr;
use std::thread;

use crossbeam_channel::{unbounded, Sender};
use futures::future::FutureExt;
use lazy_static::lazy_static;

use super::task;
use super::{JoinHandle, Task};
use crate::future::Future;
use crate::io;

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
/// # #![feature(async_await)]
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
/// # #![feature(async_await)]
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
    spawn_with_builder(Builder::new(), future, "spawn")
}

/// Spawns a task and blocks the current thread on its result.
///
/// Calling this function is similar to [spawning] a thread and immediately [joining] it, except an
/// asynchronous task will be spawned.
///
/// [spawning]: https://doc.rust-lang.org/std/thread/fn.spawn.html
/// [joining]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// use async_std::task;
///
/// fn main() {
///     task::block_on(async {
///         println!("Hello, world!");
///     })
/// }
/// ```
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T> + Send,
    T: Send,
{
    unsafe {
        // A place on the stack where the result will be stored.
        let out = &mut UnsafeCell::new(None);

        // Wrap the future into one that stores the result into `out`.
        let future = {
            let out = out.get();
            async move {
                let v = AssertUnwindSafe(future).catch_unwind().await;
                *out = Some(v);
            }
        };

        // Pin the future onto the stack.
        futures::pin_mut!(future);

        // Transmute the future into one that is static and sendable.
        let future = mem::transmute::<
            Pin<&mut dyn Future<Output = ()>>,
            Pin<&'static mut (dyn Future<Output = ()> + Send)>,
        >(future);

        // Spawn the future and wait for it to complete.
        futures::executor::block_on(spawn_with_builder(Builder::new(), future, "block_on"));

        // Take out the result.
        match (*out.get()).take().unwrap() {
            Ok(v) => v,
            Err(err) => panic::resume_unwind(err),
        }
    }
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
        Ok(spawn_with_builder(self, future, "spawn"))
    }
}

pub(crate) fn spawn_with_builder<F, T>(
    builder: Builder,
    future: F,
    fn_name: &'static str,
) -> JoinHandle<T>
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
                        TAG.with(|tag| {
                            for job in receiver {
                                tag.set(job.tag());
                                abort_on_panic(|| job.run());
                                tag.set(ptr::null());
                            }
                        });
                    })
                    .expect("cannot start a thread driving tasks");
            }

            sender
        };
    }

    let tag = task::Tag::new(name);
    let schedule = |job| QUEUE.send(job).unwrap();

    let child_id = tag.task_id().as_u64();
    let parent_id = get_task(|t| t.id().as_u64()).unwrap_or(0);
    print(
        format_args!("{}", fn_name),
        LogData {
            parent_id,
            child_id,
        },
    );

    // Wrap the future into one that drops task-local variables on exit.
    let future = async move {
        let res = future.await;

        // Abort on panic because thread-local variables behave the same way.
        abort_on_panic(|| get_task(|task| task.metadata().local_map.clear()));

        print(
            format_args!("{} completed", fn_name),
            LogData {
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

pub(crate) fn get_task<F: FnOnce(&Task) -> R, R>(f: F) -> Option<R> {
    let res = TAG.try_with(|tag| unsafe { tag.get().as_ref().map(task::Tag::task).map(f) });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}

/// Calls a function and aborts if it panics.
///
/// This is useful in unsafe code where we can't recover from panics.
#[inline]
fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    struct Bomb;

    impl Drop for Bomb {
        fn drop(&mut self) {
            std::process::abort();
        }
    }

    let bomb = Bomb;
    let t = f();
    mem::forget(bomb);
    t
}

/// This struct only exists because kv logging isn't supported from the macros right now.
struct LogData {
    parent_id: u64,
    child_id: u64,
}

impl<'a> log::kv::Source for LogData {
    fn visit<'kvs>(
        &'kvs self,
        visitor: &mut dyn log::kv::Visitor<'kvs>,
    ) -> Result<(), log::kv::Error> {
        visitor.visit_pair("parent_id".into(), self.parent_id.into())?;
        visitor.visit_pair("child_id".into(), self.child_id.into())?;
        Ok(())
    }
}

fn print(msg: Arguments<'_>, key_values: impl log::kv::Source) {
    log::logger().log(
        &log::Record::builder()
            .args(msg)
            .key_values(&key_values)
            .level(log::Level::Trace)
            .target(module_path!())
            .module_path(Some(module_path!()))
            .file(Some(file!()))
            .line(Some(line!()))
            .build(),
    );
}
