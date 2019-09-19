use super::pool;
use super::JoinHandle;
use crate::future::Future;
use crate::io;

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
        Ok(pool::get().spawn(future, self))
    }
}
