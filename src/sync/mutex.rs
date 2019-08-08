use std::cell::UnsafeCell;
use std::fmt;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

use slab::Slab;

/// Set if the mutex is locked.
const LOCK: usize = 1 << 0;

/// Set if there are tasks blocked on the mutex.
const BLOCKED: usize = 1 << 1;

/// A mutual exclusion primitive for protecting shared data.
///
/// This type is an async version of [`std::sync::Mutex`].
///
/// [`std::sync::Mutex`]: https://doc.rust-lang.org/std/sync/struct.Mutex.html
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// use async_std::{sync::Mutex, task};
/// use std::sync::Arc;
///
/// # futures::executor::block_on(async {
/// let m = Arc::new(Mutex::new(0));
/// let mut tasks = vec![];
///
/// for _ in 0..10 {
///     let m = m.clone();
///     tasks.push(task::spawn(async move {
///         *m.lock().await += 1;
///     }));
/// }
///
/// for t in tasks {
///     t.await;
/// }
/// assert_eq!(*m.lock().await, 10);
/// # })
/// ```
pub struct Mutex<T> {
    state: AtomicUsize,
    blocked: std::sync::Mutex<Slab<Option<Waker>>>,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a new mutex.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::Mutex;
    ///
    /// let mutex = Mutex::new(0);
    /// ```
    pub fn new(t: T) -> Mutex<T> {
        Mutex {
            state: AtomicUsize::new(0),
            blocked: std::sync::Mutex::new(Slab::new()),
            value: UnsafeCell::new(t),
        }
    }

    /// Acquires the lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use async_std::{sync::Mutex, task};
    /// use std::sync::Arc;
    ///
    /// # futures::executor::block_on(async {
    /// let m1 = Arc::new(Mutex::new(10));
    /// let m2 = m1.clone();
    ///
    /// task::spawn(async move {
    ///     *m1.lock().await = 20;
    /// })
    /// .await;
    ///
    /// assert_eq!(*m2.lock().await, 20);
    /// # })
    /// ```
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        pub struct LockFuture<'a, T> {
            mutex: &'a Mutex<T>,
            opt_key: Option<usize>,
            acquired: bool,
        }

        impl<'a, T> Future for LockFuture<'a, T> {
            type Output = MutexGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.mutex.try_lock() {
                    Some(guard) => {
                        self.acquired = true;
                        Poll::Ready(guard)
                    }
                    None => {
                        let mut blocked = self.mutex.blocked.lock().unwrap();

                        // Register the current task.
                        match self.opt_key {
                            None => {
                                // Insert a new entry into the list of blocked tasks.
                                let w = cx.waker().clone();
                                let key = blocked.insert(Some(w));
                                self.opt_key = Some(key);

                                if blocked.len() == 1 {
                                    self.mutex.state.fetch_or(BLOCKED, Ordering::Relaxed);
                                }
                            }
                            Some(key) => {
                                // There is already an entry in the list of blocked tasks. Just
                                // reset the waker if it was removed.
                                if blocked[key].is_none() {
                                    let w = cx.waker().clone();
                                    blocked[key] = Some(w);
                                }
                            }
                        }

                        // Try locking again because it's possible the mutex got unlocked just
                        // before the current task was registered as a blocked task.
                        match self.mutex.try_lock() {
                            Some(guard) => {
                                self.acquired = true;
                                Poll::Ready(guard)
                            }
                            None => Poll::Pending,
                        }
                    }
                }
            }
        }

        impl<T> Drop for LockFuture<'_, T> {
            fn drop(&mut self) {
                if let Some(key) = self.opt_key {
                    let mut blocked = self.mutex.blocked.lock().unwrap();
                    let opt_waker = blocked.remove(key);

                    if opt_waker.is_none() && !self.acquired {
                        // We were awoken but didn't acquire the lock. Wake up another task.
                        if let Some((_, opt_waker)) = blocked.iter_mut().next() {
                            if let Some(w) = opt_waker.take() {
                                w.wake();
                            }
                        }
                    }

                    if blocked.is_empty() {
                        self.mutex.state.fetch_and(!BLOCKED, Ordering::Relaxed);
                    }
                }
            }
        }

        LockFuture {
            mutex: self,
            opt_key: None,
            acquired: false,
        }
        .await
    }

    /// Attempts to acquire the lock.
    ///
    /// If the lock could not be acquired at this time, then [`None`] is returned. Otherwise, a
    /// guard is returned that releases the lock when dropped.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use async_std::{sync::Mutex, task};
    /// use std::sync::Arc;
    ///
    /// # futures::executor::block_on(async {
    /// let m1 = Arc::new(Mutex::new(10));
    /// let m2 = m1.clone();
    ///
    /// task::spawn(async move {
    ///     if let Some(mut guard) = m1.try_lock() {
    ///         *guard = 20;
    ///     } else {
    ///         println!("try_lock failed");
    ///     }
    /// })
    /// .await;
    ///
    /// assert_eq!(*m2.lock().await, 20);
    /// # })
    /// ```
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.state.fetch_or(LOCK, Ordering::Acquire) & LOCK == 0 {
            Some(MutexGuard(self))
        } else {
            None
        }
    }

    /// Consumes the mutex, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use async_std::sync::Mutex;
    ///
    /// let mutex = Mutex::new(10);
    /// assert_eq!(mutex.into_inner(), 10);
    /// ```
    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the mutex mutably, no actual locking takes place -- the mutable
    /// borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use async_std::sync::Mutex;
    ///
    /// # futures::executor::block_on(async {
    /// let mut mutex = Mutex::new(0);
    /// *mutex.get_mut() = 10;
    /// assert_eq!(*mutex.lock().await, 10);
    /// });
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_lock() {
            None => {
                struct LockedPlaceholder;
                impl fmt::Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                f.debug_struct("Mutex")
                    .field("data", &LockedPlaceholder)
                    .finish()
            }
            Some(guard) => f.debug_struct("Mutex").field("data", &&*guard).finish(),
        }
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(val: T) -> Mutex<T> {
        Mutex::new(val)
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

/// A guard that releases the lock when dropped.
pub struct MutexGuard<'a, T>(&'a Mutex<T>);

unsafe impl<T: Send> Send for MutexGuard<'_, T> {}
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let state = self.0.state.fetch_and(!LOCK, Ordering::AcqRel);

        // If there are any blocked tasks, wake one of them up.
        if state & BLOCKED != 0 {
            let mut blocked = self.0.blocked.lock().unwrap();

            if let Some((_, opt_waker)) = blocked.iter_mut().next() {
                // If there is no waker in this entry, that means it was already woken.
                if let Some(w) = opt_waker.take() {
                    w.wake();
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
