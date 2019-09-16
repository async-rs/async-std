use std::cell::Cell;
use std::ptr;

use crossbeam_deque::Worker;

use super::pool;
use super::task;
use super::Task;
use crate::utils::abort_on_panic;

/// Returns a handle to the current task.
///
/// # Panics
///
/// This function will panic if not called within the context of a task created by [`block_on`],
/// [`spawn`], or [`Builder::spawn`].
///
/// [`block_on`]: fn.block_on.html
/// [`spawn`]: fn.spawn.html
/// [`Builder::spawn`]: struct.Builder.html#method.spawn
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::task;
///
/// println!("The name of this task is {:?}", task::current().name());
/// #
/// # }) }
/// ```
pub fn current() -> Task {
    get_task(|task| task.clone()).expect("`task::current()` called outside the context of a task")
}

thread_local! {
    static TAG: Cell<*const task::Tag> = Cell::new(ptr::null_mut());
}

pub(crate) fn set_tag<F, R>(tag: *const task::Tag, f: F) -> R
where
    F: FnOnce() -> R,
{
    struct ResetTag<'a>(&'a Cell<*const task::Tag>);

    impl Drop for ResetTag<'_> {
        fn drop(&mut self) {
            self.0.set(ptr::null());
        }
    }

    TAG.with(|t| {
        t.set(tag);
        let _guard = ResetTag(t);

        f()
    })
}

pub(crate) fn get_task<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&Task) -> R,
{
    let res = TAG.try_with(|tag| unsafe { tag.get().as_ref().map(task::Tag::task).map(f) });

    match res {
        Ok(Some(val)) => Some(val),
        Ok(None) | Err(_) => None,
    }
}

thread_local! {
    static IS_WORKER: Cell<bool> = Cell::new(false);
    static QUEUE: Cell<Option<Worker<task::Runnable>>> = Cell::new(None);
}

pub fn is_worker() -> bool {
    IS_WORKER.with(|is_worker| is_worker.get())
}

fn get_queue<F: FnOnce(&Worker<task::Runnable>) -> T, T>(f: F) -> T {
    QUEUE.with(|queue| {
        let q = queue.take().unwrap();
        let ret = f(&q);
        queue.set(Some(q));
        ret
    })
}

pub(crate) fn schedule(task: task::Runnable) {
    if is_worker() {
        get_queue(|q| q.push(task));
    } else {
        pool::get().injector.push(task);
    }
    pool::get().sleepers.notify_one();
}

pub(crate) fn main_loop(worker: Worker<task::Runnable>) {
    IS_WORKER.with(|is_worker| is_worker.set(true));
    QUEUE.with(|queue| queue.set(Some(worker)));

    loop {
        match get_queue(|q| pool::get().find_task(q)) {
            Some(task) => set_tag(task.tag(), || abort_on_panic(|| task.run())),
            None => pool::get().sleepers.wait(),
        }
    }
}
