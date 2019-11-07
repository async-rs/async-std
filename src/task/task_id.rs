use std::fmt;

use crossbeam_utils::atomic::AtomicCell;

/// A unique identifier for a task.
///
/// # Examples
///
/// ```
/// use async_std::task;
///
/// task::block_on(async {
///     println!("id = {:?}", task::current().id());
/// })
/// ```
#[derive(Eq, PartialEq, Clone, Copy, Hash, Debug)]
pub struct TaskId(pub(crate) u64);

impl TaskId {
    /// Generates a new `TaskId`.
    pub(crate) fn generate() -> TaskId {
        static COUNTER: AtomicCell<u64> = AtomicCell::new(1u64);

        let id = COUNTER.fetch_add(1);
        if id > u64::max_value() / 2 {
            std::process::abort();
        }
        TaskId(id)
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
