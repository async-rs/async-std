use async_rwlock::{
    RwLock as RawRwLock, RwLockReadGuard as RawRwLockReadGuard,
    RwLockWriteGuard as RawRwLockWriteGuard,
};

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::Ordering;

/// A reader-writer lock for protecting shared data.
///
/// This type is an async version of [`std::sync::RwLock`].
///
/// [`std::sync::RwLock`]: https://doc.rust-lang.org/std/sync/struct.RwLock.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::sync::RwLock;
///
/// let lock = RwLock::new(5);
///
/// // Multiple read locks can be held at a time.
/// let r1 = lock.read().await;
/// let r2 = lock.read().await;
/// assert_eq!(*r1, 5);
/// assert_eq!(*r2, 5);
/// drop((r1, r2));
///
/// // Only one write locks can be held at a time.
/// let mut w = lock.write().await;
/// *w += 1;
/// assert_eq!(*w, 6);
/// #
/// # })
/// ```
pub struct RwLock<T: ?Sized> {
    inner: RawRwLock<T>,
}

unsafe impl<T: ?Sized + Send> Send for RwLock<T> {}
unsafe impl<T: ?Sized + Send + Sync> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    /// Creates a new reader-writer lock.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(0);
    /// ```
    pub fn new(t: T) -> RwLock<T> {
        Self {
            inner: RawRwLock::new(t),
        }
    }
}

impl<T: ?Sized> RwLock<T> {
    /// Acquires a read lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(1);
    ///
    /// let n = lock.read().await;
    /// assert_eq!(*n, 1);
    ///
    /// assert!(lock.try_read().is_some());
    /// #
    /// # })
    /// ```
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read()
    }

    /// Attempts to acquire a read lock.
    ///
    /// If a read lock could not be acquired at this time, then [`None`] is returned. Otherwise, a
    /// guard is returned that releases the lock when dropped.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(1);
    ///
    /// let n = lock.read().await;
    /// assert_eq!(*n, 1);
    ///
    /// assert!(lock.try_read().is_some());
    /// #
    /// # })
    /// ```
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.inner.try_read()
    }

    /// Acquires a write lock.
    ///
    /// Returns a guard that releases the lock when dropped.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(1);
    ///
    /// let mut n = lock.write().await;
    /// *n = 2;
    ///
    /// assert!(lock.try_read().is_none());
    /// #
    /// # })
    /// ```
    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.inner.write()
    }

    /// Attempts to acquire a write lock.
    ///
    /// If a write lock could not be acquired at this time, then [`None`] is returned. Otherwise, a
    /// guard is returned that releases the lock when dropped.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(1);
    ///
    /// let n = lock.read().await;
    /// assert_eq!(*n, 1);
    ///
    /// assert!(lock.try_write().is_none());
    /// #
    /// # })
    /// ```
    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        self.inner.try_write()
    }

    /// Consumes the lock, returning the underlying data.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::sync::RwLock;
    ///
    /// let lock = RwLock::new(10);
    /// assert_eq!(lock.into_inner(), 10);
    /// ```
    pub fn into_inner(self) -> T
    where
        T: Sized,
    {
        self.inner.into_inner()
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// Since this call borrows the lock mutably, no actual locking takes place -- the mutable
    /// borrow statically guarantees no locks exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::RwLock;
    ///
    /// let mut lock = RwLock::new(0);
    /// *lock.get_mut() = 10;
    /// assert_eq!(*lock.write().await, 10);
    /// #
    /// # })
    /// ```
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(val: T) -> RwLock<T> {
        RwLock::new(val)
    }
}

impl<T: ?Sized + Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

/// A guard that releases the read lock when dropped.
pub struct RwLockReadGuard<'a, T: ?Sized>(&'a RawRwLockReadGuard<T>);

unsafe impl<T: ?Sized + Send> Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.0.drop()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.deref()
    }
}

/// A guard that releases the write lock when dropped.
pub struct RwLockWriteGuard<'a, T: ?Sized>(&'a RwLock<T>);

unsafe impl<T: ?Sized + Send> Send for RwLockWriteGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T: ?Sized> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.0.state.store(0, Ordering::SeqCst);

        // Notify all blocked readers.
        if !self.0.read_wakers.notify_all() {
            // If there were no blocked readers, notify a blocked writer if none were notified
            // already.
            self.0.write_wakers.notify_any();
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
