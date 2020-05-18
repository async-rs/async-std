use crate::task::{JoinHandle, Task};

/// Spawns a blocking task.
///
/// The task will be spawned onto a thread pool specifically dedicated to blocking tasks. This
/// is useful to prevent long-running synchronous operations from blocking the main futures
/// executor.
///
/// See also: [`task::block_on`], [`task::spawn`].
///
/// [`task::block_on`]: fn.block_on.html
/// [`task::spawn`]: fn.spawn.html
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # #[cfg(feature = "unstable")]
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// task::spawn_blocking(|| {
///     println!("long-running task here");
/// })
/// .await;
/// #
/// # })
/// ```
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[inline]
pub fn spawn_blocking<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    once_cell::sync::Lazy::force(&crate::rt::RUNTIME);

    let handle = smol::Task::spawn(async move { f() }).into();
    JoinHandle::new(handle, Task::new(None))
}
