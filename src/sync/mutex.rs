use std::cell::UnsafeCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use slab::Slab;

use crate::future::Future;
use crate::task::{Context, Poll, Waker};

/// Set if the mutex is locked.
const LOCK: usize = 1;

/// Set if there are tasks blocked on the mutex.
const BLOCKED: usize = 1 << 1;

struct RawMutex {
    state: AtomicUsize,
    blocked: std::sync::Mutex<Slab<Option<Waker>>>,
}

unsafe impl Send for RawMutex {}
unsafe impl Sync for RawMutex {}

impl RawMutex {
    /// Creates a new raw mutex.
    #[inline]
    pub fn new() -> RawMutex {
        RawMutex {
            state: AtomicUsize::new(0),
            blocked: std::sync::Mutex::new(Slab::new()),
        }
    }

    /// Acquires the lock.
    ///
    /// We don't use `async` signature here for performance concern.
    #[inline]
    pub fn lock(&self) -> RawLockFuture<'_> {
        RawLockFuture {
            mutex: self,
            opt_key: None,
            acquired: false,
        }
    }

    /// Attempts to acquire the lock.
    #[inline]
    pub fn try_lock(&self) -> bool {
        self.state.fetch_or(LOCK, Ordering::Acquire) & LOCK == 0
    }

    #[cold]
    fn unlock_slow(&self) {
        let mut blocked = self.blocked.lock().unwrap();

        if let Some((_, opt_waker)) = blocked.iter_mut().next() {
            // If there is no waker in this entry, that means it was already woken.
            if let Some(w) = opt_waker.take() {
                w.wake();
            }
        }
    }

    /// Unlock this mutex.
    #[inline]
    pub fn unlock(&self) {
        let state = self.state.fetch_and(!LOCK, Ordering::AcqRel);

        // If there are any blocked tasks, wake one of them up.
        if state & BLOCKED != 0 {
            self.unlock_slow();
        }
    }
}

struct RawLockFuture<'a> {
    mutex: &'a RawMutex,
    opt_key: Option<usize>,
    acquired: bool,
}

impl<'a> Future for RawLockFuture<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.mutex.try_lock() {
            self.acquired = true;
            Poll::Ready(())
        } else {
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
            if self.mutex.try_lock() {
                self.acquired = true;
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }
    }
}

impl Drop for RawLockFuture<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.opt_key {
            let mut blocked = self.mutex.blocked.lock().unwrap();
            let opt_waker = blocked.remove(key);

            if opt_waker.is_none() && !self.acquired {
                // We were awoken but didn't acquire the lock. Wake up another task.
                if let Some((_, opt_waker)) = blocked.iter_mut().next() {
                    // If there is no waker in this entry, that means it was already woken.
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
    mutex: RawMutex,
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
    #[inline]
    pub fn new(t: T) -> Mutex<T> {
        Mutex {
            mutex: RawMutex::new(),
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
        self.mutex.lock().await;
        MutexGuard(self)
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
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self.mutex.try_lock() {
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn from(val: T) -> Mutex<T> {
        Mutex::new(val)
    }
}

impl<T: Default> Default for Mutex<T> {
    #[inline]
    fn default() -> Mutex<T> {
        Mutex::new(Default::default())
    }
}

/// A guard that releases the lock when dropped.
pub struct MutexGuard<'a, T>(&'a Mutex<T>);

unsafe impl<T: Send> Send for MutexGuard<'_, T> {}
unsafe impl<T: Sync> Sync for MutexGuard<'_, T> {}

impl<T> Drop for MutexGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        self.0.mutex.unlock();
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

    #[inline]
    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
