use std::pin::Pin;
use std::time::Duration;

use slab::Slab;

use super::mutex::{guard_lock, MutexGuard};
use crate::future::{timeout, Future};
use crate::task::{Context, Poll, Waker};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct WaitTimeoutResult(bool);

/// A type indicating whether a timed wait on a condition variable returned due to a time out or
/// not
impl WaitTimeoutResult {
    /// Returns `true` if the wait was known to have timed out.
    pub fn timed_out(self) -> bool {
        self.0
    }
}

/// A Condition Variable
///
/// This type is an async version of [`std::sync::Mutex`].
///
/// [`std::sync::Condvar`]: https://doc.rust-lang.org/std/sync/struct.Condvar.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use std::sync::Arc;
///
/// use async_std::sync::{Mutex, Condvar};
/// use async_std::task;
///
/// let pair = Arc::new((Mutex::new(false), Condvar::new()));
/// let pair2 = pair.clone();
///
/// // Inside of our lock, spawn a new thread, and then wait for it to start.
/// task::spawn(async move {
///     let (lock, cvar) = &*pair2;
///     let mut started = lock.lock().await;
///     *started = true;
///     // We notify the condvar that the value has changed.
///     cvar.notify_one();
/// });
///
/// // Wait for the thread to start up.
/// let (lock, cvar) = &*pair;
/// let mut started = lock.lock().await;
/// while !*started {
///     started = cvar.wait(started).await;
/// }
///
/// # })
/// ```
#[derive(Debug)]
pub struct Condvar {
    blocked: std::sync::Mutex<Slab<Option<Waker>>>,
}

impl Default for Condvar {
    fn default() -> Self {
        Condvar::new()
    }
}

impl Condvar {
    /// Creates a new condition variable
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::Condvar;
    ///
    /// let cvar = Condvar::new();
    /// ```
    pub fn new() -> Self {
        Condvar {
            blocked: std::sync::Mutex::new(Slab::new()),
        }
    }

    /// Blocks the current task until this condition variable receives a notification.
    ///
    /// Unlike the std equivalent, this does not check that a single mutex is used at runtime.
    /// However, as a best practice avoid using with multiple mutexes.
    ///
    /// # Warning
    /// Any attempt to poll this future before the notification is received will result in a
    /// spurious wakeup. This allows the implementation to be efficient, and is technically valid
    /// semantics for a condition variable. However, this may result in unexpected behaviour when this future is
    /// used with future combinators. In most cases `Condvar::wait_until` is easier to use correctly.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///     let (lock, cvar) = &*pair2;
    ///     let mut started = lock.lock().await;
    ///     *started = true;
    ///     // We notify the condvar that the value has changed.
    ///     cvar.notify_one();
    /// });
    ///
    /// // Wait for the thread to start up.
    /// let (lock, cvar) = &*pair;
    /// let mut started = lock.lock().await;
    /// while !*started {
    ///     started = cvar.wait(started).await;
    /// }
    /// # })
    /// ```
    #[allow(clippy::needless_lifetimes)]
    pub async fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        let mutex = guard_lock(&guard);

        self.await_notify(guard).await;

