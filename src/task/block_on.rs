use std::cell::Cell;
use std::mem::{self, ManuallyDrop};
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable};
use std::thread;

use crossbeam_utils::sync::Parker;
use kv_log_macro::trace;
use log::log_enabled;

use crate::future::Future;
use crate::task::{Context, Poll, Task, Waker};

/// Spawns a task and blocks the current thread on its result.
///
/// Calling this function is similar to [spawning] a thread and immediately [joining] it, except an
/// asynchronous task will be spawned.
///
/// See also: [`task::spawn_blocking`].
///
/// [`task::spawn_blocking`]: fn.spawn_blocking.html
///
/// [spawning]: https://doc.rust-lang.org/std/thread/fn.spawn.html
/// [joining]: https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join
///
/// # Examples
///
/// ```no_run
/// use async_std::task;
///
/// task::block_on(async {
///     println!("Hello, world!");
/// })
/// ```
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    // Create a new task handle.
    let task = Task::new(None);

    // Log this `block_on` operation.
    if log_enabled!(log::Level::Trace) {
        trace!("block_on", {
            task_id: task.id().0,
            parent_task_id: Task::get_current(|t| t.id().0).unwrap_or(0),
        });
    }

    let future = async move {
        // Drop task-locals on exit.
        defer! {
            Task::get_current(|t| unsafe { t.drop_locals() });
        }

        // Log completion on exit.
        defer! {
            if log_enabled!(log::Level::Trace) {
                Task::get_current(|t| {
                    trace!("completed", {
                        task_id: t.id().0,
                    });
                });
            }
        }

        future.await
    };

    // Run the future as a task.
    unsafe { Task::set_current(&task, || run(future)) }
}

/// Blocks the current thread on a future's result.
fn run<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    thread_local! {
        // May hold a pre-allocated parker that can be reused for efficiency.
        //
        // Note that each invocation of `block` needs its own parker. In particular, if `block`
        // recursively calls itself, we must make sure that each recursive call uses a distinct
        // parker instance.
        static CACHE: Cell<Option<Arc<Parker>>> = Cell::new(None);
    }

    // Virtual table for wakers based on `Arc<Parker>`.
    static VTABLE: RawWakerVTable = {
        unsafe fn clone_raw(ptr: *const ()) -> RawWaker {
            let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const Parker));
            mem::forget(arc.clone());
            RawWaker::new(ptr, &VTABLE)
        }

        unsafe fn wake_raw(ptr: *const ()) {
            let arc = Arc::from_raw(ptr as *const Parker);
            arc.unparker().unpark();
        }

        unsafe fn wake_by_ref_raw(ptr: *const ()) {
            let arc = ManuallyDrop::new(Arc::from_raw(ptr as *const Parker));
            arc.unparker().unpark();
        }

        unsafe fn drop_raw(ptr: *const ()) {
            drop(Arc::from_raw(ptr as *const Parker))
        }

        RawWakerVTable::new(clone_raw, wake_raw, wake_by_ref_raw, drop_raw)
    };

    // Pin the future on the stack.
    pin_utils::pin_mut!(future);

    CACHE.with(|cache| {
        // Reuse a cached parker or create a new one for this invocation of `block`.
        let arc_parker: Arc<Parker> = cache.take().unwrap_or_else(|| Arc::new(Parker::new()));
        let ptr = (&*arc_parker as *const Parker) as *const ();

        // Create a waker and task context.
        let waker = unsafe { ManuallyDrop::new(Waker::from_raw(RawWaker::new(ptr, &VTABLE))) };
        let cx = &mut Context::from_waker(&waker);

        let mut step = 0;
        loop {
            if let Poll::Ready(t) = future.as_mut().poll(cx) {
                // Save the parker for the next invocation of `block`.
                cache.set(Some(arc_parker));
                return t;
            }

            // Yield a few times or park the current thread.
            if step < 3 {
                thread::yield_now();
                step += 1;
            } else {
                arc_parker.park();
                step = 0;
            }
        }
    })
}
