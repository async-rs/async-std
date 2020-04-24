use std::future::Future;

use kv_log_macro::trace;

use crate::io;
use crate::task::{JoinHandle, Task};

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

    /// Spawns a task with the configured settings.
    pub fn spawn<F, T>(self, future: F) -> io::Result<JoinHandle<T>>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // Create a new task handle.
        let task = Task::new(self.name);

        // Log this `spawn` operation.
        trace!("spawn", {
            task_id: task.id().0,
            parent_task_id: Task::get_current(|t| t.id().0).unwrap_or(0),
        });

        let wrapped_future = async move {
            // Drop task-locals on exit.
            defer! {
                Task::get_current(|t| unsafe { t.drop_locals() });
            }

            // Log completion on exit.
            defer! {
                trace!("completed", {
                    task_id: Task::get_current(|t| t.id().0),
                });
            }
            future.await
        };

        once_cell::sync::Lazy::force(&crate::rt::RUNTIME);

        // FIXME: figure out how to set the current task.

        let smol_task = smol::Task::spawn(wrapped_future).detach();
        Ok(JoinHandle::new(smol_task, task))
    }
}