        mutex.lock().await
    }

    fn await_notify<'a, T>(&self, guard: MutexGuard<'a, T>) -> AwaitNotify<'_, 'a, T> {
        AwaitNotify {
            cond: self,
            guard: Some(guard),
            key: None,
            notified: false,
        }
    }

    /// Blocks the current taks until this condition variable receives a notification and the
    /// required condition is met. Spurious wakeups are ignored and this function will only
    /// return once the condition has been met.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///     let (lock, cvar) = &*pair2;
    ///     let mut started = lock.lock().await;
    ///     *started = true;
    ///     // We notify the condvar that the value has changed.
    ///     cvar.notify_one();
    /// });
    ///
    /// // Wait for the thread to start up.
    /// let (lock, cvar) = &*pair;
    /// // As long as the value inside the `Mutex<bool>` is `false`, we wait.
    /// let _guard = cvar.wait_until(lock.lock().await, |started| { *started }).await;
    /// #
    /// # })
    /// ```
    #[allow(clippy::needless_lifetimes)]
    pub async fn wait_until<'a, T, F>(
        &self,
        mut guard: MutexGuard<'a, T>,
        mut condition: F,
    ) -> MutexGuard<'a, T>
    where
        F: FnMut(&mut T) -> bool,
    {
        while !condition(&mut *guard) {
            guard = self.wait(guard).await;
        }
        guard
    }

    /// Waits on this condition variable for a notification, timing out after a specified duration.
    ///
    /// # Warning
    /// This has similar limitations to  `Condvar::wait`, where polling before a notify is sent can
    /// result in a spurious wakeup. In addition, the timeout may itself trigger a spurious wakeup,
    /// if no other task is holding the mutex when the future is polled. Thus the
    /// `WaitTimeoutResult` should not be trusted to determine if the condition variable was
    /// actually notified.
    ///
    /// For these reasons `Condvar::wait_timeout_until` is recommended in most cases.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///   let (lock, cvar) = &*pair2;
    ///   let mut started = lock.lock().await;
    ///   *started = true;
    ///   // We notify the condvar that the value has changed.
    ///   cvar.notify_one();
    /// });
    ///
    /// // wait for the thread to start up
    /// let (lock, cvar) = &*pair;
    /// let mut started = lock.lock().await;
    /// loop {
    ///   let result = cvar.wait_timeout(started, Duration::from_millis(10)).await;
    ///   started = result.0;
    ///   if *started == true {
    ///       // We received the notification and the value has been updated, we can leave.
    ///       break
    ///   }
    /// }
    /// #
    /// # })
    /// ```
    #[cfg(feature = "unstable")]
    #[allow(clippy::needless_lifetimes)]
    pub async fn wait_timeout<'a, T>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult) {
        let mutex = guard_lock(&guard);
        match timeout(dur, self.wait(guard)).await {
            Ok(guard) => (guard, WaitTimeoutResult(false)),
            Err(_) => (mutex.lock().await, WaitTimeoutResult(true)),
        }
    }

    /// Waits on this condition variable for a notification, timing out after a specified duration.
    /// Spurious wakes will not cause this function to return.
    ///
    /// # Examples
    /// ```
    /// # async_std::task::block_on(async {
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///     let (lock, cvar) = &*pair2;
    ///     let mut started = lock.lock().await;
    ///     *started = true;
    ///     // We notify the condvar that the value has changed.
    ///     cvar.notify_one();
    /// });
    ///
    /// // wait for the thread to start up
    /// let (lock, cvar) = &*pair;
    /// let result = cvar.wait_timeout_until(
    ///     lock.lock().await,
    ///     Duration::from_millis(100),
    ///     |&mut started| started,
    /// ).await;
    /// if result.1.timed_out() {
    ///     // timed-out without the condition ever evaluating to true.
    /// }
    /// // access the locked mutex via result.0
    /// # });
    /// ```
    #[allow(clippy::needless_lifetimes)]
    pub async fn wait_timeout_until<'a, T, F>(
        &self,
        guard: MutexGuard<'a, T>,
        dur: Duration,
        condition: F,
    ) -> (MutexGuard<'a, T>, WaitTimeoutResult)
    where
        F: FnMut(&mut T) -> bool,
    {
        let mutex = guard_lock(&guard);
        match timeout(dur, self.wait_until(guard, condition)).await {
            Ok(guard) => (guard, WaitTimeoutResult(false)),
            Err(_) => (mutex.lock().await, WaitTimeoutResult(true)),
        }
    }

    /// Wakes up one blocked task on this condvar.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///     let (lock, cvar) = &*pair2;
    ///     let mut started = lock.lock().await;
    ///     *started = true;
    ///     // We notify the condvar that the value has changed.
    ///     cvar.notify_one();
    /// });
    ///
    /// // Wait for the thread to start up.
    /// let (lock, cvar) = &*pair;
    /// let mut started = lock.lock().await;
    /// while !*started {
    ///     started = cvar.wait(started).await;
    /// }
    /// # }) }
    /// ```
    pub fn notify_one(&self) {
        let blocked = self.blocked.lock().unwrap();
        notify(blocked, false);
    }

    /// Wakes up all blocked tasks on this condvar.
    ///
    /// # Examples
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::{Mutex, Condvar};
    /// use async_std::task;
    ///
    /// let pair = Arc::new((Mutex::new(false), Condvar::new()));
    /// let pair2 = pair.clone();
    ///
    /// task::spawn(async move {
    ///     let (lock, cvar) = &*pair2;
    ///     let mut started = lock.lock().await;
    ///     *started = true;
    ///     // We notify the condvar that the value has changed.
    ///     cvar.notify_all();
    /// });
    ///
    /// // Wait for the thread to start up.
    /// let (lock, cvar) = &*pair;
    /// let mut started = lock.lock().await;
    /// // As long as the value inside the `Mutex<bool>` is `false`, we wait.
    /// while !*started {
    ///     started = cvar.wait(started).await;
    /// }
    /// #
    /// # }) }
    /// ```
    pub fn notify_all(&self) {
        let blocked = self.blocked.lock().unwrap();
        notify(blocked, true);
    }
}

#[inline]
fn notify(mut blocked: std::sync::MutexGuard<'_, Slab<Option<Waker>>>, all: bool) {
    for (_, entry) in blocked.iter_mut() {
        if let Some(w) = entry.take() {
            w.wake();
            if !all {
                return;
            }
        }
    }
}

struct AwaitNotify<'a, 'b, T> {
    cond: &'a Condvar,
    guard: Option<MutexGuard<'b, T>>,
    key: Option<usize>,
    notified: bool,
}

impl<'a, 'b, T> Future for AwaitNotify<'a, 'b, T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.guard.take() {
            Some(_) => {
                let mut blocked = self.cond.blocked.lock().unwrap();
                let w = cx.waker().clone();
                self.key = Some(blocked.insert(Some(w)));

                // the guard is dropped when we return, which frees the lock
                Poll::Pending
            }
            None => {
                self.notified = true;
                Poll::Ready(())
            }
        }
    }
}

impl<'a, 'b, T> Drop for AwaitNotify<'a, 'b, T> {
    fn drop(&mut self) {
        if let Some(key) = self.key {
            let mut blocked = self.cond.blocked.lock().unwrap();
            let opt_waker = blocked.remove(key);

            if opt_waker.is_none() && !self.notified {
                // wake up the next task, because this task was notified, but
                // we are dropping it before it can finished.
                // This may result in a spurious wake-up, but that's ok.
                notify(blocked, false);
            }
        }
    }
}
