use std::cell::UnsafeCell;
use std::error::Error;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use lazy_static::lazy_static;

use super::worker;
use crate::utils::abort_on_panic;
use std::future::Future;

/// Declares task-local values.
///
/// The macro wraps any number of static declarations and makes them task-local. Attributes and
/// visibility modifiers are allowed.
///
/// Each declared value is of the accessor type [`LocalKey`].
///
/// [`LocalKey`]: task/struct.LocalKey.html
///
/// # Examples
///
/// ```
/// #
/// use std::cell::Cell;
///
/// use async_std::task;
/// use async_std::prelude::*;
///
/// task_local! {
///     static VAL: Cell<u32> = Cell::new(5);
/// }
///
/// task::block_on(async {
///     let v = VAL.with(|c| c.get());
///     assert_eq!(v, 5);
/// });
/// ```
#[macro_export]
macro_rules! task_local {
    () => ();

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $(#[$attr])* $vis static $name: $crate::task::LocalKey<$t> = {
            #[inline]
            fn __init() -> $t {
                $init
            }

            $crate::task::LocalKey {
                __init,
                __key: ::std::sync::atomic::AtomicUsize::new(0),
            }
        };
    );

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::task_local!($(#[$attr])* $vis static $name: $t = $init);
        $crate::task_local!($($rest)*);
    );
}

/// The key for accessing a task-local value.
///
/// Every task-local value is lazily initialized on first access and destroyed when the task
/// completes.
#[derive(Debug)]
pub struct LocalKey<T: Send + 'static> {
    #[doc(hidden)]
    pub __init: fn() -> T,

    #[doc(hidden)]
    pub __key: AtomicUsize,
}

impl<T: Send + 'static> LocalKey<T> {
    /// Gets a reference to the task-local value with this key.
    ///
    /// The passed closure receives a reference to the task-local value.
    ///
    /// The task-local value will be lazily initialized if this task has not accessed it before.
    ///
    /// # Panics
    ///
    /// This function will panic if not called within the context of a task created by
    /// [`block_on`], [`spawn`], or [`Builder::spawn`].
    ///
    /// [`block_on`]: fn.block_on.html
    /// [`spawn`]: fn.spawn.html
    /// [`Builder::spawn`]: struct.Builder.html#method.spawn
    ///
    /// # Examples
    ///
    /// ```
    /// #
    /// use std::cell::Cell;
    ///
    /// use async_std::task;
    /// use async_std::prelude::*;
    ///
    /// task_local! {
    ///     static NUMBER: Cell<u32> = Cell::new(5);
    /// }
    ///
    /// task::block_on(async {
    ///     let v = NUMBER.with(|c| c.get());
    ///     assert_eq!(v, 5);
    /// });
    /// ```
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.try_with(f)
            .expect("`LocalKey::with` called outside the context of a task")
    }

    /// Attempts to get a reference to the task-local value with this key.
    ///
    /// The passed closure receives a reference to the task-local value.
    ///
    /// The task-local value will be lazily initialized if this task has not accessed it before.
    ///
    /// This function returns an error if not called within the context of a task created by
    /// [`block_on`], [`spawn`], or [`Builder::spawn`].
    ///
    /// [`block_on`]: fn.block_on.html
    /// [`spawn`]: fn.spawn.html
    /// [`Builder::spawn`]: struct.Builder.html#method.spawn
    ///
    /// # Examples
    ///
    /// ```
    /// #
    /// use std::cell::Cell;
    ///
    /// use async_std::task;
    /// use async_std::prelude::*;
    ///
    /// task_local! {
    ///     static VAL: Cell<u32> = Cell::new(5);
    /// }
    ///
    /// task::block_on(async {
    ///     let v = VAL.try_with(|c| c.get());
    ///     assert_eq!(v, Ok(5));
    /// });
    ///
    /// // Returns an error because not called within the context of a task.
    /// assert!(VAL.try_with(|c| c.get()).is_err());
    /// ```
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        worker::get_task(|task| unsafe {
            // Prepare the numeric key, initialization function, and the map of task-locals.
            let key = self.key();
            let init = || Box::new((self.__init)()) as Box<dyn Send>;
            let map = &task.metadata().local_map;

            // Get the value in the map of task-locals, or initialize and insert one.
            let value: *const dyn Send = map.get_or_insert(key, init);

            // Call the closure with the value passed as an argument.
            f(&*(value as *const T))
        })
        .ok_or(AccessError { _private: () })
    }

    /// Returns the numeric key associated with this task-local.
    #[inline]
    fn key(&self) -> usize {
        #[cold]
        fn init(key: &AtomicUsize) -> usize {
            lazy_static! {
                static ref COUNTER: Mutex<usize> = Mutex::new(1);
            }

            let mut counter = COUNTER.lock().unwrap();
            let prev = key.compare_and_swap(0, *counter, Ordering::AcqRel);

            if prev == 0 {
                *counter += 1;
                *counter - 1
            } else {
                prev
            }
        }

        let key = self.__key.load(Ordering::Acquire);
        if key == 0 { init(&self.__key) } else { key }
    }
}

/// An error returned by [`LocalKey::try_with`].
///
/// [`LocalKey::try_with`]: struct.LocalKey.html#method.try_with
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct AccessError {
    _private: (),
}

impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessError").finish()
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "already destroyed or called outside the context of a task".fmt(f)
    }
}

impl Error for AccessError {}

/// A map that holds task-locals.
pub(crate) struct Map {
    /// A list of `(key, value)` entries sorted by the key.
    entries: UnsafeCell<Vec<(usize, Box<dyn Send>)>>,
}

impl Map {
    /// Creates an empty map of task-locals.
    pub fn new() -> Map {
        Map {
            entries: UnsafeCell::new(Vec::new()),
        }
    }

    /// Returns a thread-local value associated with `key` or inserts one constructed by `init`.
    #[inline]
    pub fn get_or_insert(&self, key: usize, init: impl FnOnce() -> Box<dyn Send>) -> &dyn Send {
        let entries = unsafe { &mut *self.entries.get() };

        let index = match entries.binary_search_by_key(&key, |e| e.0) {
            Ok(i) => i,
            Err(i) => {
                entries.insert(i, (key, init()));
                i
            }
        };

        &*entries[index].1
    }

    /// Clears the map and drops all task-locals.
    pub fn clear(&self) {
        let entries = unsafe { &mut *self.entries.get() };
        entries.clear();
    }
}

// Wrap the future into one that drops task-local variables on exit.
pub(crate) unsafe fn add_finalizer<T>(f: impl Future<Output = T>) -> impl Future<Output = T> {
    async move {
        let res = f.await;

        // Abort on panic because thread-local variables behave the same way.
        abort_on_panic(|| worker::get_task(|task| task.metadata().local_map.clear()));

        res
    }
}
