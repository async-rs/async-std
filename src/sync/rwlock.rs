use std::cell::UnsafeCell;
use std::fmt;
use std::isize;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::process;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::sync::WakerSet;
use crate::task::{Context, Poll};

/// Set if a write lock is held.
#[allow(clippy::identity_op)]
const WRITE_LOCK: usize = 1 << 0;

/// The value of a single blocked read contributing to the read count.
const ONE_READ: usize = 1 << 1;

/// The bits in which the read count is stored.
const READ_COUNT_MASK: usize = !(ONE_READ - 1);

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
    state: AtomicUsize,
    read_wakers: WakerSet,
    write_wakers: WakerSet,
    value: UnsafeCell<T>,
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
        RwLock {
            state: AtomicUsize::new(0),
            read_wakers: WakerSet::new(),
            write_wakers: WakerSet::new(),
            value: UnsafeCell::new(t),
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
        pub struct ReadFuture<'a, T: ?Sized> {
            lock: &'a RwLock<T>,
            opt_key: Option<usize>,
        }

        impl<'a, T: ?Sized> Future for ReadFuture<'a, T> {
            type Output = RwLockReadGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                loop {
                    // If the current task is in the set, remove it.
                    if let Some(key) = self.opt_key.take() {
                        self.lock.read_wakers.remove(key);
                    }

                    // Try acquiring a read lock.
                    match self.lock.try_read() {
                        Some(guard) => return Poll::Ready(guard),
                        None => {
                            // Insert this lock operation.
                            self.opt_key = Some(self.lock.read_wakers.insert(cx));

                            // If the lock is still acquired for writing, return.
                            if self.lock.state.load(Ordering::SeqCst) & WRITE_LOCK != 0 {
                                return Poll::Pending;
                            }
                        }
                    }
                }
            }
        }

        impl<T: ?Sized> Drop for ReadFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
                if let Some(key) = self.opt_key {
                    self.lock.read_wakers.cancel(key);

                    // If there are no active readers, notify a blocked writer if none were
                    // notified already.
                    if self.lock.state.load(Ordering::SeqCst) & READ_COUNT_MASK == 0 {
                        self.lock.write_wakers.notify_any();
                    }
                }
            }
        }

        ReadFuture {
            lock: self,
            opt_key: None,
        }
        .await
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
        let mut state = self.state.load(Ordering::SeqCst);

        loop {
            // If a write lock is currently held, then a read lock cannot be acquired.
            if state & WRITE_LOCK != 0 {
                return None;
            }

            // Make sure the number of readers doesn't overflow.
            if state > isize::MAX as usize {
                process::abort();
            }

            // Increment the number of active reads.
            match self.state.compare_exchange_weak(
                state,
                state + ONE_READ,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Some(RwLockReadGuard(self)),
                Err(s) => state = s,
            }
        }
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
        pub struct WriteFuture<'a, T: ?Sized> {
            lock: &'a RwLock<T>,
            opt_key: Option<usize>,
        }

        impl<'a, T: ?Sized> Future for WriteFuture<'a, T> {
            type Output = RwLockWriteGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                loop {
                    // If the current task is in the set, remove it.
                    if let Some(key) = self.opt_key.take() {
                        self.lock.write_wakers.remove(key);
                    }

                    // Try acquiring a write lock.
                    match self.lock.try_write() {
                        Some(guard) => return Poll::Ready(guard),
                        None => {
                            // Insert this lock operation.
                            self.opt_key = Some(self.lock.write_wakers.insert(cx));

                            // If the lock is still acquired for reading or writing, return.
                            if self.lock.state.load(Ordering::SeqCst) != 0 {
                                return Poll::Pending;
                            }
                        }
                    }
                }
            }
        }

        impl<T: ?Sized> Drop for WriteFuture<'_, T> {
            fn drop(&mut self) {
                // If the current task is still in the set, that means it is being cancelled now.
                if let Some(key) = self.opt_key {
                    if !self.lock.write_wakers.cancel(key) {
                        // If no other blocked reader was notified, notify all readers.
                        self.lock.read_wakers.notify_all();
                    }
                }
            }
        }

        WriteFuture {
            lock: self,
            opt_key: None,
        }
        .await
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
        if self.state.compare_and_swap(0, WRITE_LOCK, Ordering::SeqCst) == 0 {
            Some(RwLockWriteGuard(self))
        } else {
            None
        }
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
    pub fn into_inner(self) -> T where T: Sized {
        self.value.into_inner()
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
        unsafe { &mut *self.value.get() }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Locked;
        impl fmt::Debug for Locked {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("<locked>")
            }
        }

        match self.try_read() {
            None => f.debug_struct("RwLock").field("data", &Locked).finish(),
            Some(guard) => f.debug_struct("RwLock").field("data", &&*guard).finish(),
        }
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
pub struct RwLockReadGuard<'a, T: ?Sized>(&'a RwLock<T>);

unsafe impl<T: ?Sized + Send> Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: ?Sized + Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T: ?Sized> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let state = self.0.state.fetch_sub(ONE_READ, Ordering::SeqCst);

        // If this was the last reader, notify a blocked writer if none were notified already.
        if state & READ_COUNT_MASK == ONE_READ {
            self.0.write_wakers.notify_any();
        }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
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
