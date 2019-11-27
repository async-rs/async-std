use std::cell::Cell;
use std::fmt;
use std::mem::ManuallyDrop;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Arc;

use crate::task::{LocalsMap, TaskId};
use crate::utils::abort_on_panic;

thread_local! {
    /// A pointer to the currently running task.
    static CURRENT: Cell<*const Task> = Cell::new(ptr::null_mut());
}

/// The inner representation of a task handle.
struct Inner {
    /// The task ID.
    id: TaskId,

    /// The optional task name.
    name: Option<Box<str>>,

    /// The map holding task-local values.
    locals: LocalsMap,
}

impl Inner {
    #[inline]
    fn new(name: Option<String>) -> Inner {
        Inner {
            id: TaskId::generate(),
            name: name.map(String::into_boxed_str),
            locals: LocalsMap::new(),
        }
    }
}

/// A handle to a task.
pub struct Task {
    /// The inner representation.
    ///
    /// This pointer is lazily initialized on first use. In most cases, the inner representation is
    /// never touched and therefore we don't allocate it unless it's really needed.
    inner: AtomicPtr<Inner>,
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    /// Creates a new task handle.
    ///
    /// If the task is unnamed, the inner representation of the task will be lazily allocated on
    /// demand.
    #[inline]
    pub(crate) fn new(name: Option<String>) -> Task {
        let inner = match name {
            None => AtomicPtr::default(),
            Some(name) => {
                let raw = Arc::into_raw(Arc::new(Inner::new(Some(name))));
                AtomicPtr::new(raw as *mut Inner)
            }
        };
        Task { inner }
    }

    /// Gets the task's unique identifier.
    #[inline]
    pub fn id(&self) -> TaskId {
        self.inner().id
    }

    /// Returns the name of this task.
    ///
    /// The name is configured by [`Builder::name`] before spawning.
    ///
    /// [`Builder::name`]: struct.Builder.html#method.name
    pub fn name(&self) -> Option<&str> {
        self.inner().name.as_ref().map(|s| &**s)
    }

    /// Returns the map holding task-local values.
    pub(crate) fn locals(&self) -> &LocalsMap {
        &self.inner().locals
    }

    /// Drops all task-local values.
    ///
    /// This method is only safe to call at the end of the task.
    #[inline]
    pub(crate) unsafe fn drop_locals(&self) {
        let raw = self.inner.load(Ordering::Acquire);
        if let Some(inner) = raw.as_mut() {
            // Abort the process if dropping task-locals panics.
            abort_on_panic(|| {
                inner.locals.clear();
            });
        }
    }

    /// Returns the inner representation, initializing it on first use.
    fn inner(&self) -> &Inner {
        loop {
            let raw = self.inner.load(Ordering::Acquire);
            if !raw.is_null() {
                return unsafe { &*raw };
            }

            let new = Arc::into_raw(Arc::new(Inner::new(None))) as *mut Inner;
            if self.inner.compare_and_swap(raw, new, Ordering::AcqRel) != raw {
                unsafe {
                    drop(Arc::from_raw(new));
                }
            }
        }
    }

    /// Set a reference to the current task.
    pub(crate) unsafe fn set_current<F, R>(task: *const Task, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        CURRENT.with(|current| {
            let old_task = current.replace(task);
            defer! {
                current.set(old_task);
            }
            f()
        })
    }

    /// Gets a reference to the current task.
    pub(crate) fn get_current<F, R>(f: F) -> Option<R>
    where
        F: FnOnce(&Task) -> R,
    {
        let res = CURRENT.try_with(|current| unsafe { current.get().as_ref().map(f) });
        match res {
            Ok(Some(val)) => Some(val),
            Ok(None) | Err(_) => None,
        }
    }
}

impl Drop for Task {
    fn drop(&mut self) {
        // Deallocate the inner representation if it was initialized.
        let raw = *self.inner.get_mut();
        if !raw.is_null() {
            unsafe {
                drop(Arc::from_raw(raw));
            }
        }
    }
}

impl Clone for Task {
    fn clone(&self) -> Task {
        // We need to make sure the inner representation is initialized now so that this instance
        // and the clone have raw pointers that point to the same `Arc<Inner>`.
        let arc = unsafe { ManuallyDrop::new(Arc::from_raw(self.inner())) };
        let raw = Arc::into_raw(Arc::clone(&arc));
        Task {
            inner: AtomicPtr::new(raw as *mut Inner),
        }
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id())
            .field("name", &self.name())
            .finish()
    }
}
