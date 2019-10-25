//! A common utility for building synchronization primitives.
//!
//! When an async operation is blocked, it needs to register itself somewhere so that it can be
//! notified later on. Additionally, operations may be cancellable and we need to make sure
//! notifications are not lost if an operation gets cancelled just after picking up a notification.
//!
//! The `Registry` type helps with registering and notifying such async operations.

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

/// A list of blocked operations.
struct Blocked {
    /// A list of registered operations.
    ///
    /// Each entry has a waker associated with the task that is executing the operation. If the
    /// waker is set to `None`, that means the task has been woken up but hasn't removed itself
    /// from the registry yet.
    entries: Slab<Option<Waker>>,

    /// The number of entries that are `None`.
    none_count: usize,
}

/// A registry of blocked async operations.
pub struct Registry {
    /// Holds three bits: `LOCKED`, `NOTIFY_ONE`, and `NOTIFY_ALL`.
    flag: AtomicUsize,

    /// A list of registered blocked operations.
    blocked: UnsafeCell<Blocked>,
}

impl Registry {
    /// Creates a new registry.
    #[inline]
    pub fn new() -> Registry {
        Registry {
            flag: AtomicUsize::new(0),
            blocked: UnsafeCell::new(Blocked {
                entries: Slab::new(),
                none_count: 0,
            }),
        }
    }

    /// Registers a blocked operation and returns a key associated with it.
    pub fn register(&self, cx: &Context<'_>) -> usize {
        let w = cx.waker().clone();
        self.lock().entries.insert(Some(w))
    }

    /// Re-registers a blocked operation by filling in its waker.
    pub fn reregister(&self, key: usize, cx: &Context<'_>) {
        let mut blocked = self.lock();

        match &mut blocked.entries[key] {
            None => {
                // Fill in the waker.
                let w = cx.waker().clone();
                blocked.entries[key] = Some(w);
                blocked.none_count -= 1;
            }
            Some(w) => {
                // Replace the waker if the existing one is different.
                if !w.will_wake(cx.waker()) {
                    *w = cx.waker().clone();
                }
            }
        }
    }

    /// Unregisters a completed operation.
    pub fn complete(&self, key: usize) {
        let mut blocked = self.lock();
        if blocked.entries.remove(key).is_none() {
            blocked.none_count -= 1;
        }
    }

    /// Unregisters a cancelled operation.
    pub fn cancel(&self, key: usize) {
        let mut blocked = self.lock();
        if blocked.entries.remove(key).is_none() {
            blocked.none_count -= 1;

            // The operation was cancelled and notified so notify another operation instead.
            if let Some((_, opt_waker)) = blocked.entries.iter_mut().next() {
                if let Some(w) = opt_waker.take() {
                    w.wake();
                    blocked.none_count += 1;
                }
            }
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
    // TODO: Delete this attribute when `crate::sync::channel` is stabilized.
    #[cfg(feature = "unstable")]
    #[inline]
    pub fn notify_all(&self) {
        // Use `SeqCst` ordering to synchronize with `Lock::drop()`.
        if self.flag.load(Ordering::SeqCst) & NOTIFY_ALL != 0 {
            self.notify(true);
        }
    }

    /// Notifies registered operations, either one or all of them.
    fn notify(&self, all: bool) {
        let mut blocked = &mut *self.lock();

        for (_, opt_waker) in blocked.entries.iter_mut() {
            // If there is no waker in this entry, that means it was already woken.
            if let Some(w) = opt_waker.take() {
                w.wake();
                blocked.none_count += 1;
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
        Lock { registry: self }
    }
}

/// Guard holding a registry locked.
struct Lock<'a> {
    registry: &'a Registry,
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

        // Use `SeqCst` ordering to synchronize with `Registry::lock_to_notify()`.
        self.registry.flag.store(flag, Ordering::SeqCst);
    }
}

impl Deref for Lock<'_> {
    type Target = Blocked;

    #[inline]
    fn deref(&self) -> &Blocked {
        unsafe { &*self.registry.blocked.get() }
    }
}

impl DerefMut for Lock<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Blocked {
        unsafe { &mut *self.registry.blocked.get() }
    }
}
