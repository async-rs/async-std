use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::mem;
use std::ptr::NonNull;

use crate::header::Header;
use crate::raw::RawTask;
use crate::JoinHandle;

/// Creates a new task.
///
/// This constructor returns a `Task` reference that runs the future and a [`JoinHandle`] that
/// awaits its result.
///
/// The `tag` is stored inside the allocated task.
///
/// When run, the task polls `future`. When woken, it gets scheduled for running by the
/// `schedule` function.
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// use crossbeam::channel;
///
/// // The future inside the task.
/// let future = async {
///     println!("Hello, world!");
/// };
///
/// // If the task gets woken, it will be sent into this channel.
/// let (s, r) = channel::unbounded();
/// let schedule = move |task| s.send(task).unwrap();
///
/// // Create a task with the future and the schedule function.
/// let (task, handle) = async_task::spawn(future, schedule, ());
/// ```
///
/// [`JoinHandle`]: struct.JoinHandle.html
pub fn spawn<F, R, S, T>(future: F, schedule: S, tag: T) -> (Task<T>, JoinHandle<R, T>)
where
    F: Future<Output = R> + Send + 'static,
    R: Send + 'static,
    S: Fn(Task<T>) + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let raw_task = RawTask::<F, R, S, T>::allocate(tag, future, schedule);
    let task = Task {
        raw_task,
        _marker: PhantomData,
    };
    let handle = JoinHandle {
        raw_task,
        _marker: PhantomData,
    };
    (task, handle)
}

/// A task that runs a future.
///
/// # Construction
///
/// A task is a heap-allocated structure containing:
///
/// * A reference counter.
/// * The state of the task.
/// * Arbitrary piece of data called a *tag*.
/// * A function that schedules the task when woken.
/// * A future or its result if polling has completed.
///
/// Constructor [`Task::create()`] returns a [`Task`] and a [`JoinHandle`]. Those two references
/// are like two sides of the task: one runs the future and the other awaits its result.
///
/// # Behavior
///
/// The [`Task`] reference "owns" the task itself and is used to [run] it. Running consumes the
/// [`Task`] reference and polls its internal future. If the future is still pending after being
/// polled, the [`Task`] reference will be recreated when woken by a [`Waker`]. If the future
/// completes, its result becomes available to the [`JoinHandle`].
///
/// The [`JoinHandle`] is a [`Future`] that awaits the result of the task.
///
/// When the task is woken, its [`Task`] reference is recreated and passed to the schedule function
/// provided during construction. In most executors, scheduling simply pushes the [`Task`] into a
/// queue of runnable tasks.
///
/// If the [`Task`] reference is dropped without being run, the task is cancelled.
///
/// Both [`Task`] and [`JoinHandle`] have methods that cancel the task. When cancelled, the task
/// won't be scheduled again even if a [`Waker`] wakes it or the [`JoinHandle`] is polled. An
/// attempt to run a cancelled task won't do anything. And if the cancelled task has already
/// completed, awaiting its result through [`JoinHandle`] will return `None`.
///
/// If polling the task's future panics, it gets cancelled automatically.
///
/// # Task states
///
/// A task can be in the following states:
///
/// * Sleeping: The [`Task`] reference doesn't exist and is waiting to be scheduled by a [`Waker`].
/// * Scheduled: The [`Task`] reference exists and is waiting to be [run].
/// * Completed: The [`Task`] reference doesn't exist anymore and can't be rescheduled, but its
///   result is available to the [`JoinHandle`].
/// * Cancelled: The [`Task`] reference may or may not exist, but running it does nothing and
///   awaiting the [`JoinHandle`] returns `None`.
///
/// When constructed, the task is initially in the scheduled state.
///
/// # Destruction
///
/// The future inside the task gets dropped in the following cases:
///
/// * When [`Task`] is dropped.
/// * When [`Task`] is run to completion.
///
/// If the future hasn't been dropped and the last [`Waker`] or [`JoinHandle`] is dropped, or if
/// a [`JoinHandle`] cancels the task, then the task will be scheduled one last time so that its
/// future gets dropped by the executor. In other words, the task's future can be dropped only by
/// [`Task`].
///
/// When the task completes, the result of its future is stored inside the allocation. This result
/// is taken out when the [`JoinHandle`] awaits it. When the task is cancelled or the
/// [`JoinHandle`] is dropped without being awaited, the result gets dropped too.
///
/// The task gets deallocated when all references to it are dropped, which includes the [`Task`],
/// the [`JoinHandle`], and any associated [`Waker`]s.
///
/// The tag inside the task and the schedule function get dropped at the time of deallocation.
///
/// # Panics
///
/// If polling the inner future inside [`run()`] panics, the panic will be propagated into
/// the caller. Likewise, a panic inside the task result's destructor will be propagated. All other
/// panics result in the process being aborted.
///
/// More precisely, the process is aborted if a panic occurs:
///
/// * Inside the schedule function.
/// * While dropping the tag.
/// * While dropping the future.
/// * While dropping the schedule function.
/// * While waking the task awaiting the [`JoinHandle`].
///
/// [`run()`]: struct.Task.html#method.run
/// [run]: struct.Task.html#method.run
/// [`JoinHandle`]: struct.JoinHandle.html
/// [`Task`]: struct.Task.html
/// [`Task::create()`]: struct.Task.html#method.create
/// [`Future`]: https://doc.rust-lang.org/std/future/trait.Future.html
/// [`Waker`]: https://doc.rust-lang.org/std/task/struct.Waker.html
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// use async_task::Task;
/// use crossbeam::channel;
/// use futures::executor;
///
/// // The future inside the task.
/// let future = async {
///     println!("Hello, world!");
/// };
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
/// // Await its result.
/// executor::block_on(handle);
/// ```
pub struct Task<T> {
    /// A pointer to the heap-allocated task.
    pub(crate) raw_task: NonNull<()>,

