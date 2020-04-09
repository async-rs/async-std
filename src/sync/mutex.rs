use std::cell::UnsafeCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::future::Future;

use crate::sync::WakerSet;
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
/// use async_std::sync::{Arc, Mutex};
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
pub struct Mutex<T: ?Sized> {
    locked: AtomicBool,
    wakers: WakerSet,
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}

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
            wakers: WakerSet::new(),
            value: UnsafeCell::new(t),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// Acquires the lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::{Arc, Mutex};
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
        pub struct LockFuture<'a, T: ?Sized> {
            mutex: &'a Mutex<T>,
            opt_key: Option<usize>,
        }

        impl<'a, T: ?Sized> Future for LockFuture<'a, T> {
            type Output = MutexGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                loop {
                    // If the current task is in the set, remove it.
                    if let Some(key) = self.opt_key.take() {
                        self.mutex.wakers.remove(key);
                    }

                    // Try acquiring the lock.
                    match self.mutex.try_lock() {
                        Some(guard) => return Poll::Ready(guard),
                        None => {
                            // Insert this lock operation.
                            self.opt_key = Some(self.mutex.wakers.insert(cx));

                            // If the mutex is still locked, return.
                            if self.mutex.locked.load(Ordering::SeqCst) {
                                return Poll::Pending;
                            }
                        }
                    }
                }
            }
        }

        impl<T: ?Sized> Drop for LockFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
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
    /// use async_std::sync::{Arc, Mutex};
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
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if !self.locked.swap(true, Ordering::SeqCst) {
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
    pub fn into_inner(self) -> T where T: Sized {
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

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
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

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

/// A guard that releases the lock when dropped.
pub struct MutexGuard<'a, T: ?Sized>(&'a Mutex<T>);

unsafe impl<T: ?Sized + Send> Send for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // Use `SeqCst` ordering to synchronize with `WakerSet::insert()` and `WakerSet::update()`.
        self.0.locked.store(false, Ordering::SeqCst);

        // Notify a blocked `lock()` operation if none were notified already.
        self.0.wakers.notify_any();
    }
}

impl<T: ?Sized +fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
