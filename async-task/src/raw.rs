use std::alloc::{self, Layout};
use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomData;
use std::mem::{self, ManuallyDrop};
use std::pin::Pin;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use crate::header::Header;
use crate::state::*;
use crate::utils::{abort_on_panic, extend};
use crate::Task;

/// The vtable for a task.
pub(crate) struct TaskVTable {
    /// The raw waker vtable.
    pub(crate) raw_waker: RawWakerVTable,

    /// Schedules the task.
    pub(crate) schedule: unsafe fn(*const ()),

    /// Drops the future inside the task.
    pub(crate) drop_future: unsafe fn(*const ()),

    /// Returns a pointer to the output stored after completion.
    pub(crate) get_output: unsafe fn(*const ()) -> *const (),

    /// Drops a waker or a task.
    pub(crate) decrement: unsafe fn(ptr: *const ()),

    /// Destroys the task.
    pub(crate) destroy: unsafe fn(*const ()),

    /// Runs the task.
    pub(crate) run: unsafe fn(*const ()),
}

/// Memory layout of a task.
///
/// This struct contains the information on:
///
/// 1. How to allocate and deallocate the task.
/// 2. How to access the fields inside the task.
#[derive(Clone, Copy)]
pub(crate) struct TaskLayout {
    /// Memory layout of the whole task.
    pub(crate) layout: Layout,

    /// Offset into the task at which the tag is stored.
    pub(crate) offset_t: usize,

    /// Offset into the task at which the schedule function is stored.
    pub(crate) offset_s: usize,

    /// Offset into the task at which the future is stored.
    pub(crate) offset_f: usize,

    /// Offset into the task at which the output is stored.
    pub(crate) offset_r: usize,
}

/// Raw pointers to the fields of a task.
pub(crate) struct RawTask<F, R, S, T> {
    /// The task header.
    pub(crate) header: *const Header,

    /// The schedule function.
    pub(crate) schedule: *const S,

    /// The tag inside the task.
    pub(crate) tag: *mut T,

    /// The future.
    pub(crate) future: *mut F,

    /// The output of the future.
    pub(crate) output: *mut R,
}

impl<F, R, S, T> Copy for RawTask<F, R, S, T> {}

impl<F, R, S, T> Clone for RawTask<F, R, S, T> {
    fn clone(&self) -> Self {
        Self {
            header: self.header,
            schedule: self.schedule,
            tag: self.tag,
            future: self.future,
            output: self.output,
        }
    }
}

