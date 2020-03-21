use std::cell::Cell;
use std::io;
use std::iter;
use std::sync::atomic::{self, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_deque::{Injector, Steal, Stealer, Worker};
use crossbeam_utils::thread::scope;
use once_cell::unsync::OnceCell;

use crate::rt::Reactor;
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

/// An async runtime.
pub struct Runtime {
    /// The reactor.
    reactor: Reactor,

    /// The global queue of tasks.
    injector: Injector<Runnable>,

    /// Handles to local queues for stealing work.
    stealers: Vec<Stealer<Runnable>>,

    /// Machines to start
    machines: Vec<Arc<Machine>>,

    /// The scheduler state.
    sched: Mutex<Scheduler>,
}

impl Runtime {
    /// Creates a new runtime.
    pub fn new() -> Runtime {
        let cpus = num_cpus::get().max(1);
        let processors: Vec<_> = (0..cpus).map(|_| Processor::new()).collect();

        let machines: Vec<_> = processors
            .into_iter()
            .map(|p| Arc::new(Machine::new(p)))
            .collect();

        let stealers = machines
            .iter()
            .map(|m| m.processor.worker.stealer())
            .collect();

        Runtime {
            reactor: Reactor::new().unwrap(),
            injector: Injector::new(),
            stealers,
            machines,
            sched: Mutex::new(Scheduler { polling: false }),
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
                    self.injector.push(task);
                    self.notify();
                }
                Some(m) => m.schedule(&self, task),
            }
        });
    }

    /// Runs the runtime on the current thread.
    pub fn run(&self) {
        scope(|s| {
            for m in &self.machines {
                s.builder()
                    .name("async-std/machine".to_string())
                    .spawn(move |_| {
                        abort_on_panic(|| {
                            let _ = MACHINE.with(|machine| machine.set(m.clone()));
                            m.run(self);
                        })
                    })
                    .expect("cannot start a machine thread");
            }
        })
        .unwrap();
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

    fn poll(&self) -> io::Result<bool> {
        let mut sched = self.sched.lock().unwrap();
        sched.polling = true;
        drop(sched);

        let result = self.reactor.poll(None);

        let mut sched = self.sched.lock().unwrap();
        sched.polling = false;

        result
    }
}

/// A thread running a processor.
struct Machine {
    /// Holds the processor until it gets stolen.
    processor: Processor,
}

unsafe impl Send for Machine {}
unsafe impl Sync for Machine {}

impl Machine {
    /// Creates a new machine running a processor.
    fn new(p: Processor) -> Machine {
        Machine { processor: p }
    }

    /// Schedules a task onto the machine.
    fn schedule(&self, rt: &Runtime, task: Runnable) {
        self.processor.schedule(rt, task);
    }

    /// Finds the next runnable task.
    fn find_task(&self, rt: &Runtime) -> Steal<Runnable> {
        let mut retry = false;

        // First try finding a task in the local queue or in the global queue.
        if let Some(task) = self.processor.pop_task() {
            return Steal::Success(task);
        }

        match self.processor.steal_from_global(rt) {
            Steal::Empty => {}
            Steal::Retry => retry = true,
            Steal::Success(task) => return Steal::Success(task),
        }

        // Try polling the reactor, but don't block on it.
        let progress = rt.quick_poll().unwrap();

        // Try finding a task in the local queue, which might hold tasks woken by the reactor. If
        // the local queue is still empty, try stealing from other processors.
        if progress {
            if let Some(task) = self.processor.pop_task() {
                return Steal::Success(task);
            }
        }

        match self.processor.steal_from_others(rt) {
            Steal::Empty => {}
            Steal::Retry => retry = true,
            Steal::Success(task) => return Steal::Success(task),
        }

        if retry { Steal::Retry } else { Steal::Empty }
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
                    self.processor.flush_slot(rt);
                }
            });

            // After a number of runs in a row, do some work to ensure no task is left behind
            // indefinitely. Poll the reactor, steal tasks from the global queue, and flush the
            // task slot.
            if runs >= RUNS {
                runs = 0;
                rt.quick_poll().unwrap();

                if let Steal::Success(task) = self.processor.steal_from_global(rt) {
                    self.processor.schedule(rt, task);
                }

                self.processor.flush_slot(rt);
            }

            // Try to find a runnable task.
            if let Steal::Success(task) = self.find_task(rt) {
                task.run();
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

            rt.poll().unwrap();

            runs = 0;
            fails = 0;
        }
    }
}

struct Processor {
    /// The local task queue.
    worker: Worker<Runnable>,

    /// Contains the next task to run as an optimization that skips the queue.
    slot: Cell<Option<Runnable>>,
}

impl Processor {
    /// Creates a new processor.
    fn new() -> Processor {
        Processor {
            worker: Worker::new_fifo(),
            slot: Cell::new(None),
        }
    }

    /// Schedules a task to run on this processor.
    fn schedule(&self, rt: &Runtime, task: Runnable) {
        match self.slot.replace(Some(task)) {
            None => {}
            Some(task) => {
                self.worker.push(task);
                rt.notify();
            }
        }
    }

    /// Flushes a task from the slot into the local queue.
    fn flush_slot(&self, rt: &Runtime) {
        if let Some(task) = self.slot.take() {
            self.worker.push(task);
            rt.notify();
        }
    }

    /// Pops a task from this processor.
    fn pop_task(&self) -> Option<Runnable> {
        self.slot.take().or_else(|| self.worker.pop())
    }

    /// Steals a task from the global queue.
    fn steal_from_global(&self, rt: &Runtime) -> Steal<Runnable> {
        rt.injector.steal_batch_and_pop(&self.worker)
    }

    /// Steals a task from other processors.
    fn steal_from_others(&self, rt: &Runtime) -> Steal<Runnable> {
        // Pick a random starting point in the list of queues.
        let len = rt.stealers.len();
        let start = random(len as u32) as usize;

        // Create an iterator over stealers that starts from the chosen point.
        let (l, r) = rt.stealers.split_at(start);
        let stealers = r.iter().chain(l.iter());

        // Try stealing a batch of tasks from each queue.
        stealers
            .map(|s| s.steal_batch_and_pop(&self.worker))
            .collect()
    }
}
