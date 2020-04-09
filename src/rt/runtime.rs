use std::cell::Cell;
use std::io;
use std::iter;
use std::sync::atomic::{self, Ordering};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{unbounded, Receiver, Sender};
use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use crossbeam_utils::thread::{scope, Scope};
use once_cell::unsync::OnceCell;

use crate::rt::Reactor;
use crate::sync::Spinlock;
use crate::task::Runnable;
use crate::utils::{abort_on_panic, random};

thread_local! {
    /// A reference to the current machine, if the current thread runs tasks.
    static MACHINE: OnceCell<Arc<Machine>> = OnceCell::new();

    /// This flag is set to true whenever `task::yield_now()` is invoked.
    static YIELD_NOW: Cell<bool> = Cell::new(false);
}

struct Scheduler {
    /// Set to `true` while a machine is polling the reactor.
    polling: bool,
}

/// Task to be sent to worker thread
enum Task {
    Runnable(Runnable),
    Terminate,
}

/// Action to be sent to runtime
enum Action {
    ScaleUp,
    ScaleDown,
    Terminate,
}

/// An async runtime.
pub struct Runtime {
    /// The reactor.
    reactor: Reactor,

    /// The global queue of tasks.
    injector: Injector<Task>,

    /// Handles to local queues for stealing work.
    stealers: RwLock<Vec<(Stealer<Task>, usize)>>,

    /// The scheduler state.
    sched: Mutex<Scheduler>,

    /// Number of minimal worker thread that must available
    min_worker: usize,

    /// Counter for generating id of worker thread
    counter: AtomicUsize,

    /// Reciever side for runtime action channel
    reciever: Receiver<Action>,

    /// Sender side for runtime action channel
    sender: Sender<Action>,
}

impl Runtime {
    /// Creates a new runtime.
    pub fn new() -> Runtime {
        let (sender, reciever) = unbounded();
        Runtime {
            reactor: Reactor::new().unwrap(),
            injector: Injector::new(),
            stealers: RwLock::new(vec![]),
            sched: Mutex::new(Scheduler { polling: false }),
            min_worker: num_cpus::get().max(1),
            counter: AtomicUsize::new(0),
            reciever,
            sender,
        }
    }

    /// Returns a reference to the reactor.
    pub fn reactor(&self) -> &Reactor {
        &self.reactor
    }

    /// Flushes the task slot so that tasks get run more fairly.
    pub fn yield_now(&self) {
        YIELD_NOW.with(|flag| flag.set(true));
    }

    /// Schedules a task.
    pub fn schedule(&self, task: Runnable) {
        MACHINE.with(|machine| {
            // If the current thread is a worker thread, schedule it onto the current machine.
            // Otherwise, push it into the global task queue.
            match machine.get() {
                None => {
                    self.injector.push(Task::Runnable(task));
                    self.notify();
                }
                Some(m) => m.schedule(&self, Task::Runnable(task)),
            }
        });
    }

    /// Start a worker thread.
    fn start_new_thread<'e, 's: 'e>(&'s self, scope: &Scope<'e>) {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        let m = Arc::new(Machine::new(id, Processor::new()));

        self.stealers
            .write()
            .unwrap()
            .push((m.processor.lock().worker.stealer(), id));