impl<F, R, S, T> RawTask<F, R, S, T>
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
    S: Fn(Task<T>) + Send + Sync + 'static,
    T: Send + 'static,
{
    /// Allocates a task with the given `future` and `schedule` function.
    ///
    /// It is assumed there are initially only the `Task` reference and the `JoinHandle`.
    pub(crate) fn allocate(tag: T, future: F, schedule: S) -> NonNull<()> {
        // Compute the layout of the task for allocation. Abort if the computation fails.
        let task_layout = abort_on_panic(|| Self::task_layout());

        unsafe {
            // Allocate enough space for the entire task.
            let raw_task = match NonNull::new(alloc::alloc(task_layout.layout) as *mut ()) {
                None => std::process::abort(),
                Some(p) => p,
            };

            let raw = Self::from_ptr(raw_task.as_ptr());

            // Write the header as the first field of the task.
            (raw.header as *mut Header).write(Header {
                state: AtomicUsize::new(SCHEDULED | HANDLE | REFERENCE),
                awaiter: Cell::new(None),
                vtable: &TaskVTable {
                    raw_waker: RawWakerVTable::new(
                        Self::clone_waker,
                        Self::wake,
                        Self::wake_by_ref,
                        Self::decrement,
                    ),
                    schedule: Self::schedule,
                    drop_future: Self::drop_future,
                    get_output: Self::get_output,
                    decrement: Self::decrement,
                    destroy: Self::destroy,
                    run: Self::run,
                },
            });

            // Write the tag as the second field of the task.
            (raw.tag as *mut T).write(tag);

            // Write the schedule function as the third field of the task.
            (raw.schedule as *mut S).write(schedule);

            // Write the future as the fourth field of the task.
            raw.future.write(future);

            raw_task
        }
    }

    /// Creates a `RawTask` from a raw task pointer.
    #[inline]
    pub(crate) fn from_ptr(ptr: *const ()) -> Self {
        let task_layout = Self::task_layout();
        let p = ptr as *const u8;

        unsafe {
            Self {
                header: p as *const Header,
                tag: p.add(task_layout.offset_t) as *mut T,
                schedule: p.add(task_layout.offset_s) as *const S,
                future: p.add(task_layout.offset_f) as *mut F,
                output: p.add(task_layout.offset_r) as *mut R,
            }
        }
    }

    /// Returns the memory layout for a task.
    #[inline]
    fn task_layout() -> TaskLayout {
        // Compute the layouts for `Header`, `T`, `S`, `F`, and `R`.
        let layout_header = Layout::new::<Header>();
        let layout_t = Layout::new::<T>();
        let layout_s = Layout::new::<S>();
        let layout_f = Layout::new::<F>();
        let layout_r = Layout::new::<R>();

        // Compute the layout for `union { F, R }`.
        let size_union = layout_f.size().max(layout_r.size());
        let align_union = layout_f.align().max(layout_r.align());
        let layout_union = unsafe { Layout::from_size_align_unchecked(size_union, align_union) };

        // Compute the layout for `Header` followed by `T`, then `S`, then `union { F, R }`.
        let layout = layout_header;
        let (layout, offset_t) = extend(layout, layout_t);
        let (layout, offset_s) = extend(layout, layout_s);
        let (layout, offset_union) = extend(layout, layout_union);
        let offset_f = offset_union;
        let offset_r = offset_union;

        TaskLayout {
            layout,
            offset_t,
            offset_s,
            offset_f,
            offset_r,
        }
    }

    /// Wakes a waker.
    unsafe fn wake(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        let mut state = (*raw.header).state.load(Ordering::Acquire);

        loop {
            // If the task is completed or closed, it can't be woken.
            if state & (COMPLETED | CLOSED) != 0 {
                // Drop the waker.
                Self::decrement(ptr);
                break;
            }

            // If the task is already scheduled, we just need to synchronize with the thread that
            // will run the task by "publishing" our current view of the memory.
            if state & SCHEDULED != 0 {
                // Update the state without actually modifying it.
                match (*raw.header).state.compare_exchange_weak(
                    state,
                    state,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // Drop the waker.
                        Self::decrement(ptr);
                        break;
                    }
                    Err(s) => state = s,
                }
            } else {
                // Mark the task as scheduled.
                match (*raw.header).state.compare_exchange_weak(
                    state,
                    state | SCHEDULED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // If the task is not yet scheduled and isn't currently running, now is the
                        // time to schedule it.
                        if state & (SCHEDULED | RUNNING) == 0 {
                            // Schedule the task.
                            let task = Task {
                                raw_task: NonNull::new_unchecked(ptr as *mut ()),
                                _marker: PhantomData,
                            };
                            (*raw.schedule)(task);
                        } else {
                            // Drop the waker.
                            Self::decrement(ptr);
                        }

                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    /// Wakes a waker by reference.
    unsafe fn wake_by_ref(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        let mut state = (*raw.header).state.load(Ordering::Acquire);

        loop {
            // If the task is completed or closed, it can't be woken.
            if state & (COMPLETED | CLOSED) != 0 {
                break;
            }

            // If the task is already scheduled, we just need to synchronize with the thread that
            // will run the task by "publishing" our current view of the memory.
            if state & SCHEDULED != 0 {
                // Update the state without actually modifying it.
                match (*raw.header).state.compare_exchange_weak(
                    state,
                    state,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => break,
                    Err(s) => state = s,
                }
            } else {
                // If the task is not scheduled nor running, we'll need to schedule after waking.
                let new = if state & (SCHEDULED | RUNNING) == 0 {
                    (state | SCHEDULED) + REFERENCE
                } else {
                    state | SCHEDULED
                };

                // Mark the task as scheduled.
                match (*raw.header).state.compare_exchange_weak(
                    state,
                    new,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                ) {
                    Ok(_) => {
                        // If the task is not scheduled nor running, now is the time to schedule.
                        if state & (SCHEDULED | RUNNING) == 0 {
                            // If the reference count overflowed, abort.
                            if state > isize::max_value() as usize {
                                std::process::abort();
                            }

                            // Schedule the task.
                            let task = Task {
                                raw_task: NonNull::new_unchecked(ptr as *mut ()),
                                _marker: PhantomData,
                            };
                            (*raw.schedule)(task);
                        }

                        break;
                    }
                    Err(s) => state = s,
                }
            }
        }
    }

    /// Clones a waker.
    unsafe fn clone_waker(ptr: *const ()) -> RawWaker {
        let raw = Self::from_ptr(ptr);
        let raw_waker = &(*raw.header).vtable.raw_waker;

        // Increment the reference count. With any kind of reference-counted data structure,
        // relaxed ordering is fine when the reference is being cloned.
        let state = (*raw.header).state.fetch_add(REFERENCE, Ordering::Relaxed);

        // If the reference count overflowed, abort.
        if state > isize::max_value() as usize {
            std::process::abort();
        }

        RawWaker::new(ptr, raw_waker)
    }

    /// Drops a waker or a task.
    ///
    /// This function will decrement the reference count. If it drops down to zero and the
    /// associated join handle has been dropped too, then the task gets destroyed.
    #[inline]
    unsafe fn decrement(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // Decrement the reference count.
        let new = (*raw.header).state.fetch_sub(REFERENCE, Ordering::AcqRel) - REFERENCE;

        // If this was the last reference to the task and the `JoinHandle` has been dropped as
        // well, then destroy task.
        if new & !(REFERENCE - 1) == 0 && new & HANDLE == 0 {
            Self::destroy(ptr);
        }
    }

    /// Schedules a task for running.
    ///
    /// This function doesn't modify the state of the task. It only passes the task reference to
    /// its schedule function.
    unsafe fn schedule(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        (*raw.schedule)(Task {
            raw_task: NonNull::new_unchecked(ptr as *mut ()),
            _marker: PhantomData,
        });
    }

    /// Drops the future inside a task.
    #[inline]
    unsafe fn drop_future(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // We need a safeguard against panics because the destructor can panic.
        abort_on_panic(|| {
            raw.future.drop_in_place();
        })
    }

    /// Returns a pointer to the output inside a task.
    unsafe fn get_output(ptr: *const ()) -> *const () {
        let raw = Self::from_ptr(ptr);
        raw.output as *const ()
    }

    /// Cleans up task's resources and deallocates it.
    ///
    /// If the task has not been closed, then its future or the output will be dropped. The
    /// schedule function and the tag get dropped too.
    #[inline]
    unsafe fn destroy(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);
        let task_layout = Self::task_layout();

        // We need a safeguard against panics because destructors can panic.
        abort_on_panic(|| {
            // Drop the schedule function.
            (raw.schedule as *mut S).drop_in_place();

            // Drop the tag.
            (raw.tag as *mut T).drop_in_place();
        });

        // Finally, deallocate the memory reserved by the task.
        alloc::dealloc(ptr as *mut u8, task_layout.layout);
    }

    /// Runs a task.
    ///
    /// If polling its future panics, the task will be closed and the panic propagated into the
    /// caller.
    unsafe fn run(ptr: *const ()) {
        let raw = Self::from_ptr(ptr);

        // Create a context from the raw task pointer and the vtable inside the its header.
        let waker = ManuallyDrop::new(Waker::from_raw(RawWaker::new(
            ptr,
            &(*raw.header).vtable.raw_waker,
        )));
        let cx = &mut Context::from_waker(&waker);

        let mut state = (*raw.header).state.load(Ordering::Acquire);

        // Update the task's state before polling its future.
        loop {
            // If the task has been closed, drop the task reference and return.
            if state & CLOSED != 0 {
                // Notify the awaiter that the task has been closed.
                if state & AWAITER != 0 {
                    (*raw.header).notify();
                }

                // Drop the future.
                Self::drop_future(ptr);

                // Drop the task reference.
                Self::decrement(ptr);
                return;
            }

            // Mark the task as unscheduled and running.
            match (*raw.header).state.compare_exchange_weak(
                state,
                (state & !SCHEDULED) | RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    // Update the state because we're continuing with polling the future.
                    state = (state & !SCHEDULED) | RUNNING;
                    break;
                }
                Err(s) => state = s,
            }
        }

        // Poll the inner future, but surround it with a guard that closes the task in case polling
        // panics.
        let guard = Guard(raw);
        let poll = <F as Future>::poll(Pin::new_unchecked(&mut *raw.future), cx);
        mem::forget(guard);

        match poll {
            Poll::Ready(out) => {
                // Replace the future with its output.
                Self::drop_future(ptr);
                raw.output.write(out);

                // A place where the output will be stored in case it needs to be dropped.
                let mut output = None;

                // The task is now completed.
                loop {
                    // If the handle is dropped, we'll need to close it and drop the output.
                    let new = if state & HANDLE == 0 {
                        (state & !RUNNING & !SCHEDULED) | COMPLETED | CLOSED
                    } else {
                        (state & !RUNNING & !SCHEDULED) | COMPLETED
                    };

                    // Mark the task as not running and completed.
                    match (*raw.header).state.compare_exchange_weak(
                        state,
                        new,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(_) => {
                            // If the handle is dropped or if the task was closed while running,
                            // now it's time to drop the output.
                            if state & HANDLE == 0 || state & CLOSED != 0 {
                                // Read the output.
                                output = Some(raw.output.read());
                            }

                            // Notify the awaiter that the task has been completed.
                            if state & AWAITER != 0 {
                                (*raw.header).notify();
                            }

                            // Drop the task reference.
                            Self::decrement(ptr);
                            break;
                        }
                        Err(s) => state = s,
                    }
                }

                // Drop the output if it was taken out of the task.
                drop(output);
            }
            Poll::Pending => {
                // The task is still not completed.
                loop {
                    // If the task was closed while running, we'll need to unschedule in case it
                    // was woken and then clean up its resources.
                    let new = if state & CLOSED != 0 {
                        state & !RUNNING & !SCHEDULED
                    } else {
                        state & !RUNNING
                    };

                    // Mark the task as not running.
                    match (*raw.header).state.compare_exchange_weak(
                        state,
                        new,
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    ) {
                        Ok(state) => {
                            // If the task was closed while running, we need to drop its future.
                            // If the task was woken while running, we need to schedule it.
                            // Otherwise, we just drop the task reference.
                            if state & CLOSED != 0 {
                                // The thread that closed the task didn't drop the future because
                                // it was running so now it's our responsibility to do so.
                                Self::drop_future(ptr);

                                // Drop the task reference.
                                Self::decrement(ptr);
                            } else if state & SCHEDULED != 0 {
                                // The thread that has woken the task didn't reschedule it because
                                // it was running so now it's our responsibility to do so.
                                Self::schedule(ptr);
                            } else {
                                // Drop the task reference.
                                Self::decrement(ptr);
                            }
                            break;
                        }
                        Err(s) => state = s,
                    }
                }
            }
        }

        /// A guard that closes the task if polling its future panics.
        struct Guard<F, R, S, T>(RawTask<F, R, S, T>)
        where
            F: Future<Output = R> + Send + 'static,
            R: Send + 'static,
            S: Fn(Task<T>) + Send + Sync + 'static,
            T: Send + 'static;

        impl<F, R, S, T> Drop for Guard<F, R, S, T>
        where
            F: Future<Output = R> + Send + 'static,
            R: Send + 'static,
            S: Fn(Task<T>) + Send + Sync + 'static,
            T: Send + 'static,
        {
            fn drop(&mut self) {
                let raw = self.0;
                let ptr = raw.header as *const ();

                unsafe {
                    let mut state = (*raw.header).state.load(Ordering::Acquire);

                    loop {
                        // If the task was closed while running, then unschedule it, drop its
                        // future, and drop the task reference.
                        if state & CLOSED != 0 {
                            // We still need to unschedule the task because it is possible it was
                            // woken while running.
                            (*raw.header).state.fetch_and(!SCHEDULED, Ordering::AcqRel);

                            // The thread that closed the task didn't drop the future because it
                            // was running so now it's our responsibility to do so.
                            RawTask::<F, R, S, T>::drop_future(ptr);

                            // Drop the task reference.
                            RawTask::<F, R, S, T>::decrement(ptr);
                            break;
                        }

                        // Mark the task as not running, not scheduled, and closed.
                        match (*raw.header).state.compare_exchange_weak(
                            state,
                            (state & !RUNNING & !SCHEDULED) | CLOSED,
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        ) {
                            Ok(state) => {
                                // Drop the future because the task is now closed.
                                RawTask::<F, R, S, T>::drop_future(ptr);

                                // Notify the awaiter that the task has been closed.
                                if state & AWAITER != 0 {
                                    (*raw.header).notify();
                                }

                                // Drop the task reference.
                                RawTask::<F, R, S, T>::decrement(ptr);
                                break;
                            }
                            Err(s) => state = s,
                        }
                    }
                }
            }
        }
    }
}
