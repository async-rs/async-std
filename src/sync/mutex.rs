use std::cell::UnsafeCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::future::Future;
use crate::sync::WakerMap;
use crate::task::{Context, Poll};

/// A mutual exclusion primitive for protecting shared data.
///
/// This type is an async version of [`std::sync::Mutex`].
///
/// [`std::sync::Mutex`]: https://doc.rust-lang.org/std/sync/struct.Mutex.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use std::sync::Arc;
///
/// use async_std::sync::Mutex;
/// use async_std::task;
///
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
/// #
/// # })
/// ```
pub struct Mutex<T> {
    locked: AtomicBool,
    wakers: WakerMap,
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
            locked: AtomicBool::new(false),
            wakers: WakerMap::new(),
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
    /// # async_std::task::block_on(async {
    /// #
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::Mutex;
    /// use async_std::task;
    ///
    /// let m1 = Arc::new(Mutex::new(10));
    /// let m2 = m1.clone();
    ///
    /// task::spawn(async move {
    ///     *m1.lock().await = 20;
    /// })
    /// .await;
    ///
    /// assert_eq!(*m2.lock().await, 20);
    /// #
    /// # })
    /// ```
    pub async fn lock(&self) -> MutexGuard<'_, T> {
        pub struct LockFuture<'a, T> {
            mutex: &'a Mutex<T>,
            opt_key: Option<usize>,
        }

        impl<'a, T> Future for LockFuture<'a, T> {
            type Output = MutexGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let poll = match self.mutex.try_lock() {
                    Some(guard) => Poll::Ready(guard),
                    None => {
                        // Insert this lock operation.
                        match self.opt_key {
                            None => self.opt_key = Some(self.mutex.wakers.insert(cx)),
                            Some(key) => self.mutex.wakers.update(key, cx),
                        }

                        // Try locking again because it's possible the mutex got unlocked just
                        // before the current task was inserted into the waker map.
                        match self.mutex.try_lock() {
                            Some(guard) => Poll::Ready(guard),
                            None => Poll::Pending,
                        }
                    }
                };

                if poll.is_ready() {
                    // If the current task is in the map, remove it.
                    if let Some(key) = self.opt_key.take() {
                        self.mutex.wakers.complete(key);
                    }
                }

                poll
            }
        }

        impl<T> Drop for LockFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the map, that means it is being cancelled now.
                if let Some(key) = self.opt_key {
                    self.mutex.wakers.cancel(key);
                }
            }
        }

        LockFuture {
            mutex: self,
            opt_key: None,
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
    /// # async_std::task::block_on(async {
    /// #
    /// use std::sync::Arc;
    ///
    /// use async_std::sync::Mutex;
    /// use async_std::task;
    ///
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
    /// #
    /// # })
    /// ```
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if !self.locked.swap(true, Ordering::Acquire) {
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
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::Mutex;
    ///
    /// let mut mutex = Mutex::new(0);
    /// *mutex.get_mut() = 10;
    /// assert_eq!(*mutex.lock().await, 10);
    /// #
    /// # })
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Locked;
        impl fmt::Debug for Locked {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<locked>")
            }
        }

        match self.try_lock() {
            None => f.debug_struct("Mutex").field("data", &Locked).finish(),
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
        // Use `AcqRel` to:
        // 1. Release changes made to the value inside the mutex.
        // 2. Acquire changes made to the waker map.
        self.0.locked.swap(false, Ordering::AcqRel);

        // Notify one blocked `lock()` operation.
        self.0.wakers.notify_one();
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
