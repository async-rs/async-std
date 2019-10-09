use std::fmt;
use std::i64;
use std::mem;
use std::num::NonZeroU64;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use super::task_local;
use crate::future::Future;
use crate::task::{Context, Poll};

/// A handle to a task.
#[derive(Clone)]
pub struct Task(Arc<Metadata>);

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    /// Returns a reference to task metadata.
    pub(crate) fn metadata(&self) -> &Metadata {
        &self.0
    }

    /// Gets the task's unique identifier.
    pub fn id(&self) -> TaskId {
        self.metadata().task_id
    }

    /// Returns the name of this task.
    ///
    /// The name is configured by [`Builder::name`] before spawning.
    ///
    /// [`Builder::name`]: struct.Builder.html#method.name
    pub fn name(&self) -> Option<&str> {
        self.metadata().name.as_ref().map(|s| s.as_str())
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task").field("name", &self.name()).finish()
    }
}

/// A handle that awaits the result of a task.
///
/// Dropping a [`JoinHandle`] will detach the task, meaning that there is no longer
/// a handle to the task and no way to `join` on it.
///
/// Created when a task is [spawned].
///
/// [spawned]: fn.spawn.html
#[derive(Debug)]
pub struct JoinHandle<T>(async_task::JoinHandle<T, Tag>);

unsafe impl<T> Send for JoinHandle<T> {}
unsafe impl<T> Sync for JoinHandle<T> {}

impl<T> JoinHandle<T> {
    pub(crate) fn new(inner: async_task::JoinHandle<T, Tag>) -> JoinHandle<T> {
        JoinHandle(inner)
    }

    /// Returns a handle to the underlying task.
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
    /// println!("id = {}", handle.task().id());
    /// #
    /// # }) }
    pub fn task(&self) -> &Task {
        self.0.tag().task()
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.0).poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => panic!("task has panicked"),
            Poll::Ready(Some(val)) => Poll::Ready(val),
        }
    }
}

/// A unique identifier for a task.
///
/// # Examples
///
/// ```
/// #
/// use async_std::task;
///
/// task::block_on(async {
///     println!("id = {:?}", task::current().id());
/// })
/// ```
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct TaskId(NonZeroU64);

impl TaskId {
    pub(crate) fn new() -> TaskId {
        static COUNTER: AtomicU64 = AtomicU64::new(1);

        let id = COUNTER.fetch_add(1, Ordering::Relaxed);

        if id > i64::MAX as u64 {
            std::process::abort();
        }
        unsafe { TaskId(NonZeroU64::new_unchecked(id)) }
    }

    pub(crate) fn as_u64(self) -> u64 {
        self.0.get()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub(crate) type Runnable = async_task::Task<Tag>;

pub(crate) struct Metadata {
    pub task_id: TaskId,
    pub name: Option<String>,
    pub local_map: task_local::Map,
}

pub(crate) struct Tag {
    task_id: TaskId,
    raw_metadata: AtomicUsize,
}

impl Tag {
    pub fn new(name: Option<String>) -> Tag {
        let task_id = TaskId::new();

        let opt_task = name.map(|name| {
            Task(Arc::new(Metadata {
                task_id,
                name: Some(name),
                local_map: task_local::Map::new(),
            }))
        });

        Tag {
            task_id,
            raw_metadata: AtomicUsize::new(unsafe {
                mem::transmute::<Option<Task>, usize>(opt_task)
            }),
        }
    }

    pub fn task(&self) -> &Task {
        unsafe {
            let raw = self.raw_metadata.load(Ordering::Acquire);

            if mem::transmute::<&usize, &Option<Task>>(&raw).is_none() {
                let new = Some(Task(Arc::new(Metadata {
                    task_id: TaskId::new(),
                    name: None,
                    local_map: task_local::Map::new(),
                })));

                let new_raw = mem::transmute::<Option<Task>, usize>(new);

                if self
                    .raw_metadata
                    .compare_exchange(raw, new_raw, Ordering::AcqRel, Ordering::Acquire)
                    .is_err()
                {
                    let new = mem::transmute::<usize, Option<Task>>(new_raw);
                    drop(new);
                }
            }

            mem::transmute::<&AtomicUsize, &Option<Task>>(&self.raw_metadata)
                .as_ref()
                .unwrap()
        }
    }

    pub fn task_id(&self) -> TaskId {
        self.task_id
    }
}

impl Drop for Tag {
    fn drop(&mut self) {
        let raw = *self.raw_metadata.get_mut();
        let opt_task = unsafe { mem::transmute::<usize, Option<Task>>(raw) };
        drop(opt_task);
    }
}
