use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use kv_log_macro::trace;

use crate::io;
use crate::task::{JoinHandle, Task, TaskLocalsWrapper};

/// Task builder that configures the settings of a new task.
#[derive(Debug, Default)]
pub struct Builder {
    pub(crate) name: Option<String>,
}

impl Builder {
    /// Creates a new builder.
    #[inline]
    pub fn new() -> Builder {
        Builder { name: None }
    }

    /// Configures the name of the task.
    #[inline]
    pub fn name(mut self, name: String) -> Builder {
        self.name = Some(name);
        self
    }

    fn build<F, T>(self, future: F) -> SupportTaskLocals<F>
    where
        F: Future<Output = T>,
    {
        let name = self.name.map(Arc::new);

        // Create a new task handle.
        let task = Task::new(name);

        once_cell::sync::Lazy::force(&crate::rt::RUNTIME);

        let tag = TaskLocalsWrapper::new(task.clone());

        // FIXME: do not require all futures to be boxed.
        SupportTaskLocals {
            tag,
            future: Box::pin(future),
        }
    }

    /// Spawns a task with the configured settings.
    pub fn spawn<F, T>(self, future: F) -> io::Result<JoinHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let wrapped = self.build(future);

        // Log this `spawn` operation.
        trace!("spawn", {
            task_id: wrapped.tag.id().0,
            parent_task_id: TaskLocalsWrapper::get_current(|t| t.id().0).unwrap_or(0),
        });

        let task = wrapped.tag.task().clone();
        let smol_task = smol::Task::spawn(wrapped).detach();

        Ok(JoinHandle::new(smol_task, task))
    }

    /// Spawns a task with the configured settings, blocking on its execution.
    pub fn blocking<F, T>(self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        let wrapped = self.build(future);

        // Log this `block_on` operation.
        trace!("block_on", {
            task_id: wrapped.tag.id().0,
            parent_task_id: TaskLocalsWrapper::get_current(|t| t.id().0).unwrap_or(0),
        });

        // Run the future as a task.
        unsafe { TaskLocalsWrapper::set_current(&wrapped.tag, || smol::block_on(wrapped)) }
    }
}

/// Wrapper to add support for task locals.
struct SupportTaskLocals<F> {
    tag: TaskLocalsWrapper,
    future: Pin<Box<F>>,
}

impl<F> Drop for SupportTaskLocals<F> {
    fn drop(&mut self) {
        // Log completion on exit.
        trace!("completed", {
            task_id: self.tag.id().0,
        });
    }
}

impl<F: Future> Future for SupportTaskLocals<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { TaskLocalsWrapper::set_current(&self.tag, || Pin::new(&mut self.future).poll(cx)) }
    }
}