        scope
            .builder()
            .name("async-std/machine".to_string())
            .spawn(move |_| {
                abort_on_panic(|| {
                    let _ = MACHINE.with(|machine| machine.set(m.clone()));
                    m.run(self);
                })
            })
            .expect("cannot start a machine thread");
    }

    /// Runs the runtime on the current thread.
    pub fn run(&self) {
        scope(|s| {
            (0..self.min_worker).for_each(|_| self.start_new_thread(s));

            loop {
                match self.reciever.recv().unwrap() {
                    Action::ScaleUp => self.start_new_thread(s),
                    Action::ScaleDown => {
                        // Random worker thread will recieve this notification
                        // and terminate itself
                        self.injector.push(Task::Terminate)
                    }
                    Action::Terminate => return,
                }
            }
        })
        .unwrap();
    }

    /// Create more worker thread for the runtime
    pub fn scale_up(&self) {
        self.sender.send(Action::ScaleUp).unwrap()
    }

    /// Terminate 1 worker thread, it is guaranteed that
    /// the number of worker thread won't go down
    /// below the number of available cpu
    pub fn scale_down(&self) {
        if self.stealers.read().unwrap().len() > self.min_worker {
            self.sender.send(Action::ScaleDown).unwrap()
        }
    }

    /// Deregister worker thread by theirs id
    fn deregister(&self, id: usize) {
        let mut stealers = self.stealers.write().unwrap();
        let mut index = None;
        for i in 0..stealers.len() {
            if stealers[i].1 == id {
                index = Some(i);
                break;
            }
        }
        if let Some(index) = index {
            stealers.remove(index);
            if stealers.is_empty() {
                self.sender.send(Action::Terminate).unwrap();
            }
        }
    }

    /// Unparks a thread polling the reactor.
    fn notify(&self) {
        atomic::fence(Ordering::SeqCst);
        self.reactor.notify().unwrap();
    }

    /// Attempts to poll the reactor without blocking on it.
    ///
    /// Returns `Ok(true)` if at least one new task was woken.
    ///
    /// This function might not poll the reactor at all so do not rely on it doing anything. Only
    /// use for optimization.
    fn quick_poll(&self) -> io::Result<bool> {
        if let Ok(sched) = self.sched.try_lock() {
            if !sched.polling {
                return self.reactor.poll(Some(Duration::from_secs(0)));
            }
        }
        Ok(false)
    }
}

/// A thread running a processor.
struct Machine {
    /// Holds the processor until it gets stolen.
    processor: Spinlock<Processor>,

    id: usize,
    drained: AtomicBool,
}

impl Machine {
    /// Creates a new machine running a processor.
    fn new(id: usize, p: Processor) -> Machine {
        Machine {
            processor: Spinlock::new(p),
            id,
            drained: AtomicBool::new(false),
        }
    }

    /// Schedules a task onto the machine.
    fn schedule(&self, rt: &Runtime, task: Task) {
        if !self.drained.load(Ordering::SeqCst /* ??? */) {
            self.processor.lock().schedule(rt, task);
        } else {
            // We don't accept task anymore,
            // push to global queue
            rt.injector.push(task);
        }
    }

    /// Finds the next runnable task.
    fn find_task(&self, rt: &Runtime) -> Steal<Task> {
        let mut retry = false;

        // First try finding a task in the local queue or in the global queue.
        if let Some(task) = self.processor.lock().pop_task() {
            return Steal::Success(task);
        }

        match self.processor.lock().steal_from_global(rt) {
            Steal::Empty => {}
            Steal::Retry => retry = true,
            Steal::Success(task) => return Steal::Success(task),
        }

        // Try polling the reactor, but don't block on it.
        let progress = rt.quick_poll().unwrap();

        // Try finding a task in the local queue, which might hold tasks woken by the reactor. If
        // the local queue is still empty, try stealing from other processors.
        if progress {
            if let Some(task) = self.processor.lock().pop_task() {
                return Steal::Success(task);
            }
        }

        match self.processor.lock().steal_from_others(rt) {
            Steal::Empty => {}
            Steal::Retry => retry = true,
            Steal::Success(task) => return Steal::Success(task),
        }

        if retry {
            Steal::Retry
        } else {
            Steal::Empty
        }
    }