    /// A marker capturing the generic type `T`.
    pub(crate) _marker: PhantomData<T>,
}

unsafe impl<T> Send for Task<T> {}
unsafe impl<T> Sync for Task<T> {}

impl<T> Task<T> {
    /// Schedules the task.
    ///
    /// This is a convenience method that simply reschedules the task by passing it to its schedule
    /// function.
    ///
    /// If the task is cancelled, this method won't do anything.
    ///
    /// # Examples
    ///
    /// ```
    /// # #![feature(async_await)]
    /// use crossbeam::channel;
    ///
    /// // The future inside the task.
    /// let future = async {
    ///     println!("Hello, world!");
    /// };
    ///
    /// // If the task gets woken, it will be sent into this channel.
    /// let (s, r) = channel::unbounded();
    /// let schedule = move |task| s.send(task).unwrap();
    ///
    /// // Create a task with the future and the schedule function.
    /// let (task, handle) = async_task::spawn(future, schedule, ());
    ///
    /// // Send the task into the channel.
    /// task.schedule();
    ///
    /// // Retrieve the task back from the channel.
    /// let task = r.recv().unwrap();
    /// ```
    pub fn schedule(self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;
        mem::forget(self);

        unsafe {
            ((*header).vtable.schedule)(ptr);
        }
    }

    /// Runs the task.
    ///
    /// This method polls the task's future. If the future completes, its result will become
    /// available to the [`JoinHandle`]. And if the future is still pending, the task will have to
    /// be woken in order to be rescheduled and then run again.
    ///
    /// If the task is cancelled, running it won't do anything.
    ///
    /// # Panics
    ///
    /// It is possible that polling the future panics, in which case the panic will be propagated
    /// into the caller. It is advised that invocations of this method are wrapped inside
    /// [`catch_unwind`].
    ///
    /// If a panic occurs, the task is automatically cancelled.
    ///
    /// [`catch_unwind`]: https://doc.rust-lang.org/std/panic/fn.catch_unwind.html
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
    /// // Run the task. In this example, it will complete after a single run.
    /// task.run();
    /// assert!(r.is_empty());
    ///
    /// // Await the result of the task.
    /// let result = executor::block_on(handle);
    /// assert_eq!(result, Some(3));
    /// ```
    pub fn run(self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;
        mem::forget(self);

        unsafe {
            ((*header).vtable.run)(ptr);
        }
    }

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
    /// task.cancel();
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
            (*header).cancel();
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
    /// assert_eq!(*task.tag(), "a simple task");
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

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        unsafe {
            // Cancel the task.
            (*header).cancel();

            // Drop the future.
            ((*header).vtable.drop_future)(ptr);

            // Drop the task reference.
            ((*header).vtable.decrement)(ptr);
        }
    }
}

impl<T: fmt::Debug> fmt::Debug for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.raw_task.as_ptr();
        let header = ptr as *const Header;

        f.debug_struct("Task")
            .field("header", unsafe { &(*header) })
            .field("tag", self.tag())
            .finish()
    }
}
