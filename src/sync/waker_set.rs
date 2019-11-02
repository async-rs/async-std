//! A common utility for building synchronization primitives.
//!
//! When an async operation is blocked, it needs to register itself somewhere so that it can be
//! notified later on. The `WakerSet` type helps with keeping track of such async operations and
//! notifying them when they may make progress.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_utils::Backoff;
use slab::Slab;

use crate::task::{Context, Waker};

/// Set when the entry list is locked.
#[allow(clippy::identity_op)]
const LOCKED: usize = 1 << 0;

/// Set when there are tasks for `notify_one()` to wake.
const NOTIFY_ONE: usize = 1 << 1;

/// Set when there are tasks for `notify_all()` to wake.
const NOTIFY_ALL: usize = 1 << 2;

/// Inner representation of `WakerSet`.
struct Inner {
    /// A list of entries in the set.
    ///
    /// Each entry has an optional waker associated with the task that is executing the operation.
    /// If the waker is set to `None`, that means the task has been woken up but hasn't removed
    /// itself from the `WakerSet` yet.
    ///
    /// The key of each entry is its index in the `Slab`.
    entries: Slab<Option<Waker>>,

    /// The number of entries that have the waker set to `None`.
    none_count: usize,
}

/// A set holding wakers.
pub struct WakerSet {
    /// Holds three bits: `LOCKED`, `NOTIFY_ONE`, and `NOTIFY_ALL`.
    flag: AtomicUsize,

    /// A set holding wakers.
    inner: UnsafeCell<Inner>,
}

impl WakerSet {
    /// Creates a new `WakerSet`.
    #[inline]
    pub fn new() -> WakerSet {
        WakerSet {
            flag: AtomicUsize::new(0),
            inner: UnsafeCell::new(Inner {
                entries: Slab::new(),
                none_count: 0,
            }),
        }
    }

    /// Inserts a waker for a blocked operation and returns a key associated with it.
    pub fn insert(&self, cx: &Context<'_>) -> usize {
        let w = cx.waker().clone();
        self.lock().entries.insert(Some(w))
    }

    /// Updates the waker of a previously inserted entry.
    pub fn update(&self, key: usize, cx: &Context<'_>) {
        let mut inner = self.lock();

        match &mut inner.entries[key] {
            None => {
                // Fill in the waker.
                let w = cx.waker().clone();
                inner.entries[key] = Some(w);
                inner.none_count -= 1;
            }
            Some(w) => {
                // Replace the waker if the existing one is different.
                if !w.will_wake(cx.waker()) {
                    *w = cx.waker().clone();
                }
            }
        }
    }

    /// Removes the waker of a completed operation.
    pub fn complete(&self, key: usize) {
        let mut inner = self.lock();
        if inner.entries.remove(key).is_none() {
            inner.none_count -= 1;
        }
    }

    /// Removes the waker of a cancelled operation.
    ///
    /// Returns `true` if another blocked operation from the set was notified.
    pub fn cancel(&self, key: usize) -> bool {
        let mut inner = self.lock();

        if inner.entries.remove(key).is_none() {
            inner.none_count -= 1;

            // The operation was cancelled and notified so notify another operation instead.
            if let Some((_, opt_waker)) = inner.entries.iter_mut().next() {
                // If there is no waker in this entry, that means it was already woken.
                if let Some(w) = opt_waker.take() {
                    w.wake();
                    inner.none_count += 1;
                }
                return true;
            }
        }

        false
    }

    /// Notifies one blocked operation.
    ///
    /// Returns `true` if an operation was notified.
    #[inline]
    pub fn notify_one(&self) -> bool {
        // Use `SeqCst` ordering to synchronize with `Lock::drop()`.
        if self.flag.load(Ordering::SeqCst) & NOTIFY_ONE != 0 {
            self.notify(false)
        } else {
            false
        }
    }

    /// Notifies all blocked operations.
    ///
    /// Returns `true` if at least one operation was notified.
    #[inline]
    pub fn notify_all(&self) -> bool {
        // Use `SeqCst` ordering to synchronize with `Lock::drop()`.
        if self.flag.load(Ordering::SeqCst) & NOTIFY_ALL != 0 {
            self.notify(true)
        } else {
            false
        }
    }

    /// Notifies blocked operations, either one or all of them.
    ///
    /// Returns `true` if at least one operation was notified.
    fn notify(&self, all: bool) -> bool {
        let mut inner = &mut *self.lock();
        let mut notified = false;

        for (_, opt_waker) in inner.entries.iter_mut() {
            // If there is no waker in this entry, that means it was already woken.
            if let Some(w) = opt_waker.take() {
                w.wake();
                inner.none_count += 1;
            }

            notified = true;

            if !all {
                break;
            }
        }

        notified
    }

    /// Locks the list of entries.
    #[cold]
    fn lock(&self) -> Lock<'_> {
        let backoff = Backoff::new();
        while self.flag.fetch_or(LOCKED, Ordering::Acquire) & LOCKED != 0 {
            backoff.snooze();
        }
        Lock { waker_set: self }
    }
}

/// A guard holding a `WakerSet` locked.
struct Lock<'a> {
    waker_set: &'a WakerSet,
}

impl Drop for Lock<'_> {
    #[inline]
    fn drop(&mut self) {
        let mut flag = 0;

        // If there is at least one entry and all are `Some`, then `notify_one()` has work to do.
        if !self.entries.is_empty() && self.none_count == 0 {
            flag |= NOTIFY_ONE;
        }

        // If there is at least one `Some` entry, then `notify_all()` has work to do.
        if self.entries.len() - self.none_count > 0 {
            flag |= NOTIFY_ALL;
        }

        // Use `SeqCst` ordering to synchronize with `WakerSet::lock_to_notify()`.
        self.waker_set.flag.store(flag, Ordering::SeqCst);
    }
}

impl Deref for Lock<'_> {
    type Target = Inner;

    #[inline]
    fn deref(&self) -> &Inner {
        unsafe { &*self.waker_set.inner.get() }
    }
}

impl DerefMut for Lock<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Inner {
        unsafe { &mut *self.waker_set.inner.get() }
    }
}
