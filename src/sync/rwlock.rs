use std::cell::UnsafeCell;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use slab::Slab;

use crate::future::Future;
use crate::task::{Context, Poll, Waker};

/// Set if a write lock is held.
const WRITE_LOCK: usize = 1;

/// Set if there are read operations blocked on the lock.
const BLOCKED_READS: usize = 1 << 1;

/// Set if there are write operations blocked on the lock.
const BLOCKED_WRITES: usize = 1 << 2;

/// The value of a single blocked read contributing to the read count.
const ONE_READ: usize = 1 << 3;

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
pub struct RwLock<T> {
    state: AtomicUsize,
    reads: std::sync::Mutex<Slab<Option<Waker>>>,
    writes: std::sync::Mutex<Slab<Option<Waker>>>,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for RwLock<T> {}
unsafe impl<T: Send> Sync for RwLock<T> {}

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
            reads: std::sync::Mutex::new(Slab::new()),
            writes: std::sync::Mutex::new(Slab::new()),
            value: UnsafeCell::new(t),
        }
    }

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
        pub struct LockFuture<'a, T> {
            lock: &'a RwLock<T>,
            opt_key: Option<usize>,
            acquired: bool,
        }

        impl<'a, T> Future for LockFuture<'a, T> {
            type Output = RwLockReadGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.lock.try_read() {
                    Some(guard) => {
                        self.acquired = true;
                        Poll::Ready(guard)
                    }
                    None => {
                        let mut reads = self.lock.reads.lock().unwrap();

                        // Register the current task.
                        match self.opt_key {
                            None => {
                                // Insert a new entry into the list of blocked reads.
                                let w = cx.waker().clone();
                                let key = reads.insert(Some(w));
                                self.opt_key = Some(key);

                                if reads.len() == 1 {
                                    self.lock.state.fetch_or(BLOCKED_READS, Ordering::Relaxed);
                                }
                            }
                            Some(key) => {
                                // There is already an entry in the list of blocked reads. Just
                                // reset the waker if it was removed.
                                if reads[key].is_none() {
                                    let w = cx.waker().clone();
                                    reads[key] = Some(w);
                                }
                            }
                        }

                        // Try locking again because it's possible the lock got unlocked just
                        // before the current task was registered as a blocked task.
                        match self.lock.try_read() {
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
                    let mut reads = self.lock.reads.lock().unwrap();
                    let opt_waker = reads.remove(key);

                    if reads.is_empty() {
                        self.lock.state.fetch_and(!BLOCKED_READS, Ordering::Relaxed);
                    }

                    if opt_waker.is_none() {
                        // We were awoken. Wake up another blocked read.
                        if let Some((_, opt_waker)) = reads.iter_mut().next() {
                            if let Some(w) = opt_waker.take() {
                                w.wake();
                                return;
                            }
                        }
                        drop(reads);

                        if !self.acquired {
                            // We didn't acquire the lock and didn't wake another blocked read.
                            // Wake a blocked write instead.
                            let mut writes = self.lock.writes.lock().unwrap();
                            if let Some((_, opt_waker)) = writes.iter_mut().next() {
                                if let Some(w) = opt_waker.take() {
                                    w.wake();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }

        LockFuture {
            lock: self,
            opt_key: None,
            acquired: false,
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
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            // If a write lock is currently held, then a read lock cannot be acquired.
            if state & WRITE_LOCK != 0 {
                return None;
            }

            // Increment the number of active reads.
            match self.state.compare_exchange_weak(
                state,
                state + ONE_READ,
                Ordering::AcqRel,
                Ordering::Acquire,
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
        pub struct LockFuture<'a, T> {
            lock: &'a RwLock<T>,
            opt_key: Option<usize>,
            acquired: bool,
        }

        impl<'a, T> Future for LockFuture<'a, T> {
            type Output = RwLockWriteGuard<'a, T>;

            fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                match self.lock.try_write() {
                    Some(guard) => {
                        self.acquired = true;
                        Poll::Ready(guard)
                    }
                    None => {
                        let mut writes = self.lock.writes.lock().unwrap();

                        // Register the current task.
                        match self.opt_key {
                            None => {
                                // Insert a new entry into the list of blocked writes.
                                let w = cx.waker().clone();
                                let key = writes.insert(Some(w));
                                self.opt_key = Some(key);

                                if writes.len() == 1 {
                                    self.lock.state.fetch_or(BLOCKED_WRITES, Ordering::Relaxed);
                                }
                            }
                            Some(key) => {
                                // There is already an entry in the list of blocked writes. Just
                                // reset the waker if it was removed.
                                if writes[key].is_none() {
                                    let w = cx.waker().clone();
                                    writes[key] = Some(w);
                                }
                            }
                        }

                        // Try locking again because it's possible the lock got unlocked just
                        // before the current task was registered as a blocked task.
                        match self.lock.try_write() {
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
                    let mut writes = self.lock.writes.lock().unwrap();
                    let opt_waker = writes.remove(key);

                    if writes.is_empty() {
                        self.lock
                            .state
                            .fetch_and(!BLOCKED_WRITES, Ordering::Relaxed);
                    }

                    if opt_waker.is_none() && !self.acquired {
                        // We were awoken but didn't acquire the lock. Wake up another write.
                        if let Some((_, opt_waker)) = writes.iter_mut().next() {
                            if let Some(w) = opt_waker.take() {
                                w.wake();
                                return;
                            }
                        }
                        drop(writes);

                        // There are no blocked writes. Wake a blocked read instead.
                        let mut reads = self.lock.reads.lock().unwrap();
                        if let Some((_, opt_waker)) = reads.iter_mut().next() {
                            if let Some(w) = opt_waker.take() {
                                w.wake();
                                return;
                            }
                        }
                    }
                }
            }
        }

        LockFuture {
            lock: self,
            opt_key: None,
            acquired: false,
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
        let mut state = self.state.load(Ordering::Acquire);

        loop {
            // If any kind of lock is currently held, then a write lock cannot be acquired.
            if state & (WRITE_LOCK | READ_COUNT_MASK) != 0 {
                return None;
            }

            // Set the write lock.
            match self.state.compare_exchange_weak(
                state,
                state | WRITE_LOCK,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Some(RwLockWriteGuard(self)),
                Err(s) => state = s,
            }
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
    pub fn into_inner(self) -> T {
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

impl<T: fmt::Debug> fmt::Debug for RwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_read() {
            None => {
                struct LockedPlaceholder;
                impl fmt::Debug for LockedPlaceholder {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("<locked>")
                    }
                }
                f.debug_struct("RwLock")
                    .field("data", &LockedPlaceholder)
                    .finish()
            }
            Some(guard) => f.debug_struct("RwLock").field("data", &&*guard).finish(),
        }
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(val: T) -> RwLock<T> {
        RwLock::new(val)
    }
}

impl<T: Default> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

/// A guard that releases the read lock when dropped.
pub struct RwLockReadGuard<'a, T>(&'a RwLock<T>);

unsafe impl<T: Send> Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: Sync> Sync for RwLockReadGuard<'_, T> {}

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        let state = self.0.state.fetch_sub(ONE_READ, Ordering::AcqRel);

        // If this was the last read and there are blocked writes, wake one of them up.
        if (state & READ_COUNT_MASK) == ONE_READ && state & BLOCKED_WRITES != 0 {
            let mut writes = self.0.writes.lock().unwrap();

            if let Some((_, opt_waker)) = writes.iter_mut().next() {
                // If there is no waker in this entry, that means it was already woken.
                if let Some(w) = opt_waker.take() {
                    w.wake();
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for RwLockReadGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

/// A guard that releases the write lock when dropped.
pub struct RwLockWriteGuard<'a, T>(&'a RwLock<T>);

unsafe impl<T: Send> Send for RwLockWriteGuard<'_, T> {}
unsafe impl<T: Sync> Sync for RwLockWriteGuard<'_, T> {}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        let state = self.0.state.fetch_and(!WRITE_LOCK, Ordering::AcqRel);

        let mut guard = None;

        // Check if there are any blocked reads or writes.
        if state & BLOCKED_READS != 0 {
            guard = Some(self.0.reads.lock().unwrap());
        } else if state & BLOCKED_WRITES != 0 {
            guard = Some(self.0.writes.lock().unwrap());
        }

        // Wake up a single blocked task.
        if let Some(mut guard) = guard {
            if let Some((_, opt_waker)) = guard.iter_mut().next() {
                // If there is no waker in this entry, that means it was already woken.
                if let Some(w) = opt_waker.take() {
                    w.wake();
                }
            }
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: fmt::Display> fmt::Display for RwLockWriteGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (**self).fmt(f)
    }
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.value.get() }
    }
}
