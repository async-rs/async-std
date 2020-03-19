use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};

use crossbeam_utils::Backoff;

/// A simple spinlock.
#[derive(Debug)]
pub struct Spinlock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Spinlock<T> {}
unsafe impl<T: Send> Sync for Spinlock<T> {}

impl<T> Spinlock<T> {
    /// Returns a new spinlock initialized with `value`.
    pub const fn new(value: T) -> Spinlock<T> {
        Spinlock {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Locks the spinlock.
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        let backoff = Backoff::new();
        while self.locked.compare_and_swap(false, true, Ordering::Acquire) {
            backoff.snooze();
        }
        SpinlockGuard { parent: self }
    }

    /// Attempts to lock the spinlock.
    pub fn try_lock(&self) -> Option<SpinlockGuard<'_, T>> {
        if self.locked.swap(true, Ordering::Acquire) {
            None
        } else {
            Some(SpinlockGuard { parent: self })
        }
    }
}

/// A guard holding a spinlock locked.
#[derive(Debug)]
pub struct SpinlockGuard<'a, T> {
    parent: &'a Spinlock<T>,
}

unsafe impl<T: Send> Send for SpinlockGuard<'_, T> {}
unsafe impl<T: Sync> Sync for SpinlockGuard<'_, T> {}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.parent.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for SpinlockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.parent.value.get() }
    }
}

impl<'a, T> DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.parent.value.get() }
    }
}

#[test]
fn spinlock() {
    crate::task::block_on(async {
        use crate::sync::{Arc, Spinlock};
        use crate::task;

        let m = Arc::new(Spinlock::new(0));
        let mut tasks = vec![];

        for _ in 0..10 {
            let m = m.clone();
            tasks.push(task::spawn(async move {
                *m.lock() += 1;
            }));
        }

        for t in tasks {
            t.await;
        }
        assert_eq!(*m.lock(), 10);

    })
}