    /// Runs the machine on the current thread.
    fn run(&self, rt: &Runtime) {
        /// Number of yields when no runnable task is found.
        const YIELDS: u32 = 3;
        /// Number of short sleeps when no runnable task in found.
        const SLEEPS: u32 = 10;
        /// Number of runs in a row before the global queue is inspected.
        const RUNS: u32 = 64;

        // The number of times the thread found work in a row.
        let mut runs = 0;
        // The number of times the thread didn't find work in a row.
        let mut fails = 0;

        loop {
            // Check if `task::yield_now()` was invoked and flush the slot if so.
            YIELD_NOW.with(|flag| {
                if flag.replace(false) {
                    self.processor.lock().flush_slot(rt);
                }
            });

            // After a number of runs in a row, do some work to ensure no task is left behind
            // indefinitely. Poll the reactor, steal tasks from the global queue, and flush the
            // task slot.
            if runs >= RUNS {
                runs = 0;
                rt.quick_poll().unwrap();

                let mut p = self.processor.lock();
                if let Steal::Success(task) = p.steal_from_global(rt) {
                    p.schedule(rt, task);
                }

                p.flush_slot(rt);
            }

            // Try to find a runnable task.
            if let Steal::Success(task) = self.find_task(rt) {
                match task {
                    Task::Runnable(task) => task.run(),
                    Task::Terminate => {
                        self.deregister(rt);
                        return;
                    }
                }
                runs += 1;
                fails = 0;
                continue;
            }

            fails += 1;

            // Yield the current thread a few times.
            if fails <= YIELDS {
                thread::yield_now();
                continue;
            }

            // Put the current thread to sleep a few times.
            if fails <= YIELDS + SLEEPS {
                thread::sleep(Duration::from_micros(10));
                continue;
            }

            // One final check for available tasks while the scheduler is locked.
            if let Some(task) = iter::repeat_with(|| self.find_task(rt))
                .find(|s| !s.is_retry())
                .and_then(|s| s.success())
            {
                self.schedule(rt, task);
                continue;
            }

            let mut sched = rt.sched.lock().unwrap();

            if sched.polling {
                thread::sleep(Duration::from_micros(10));
                continue;
            }

            sched.polling = true;
            drop(sched);

            rt.reactor.poll(None).unwrap();

            let mut sched = rt.sched.lock().unwrap();
            sched.polling = false;

            runs = 0;
            fails = 0;
        }
    }

    /// deregister this worker thread from Runtime
    fn deregister(&self, rt: &Runtime) {
        self.drained.store(true, Ordering::SeqCst /* ??? */);
        self.processor.lock().drain(rt);
        rt.deregister(self.id);
    }
}

struct Processor {
    /// The local task queue.
    worker: Worker<Task>,

    /// Contains the next task to run as an optimization that skips the queue.
    slot: Option<Task>,
}

impl Processor {
    /// Creates a new processor.
    fn new() -> Processor {
        Processor {
            worker: Worker::new_fifo(),
            slot: None,
        }
    }

    /// Schedules a task to run on this processor.
    fn schedule(&mut self, rt: &Runtime, task: Task) {
        match self.slot.replace(task) {
            None => {}
            Some(task) => {
                self.worker.push(task);
                rt.notify();
            }
        }
    }

    /// Flushes a task from the slot into the local queue.
    fn flush_slot(&mut self, rt: &Runtime) {
        if let Some(task) = self.slot.take() {
            self.worker.push(task);
            rt.notify();
        }
    }

    /// Pops a task from this processor.
    fn pop_task(&mut self) -> Option<Task> {
        self.slot.take().or_else(|| self.worker.pop())
    }

    /// Steals a task from the global queue.
    fn steal_from_global(&self, rt: &Runtime) -> Steal<Task> {
        rt.injector.steal_batch_and_pop(&self.worker)
    }

    /// Steals a task from other processors.
    fn steal_from_others(&self, rt: &Runtime) -> Steal<Task> {
        let stealers = rt.stealers.read().unwrap();

        // Pick a random starting point in the list of queues.
        let len = stealers.len();
        let start = random(len as u32) as usize;

        // Create an iterator over stealers that starts from the chosen point.
        let (l, r) = stealers.split_at(start);
        let stealers = r.iter().chain(l.iter());

        // Try stealing a batch of tasks from each queue.
        stealers
            .map(|s| s.0.steal_batch_and_pop(&self.worker))
            .collect()
    }

    // Move all pending tasks to global queue.
    fn drain(&mut self, rt: &Runtime) {
        self.flush_slot(rt);

        while let Some(task) = self.worker.pop() {
            rt.injector.push(task);
        }
    }
}
