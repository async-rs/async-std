use crate::task::{Task, TaskLocalsWrapper};

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
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// println!("The name of this task is {:?}", task::current().name());
/// #
/// # })
/// ```
#[must_use] pub fn current() -> Task {
    TaskLocalsWrapper::get_current(|t| t.task().clone())
        .expect("`task::current()` called outside the context of a task")
}
