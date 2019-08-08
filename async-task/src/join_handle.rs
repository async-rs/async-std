use std::fmt;
use std::future::Future;
use std::marker::{PhantomData, Unpin};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::Ordering;
use std::task::{Context, Poll};

use crate::header::Header;
use crate::state::*;
use crate::utils::abort_on_panic;

/// A handle that awaits the result of a task.
///
/// If the task has completed with `value`, the handle returns it as `Some(value)`. If the task was
/// cancelled or has panicked, the handle returns `None`. Otherwise, the handle has to wait until
/// the task completes, panics, or gets cancelled.
///
/// # Examples
///
/// ```
/// #![feature(async_await)]
///
/// use crossbeam::channel;
/// use futures::executor;
///
/// // The future inside the task.
/// let future = async { 1 + 2 };
///
/// // If the task gets woken, it will be sent into this channel.
/// let (s, r) = channel::unbounded();
/// let schedule = move |task| s.send(task).unwrap();
///
/// // Create a task with the future and the schedule function.
/// let (task, handle) = async_task::spawn(future, schedule, ());
///
/// // Run the task. In this example, it will complete after a single run.
/// task.run();
/// assert!(r.is_empty());
///
/// // Await the result of the task.
/// let result = executor::block_on(handle);
/// assert_eq!(result, Some(3));
/// ```
pub struct JoinHandle<R, T> {
    /// A raw task pointer.
    pub(crate) raw_task: NonNull<()>,

    /// A marker capturing the generic type `R`.
    pub(crate) _marker: PhantomData<(R, T)>,
}

unsafe impl<R, T> Send for JoinHandle<R, T> {}
unsafe impl<R, T> Sync for JoinHandle<R, T> {}

impl<R, T> Unpin for JoinHandle<R, T> {}

impl<R, T> JoinHandle<R, T> {
    /// Cancels the task.
    ///
    /// When cancelled, the task won't be scheduled again even if a [`Waker`] wakes it. An attempt
    /// to run it won't do anything. And if it's completed, awaiting its result evaluates to
    /// `None`.
    ///
    /// [`Waker`]: https://doc.rust-lang.org/std/task/struct.Waker.html
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use crossbeam::channel;
    /// use futures::executor;
    ///
    /// // The future inside the task.
    /// let future = async { 1 + 2 };
    ///
    /// // If the task gets woken, it will be sent into this channel.
    /// let (s, r) = channel::unbounded();
    /// let schedule = move |task| s.send(task).unwrap();
    ///
    /// // Create a task with the future and the schedule function.
    /// let (task, handle) = async_task::spawn(future, schedule, ());
    ///
    /// // Cancel the task.
    /// handle.cancel();
    ///
    /// // Running a cancelled task does nothing.
    /// task.run();
    ///
    /// // Await the result of the task.
    /// let result = executor::block_on(handle);
    /// assert_eq!(result, None);
    /// ```
    pub fn cancel(&self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);

