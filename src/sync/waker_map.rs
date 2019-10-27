//! A common utility for building synchronization primitives.
//!
//! When an async operation is blocked, it needs to register itself somewhere so that it can be
//! notified later on. The `WakerMap` type helps with keeping track of such async operations and
//! notifying them when they may make progress.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_utils::Backoff;
use slab::Slab;

use crate::task::{Context, Waker};

/// Set when the entry list is locked.
const LOCKED: usize = 1 << 0;

/// Set when there are tasks for `notify_one()` to wake.
const NOTIFY_ONE: usize = 1 << 1;

/// Set when there are tasks for `notify_all()` to wake.
const NOTIFY_ALL: usize = 1 << 2;

/// Inner representation of `WakerMap`.
struct Inner {
    /// A list of entries in the map.
    ///
    /// Each entry has an optional waker associated with the task that is executing the operation.
    /// If the waker is set to `None`, that means the task has been woken up but hasn't removed
    /// itself from the `WakerMap` yet.
    ///
    /// The key of each entry is its index in the `Slab`.
    entries: Slab<Option<Waker>>,

    /// The number of entries that have the waker set to `None`.
    none_count: usize,
}

/// A map holding wakers.
pub struct WakerMap {
    /// Holds three bits: `LOCKED`, `NOTIFY_ONE`, and `NOTIFY_ALL`.
    flag: AtomicUsize,

    /// A map holding wakers.
    inner: UnsafeCell<Inner>,
}

impl WakerMap {
    /// Creates a new `WakerMap`.
    #[inline]
    pub fn new() -> WakerMap {
        WakerMap {
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

    /// Removes a waker.
    pub fn remove(&self, key: usize) {
        let mut inner = self.lock();
        if inner.entries.remove(key).is_none() {
            inner.none_count -= 1;
        }
    }

    /// Notifies one blocked operation.
    #[inline]
    pub fn notify_one(&self) {
        // Use `SeqCst` ordering to synchronize with `Lock::drop()`.
        if self.flag.load(Ordering::SeqCst) & NOTIFY_ONE != 0 {
            self.notify(false);
        }
    }

    /// Notifies all blocked operations.
    // TODO: Delete this attribute when `crate::sync::channel()` is stabilized.
    #[cfg(feature = "unstable")]
    #[inline]
    pub fn notify_all(&self) {
        // Use `SeqCst` ordering to synchronize with `Lock::drop()`.
        if self.flag.load(Ordering::SeqCst) & NOTIFY_ALL != 0 {
            self.notify(true);
        }
    }

    /// Notifies blocked operations, either one or all of them.
    fn notify(&self, all: bool) {
        let mut inner = &mut *self.lock();

        for (_, opt_waker) in inner.entries.iter_mut() {
            // If there is no waker in this entry, that means it was already woken.
            if let Some(w) = opt_waker.take() {
                w.wake();
                inner.none_count += 1;
            }
            if !all {
                break;
            }
        }
    }

    /// Locks the list of entries.
    #[cold]
    fn lock(&self) -> Lock<'_> {
        let backoff = Backoff::new();
        while self.flag.fetch_or(LOCKED, Ordering::Acquire) & LOCKED != 0 {
            backoff.snooze();
        }
        Lock { waker_map: self }
    }
}

/// A guard holding a `WakerMap` locked.
struct Lock<'a> {
    waker_map: &'a WakerMap,
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

        // Use `SeqCst` ordering to synchronize with `WakerMap::lock_to_notify()`.
        self.waker_map.flag.store(flag, Ordering::SeqCst);
    }
}

impl Deref for Lock<'_> {
    type Target = Inner;

    #[inline]
    fn deref(&self) -> &Inner {
        unsafe { &*self.waker_map.inner.get() }
    }
}

impl DerefMut for Lock<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Inner {
        unsafe { &mut *self.waker_map.inner.get() }
    }
}
