use std::future::Future;

use crate::task::{Builder, JoinHandle};

/// Spawns a task.
///
/// This function is similar to [`std::thread::spawn`], except it spawns an asynchronous task.
///
/// [`std::thread`]: https://doc.rust-lang.org/std/thread/fn.spawn.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// let handle = task::spawn(async {
///     1 + 2
/// });
///
/// assert_eq!(handle.await, 3);
/// #
/// # })
/// ```
///
/// ```
/// use async_std::task;
/// use std::time::Duration;
///
/// async fn clock() {
///     loop {
///        task::sleep(Duration::from_secs(1)).await;
///        println!("Tick");
///    }
///}
///
/// #[async_std::main]
/// async fn main() {
///    println!("Start");
///    task::spawn(clock());
///
///    for i in (0..100).rev() {
///        println!("Countdown {}", i);
///        task::sleep(Duration::from_secs(2)).await;
///    }
///    println!("End");
///}
/// ```
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    Builder::new().spawn(future).expect("cannot spawn task")
}