            loop {
                // If the task has been completed or closed, it can't be cancelled.
                if state & (COMPLETED | CLOSED) != 0 {
                    break;
                }

                // If the task is not scheduled nor running, we'll need to schedule it.
                let new = if state & (SCHEDULED | RUNNING) == 0 {
                    (state | SCHEDULED | CLOSED) + REFERENCE
                } else {
                    state | CLOSED
                };

                // Mark the task as closed.
                match (*header).state.compare_exchange_weak(
                    state,
                    new,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // If the task is not scheduled nor running, schedule it so that its future
                        // gets dropped by the executor.
                        if state & (SCHEDULED | RUNNING) == 0 {
                            ((*header).vtable.schedule)(ptr);
                        }

                        // Notify the awaiter that the task has been closed.
                        if state & AWAITER != 0 {
                            (*header).notify();
                        }

                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    /// Returns a reference to the tag stored inside the task.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use crossbeam::channel;
    ///
    /// // The future inside the task.
    /// let future = async { 1 + 2 };
    ///
    /// // If the task gets woken, it will be sent into this channel.
    /// let (s, r) = channel::unbounded();
    /// let schedule = move |task| s.send(task).unwrap();
    ///
    /// // Create a task with the future and the schedule function.
    /// let (task, handle) = async_task::spawn(future, schedule, "a simple task");
    ///
    /// // Access the tag.
    /// assert_eq!(*handle.tag(), "a simple task");
    /// ```
    pub fn tag(&self) -> &T {
        let offset = Header::offset_tag::<T>();
        let ptr = self.raw_task.as_ptr();

        unsafe {
            let raw = (ptr as *mut u8).add(offset) as *const T;
            &*raw
        }
    }
}

impl<R, T> Drop for JoinHandle<R, T> {
    fn drop(&mut self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        // A place where the output will be stored in case it needs to be dropped.
        let mut output = None;

        unsafe {
            // Optimistically assume the `JoinHandle` is being dropped just after creating the
            // task. This is a common case so if the handle is not used, the overhead of it is only
            // one compare-exchange operation.
            if let Err(mut state) = (*header).state.compare_exchange_weak(
                SCHEDULED | HANDLE | REFERENCE,
                SCHEDULED | REFERENCE,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                loop {
                    // If the task has been completed but not yet closed, that means its output
                    // must be dropped.
                    if state & COMPLETED != 0 && state & CLOSED == 0 {
                        // Mark the task as closed in order to grab its output.
                        match (*header).state.compare_exchange_weak(
                            state,
                            state | CLOSED,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // Read the output.
                                output =
                                    Some((((*header).vtable.get_output)(ptr) as *mut R).read());

                                // Update the state variable because we're continuing the loop.
                                state |= CLOSED;
                            }
                            Err(s) => state = s,
                        }
                    } else {
                        // If this is the last reference to task and it's not closed, then close
                        // it and schedule one more time so that its future gets dropped by the
                        // executor.
                        let new = if state & (!(REFERENCE - 1) | CLOSED) == 0 {
                            SCHEDULED | CLOSED | REFERENCE
                        } else {
                            state & !HANDLE
                        };

                        // Unset the handle flag.
                        match (*header).state.compare_exchange_weak(
                            state,
                            new,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(_) => {
                                // If this is the last reference to the task, we need to either
                                // schedule dropping its future or destroy it.
                                if state & !(REFERENCE - 1) == 0 {
                                    if state & CLOSED == 0 {
                                        ((*header).vtable.schedule)(ptr);
                                    } else {
                                        ((*header).vtable.destroy)(ptr);
                                    }
                                }

                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
        }

        // Drop the output if it was taken out of the task.
        drop(output);
    }
}

impl<R, T> Future for JoinHandle<R, T> {
    type Output = Option<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        unsafe {
            let mut state = (*header).state.load(Ordering::Acquire);

            loop {
                // If the task has been closed, notify the awaiter and return `None`.
                if state & CLOSED != 0 {
                    // Even though the awaiter is most likely the current task, it could also be
                    // another task.
                    (*header).notify_unless(cx.waker());
                    return Poll::Ready(None);
                }

                // If the task is not completed, register the current task.
                if state & COMPLETED == 0 {
                    // Replace the waker with one associated with the current task. We need a
                    // safeguard against panics because dropping the previous waker can panic.
                    abort_on_panic(|| {
                        (*header).swap_awaiter(Some(cx.waker().clone()));
                    });

                    // Reload the state after registering. It is possible that the task became
                    // completed or closed just before registration so we need to check for that.
                    state = (*header).state.load(Ordering::Acquire);

                    // If the task has been closed, notify the awaiter and return `None`.
                    if state & CLOSED != 0 {
                        // Even though the awaiter is most likely the current task, it could also
                        // be another task.
                        (*header).notify_unless(cx.waker());
                        return Poll::Ready(None);
                    }

                    // If the task is still not completed, we're blocked on it.
                    if state & COMPLETED == 0 {
                        return Poll::Pending;
                    }
                }

                // Since the task is now completed, mark it as closed in order to grab its output.
                match (*header).state.compare_exchange(
                    state,
                    state | CLOSED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // Notify the awaiter. Even though the awaiter is most likely the current
                        // task, it could also be another task.
                        if state & AWAITER != 0 {
                            (*header).notify_unless(cx.waker());
                        }

                        // Take the output from the task.
                        let output = ((*header).vtable.get_output)(ptr) as *mut R;
                        return Poll::Ready(Some(output.read()));
                    }
                    Err(s) => state = s,
                }
            }
        }
    }
}

impl<R, T> fmt::Debug for JoinHandle<R, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        f.debug_struct("JoinHandle")
            .field("header", unsafe { &(*header) })
            .finish()
    }
}
