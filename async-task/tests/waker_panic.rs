#![feature(async_await)]

use std::cell::Cell;
use std::future::Future;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::task::Waker;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use async_task::Task;
use crossbeam::atomic::AtomicCell;
use crossbeam::channel;
use lazy_static::lazy_static;

// Creates a future with event counters.
//
// Usage: `future!(f, waker, POLL, DROP)`
//
// The future `f` always sleeps for 200 ms, and panics the second time it is polled.
// When it gets polled, `POLL` is incremented.
// When it gets dropped, `DROP` is incremented.
//
// Every time the future is run, it stores the waker into a global variable.
// This waker can be extracted using the `waker` function.
macro_rules! future {
    ($name:pat, $waker:pat, $poll:ident, $drop:ident) => {
        lazy_static! {
            static ref $poll: AtomicCell<usize> = AtomicCell::new(0);
            static ref $drop: AtomicCell<usize> = AtomicCell::new(0);
            static ref WAKER: AtomicCell<Option<Waker>> = AtomicCell::new(None);
        }

        let ($name, $waker) = {
            struct Fut(Cell<bool>, Box<i32>);

            impl Future for Fut {
                type Output = ();

                fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                    WAKER.store(Some(cx.waker().clone()));
                    $poll.fetch_add(1);
                    thread::sleep(ms(200));

                    if self.0.get() {
                        panic!()
                    } else {
                        self.0.set(true);
                        Poll::Pending
                    }
                }
            }

            impl Drop for Fut {
                fn drop(&mut self) {
                    $drop.fetch_add(1);
                }
            }

            (Fut(Cell::new(false), Box::new(0)), || {
                WAKER.swap(None).unwrap()
            })
        };
    };
}

// Creates a schedule function with event counters.
//
// Usage: `schedule!(s, chan, SCHED, DROP)`
//
// The schedule function `s` pushes the task into `chan`.
// When it gets invoked, `SCHED` is incremented.
// When it gets dropped, `DROP` is incremented.
//
// Receiver `chan` extracts the task when it is scheduled.
macro_rules! schedule {
    ($name:pat, $chan:pat, $sched:ident, $drop:ident) => {
        lazy_static! {
            static ref $sched: AtomicCell<usize> = AtomicCell::new(0);
            static ref $drop: AtomicCell<usize> = AtomicCell::new(0);
        }

        let ($name, $chan) = {
            let (s, r) = channel::unbounded();

            struct Guard(Box<i32>);

            impl Drop for Guard {
                fn drop(&mut self) {
                    $drop.fetch_add(1);
                }
            }

            let guard = Guard(Box::new(0));
            let sched = move |task: Task<_>| {
                &guard;
                $sched.fetch_add(1);
                s.send(task).unwrap();
            };

            (sched, r)
        };
    };
}

// Creates a task with event counters.
//
// Usage: `task!(task, handle f, s, DROP)`
//
// A task with future `f` and schedule function `s` is created.
// The `Task` and `JoinHandle` are bound to `task` and `handle`, respectively.
// When the tag inside the task gets dropped, `DROP` is incremented.
macro_rules! task {
    ($task:pat, $handle: pat, $future:expr, $schedule:expr, $drop:ident) => {
        lazy_static! {
            static ref $drop: AtomicCell<usize> = AtomicCell::new(0);
        }

        let ($task, $handle) = {
            struct Tag(Box<i32>);

            impl Drop for Tag {
                fn drop(&mut self) {
                    $drop.fetch_add(1);
                }
            }

            async_task::spawn($future, $schedule, Tag(Box::new(0)))
        };
    };
}

fn ms(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn wake_during_run() {
    future!(f, waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    task.run();
    let w = waker();
    w.wake_by_ref();
    let task = chan.recv().unwrap();

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            drop(waker());
            assert_eq!(POLL.load(), 2);
            assert_eq!(SCHEDULE.load(), 1);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
            assert_eq!(chan.len(), 0);
        });

        thread::sleep(ms(100));

        w.wake();
        drop(handle);
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        thread::sleep(ms(200));

        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 1);
        assert_eq!(DROP_S.load(), 1);
        assert_eq!(DROP_D.load(), 1);
        assert_eq!(chan.len(), 0);
    })
    .unwrap();
}

#[test]
fn cancel_during_run() {
    future!(f, waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    task.run();
    let w = waker();
    w.wake();
    let task = chan.recv().unwrap();

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            drop(waker());
            assert_eq!(POLL.load(), 2);
            assert_eq!(SCHEDULE.load(), 1);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
            assert_eq!(chan.len(), 0);
        });

        thread::sleep(ms(100));

        handle.cancel();
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        drop(handle);
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        thread::sleep(ms(200));

        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 1);
        assert_eq!(DROP_S.load(), 1);
        assert_eq!(DROP_D.load(), 1);
        assert_eq!(chan.len(), 0);
    })
    .unwrap();
}

#[test]
fn wake_and_cancel_during_run() {
    future!(f, waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    task.run();
    let w = waker();
    w.wake_by_ref();
    let task = chan.recv().unwrap();

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            drop(waker());
            assert_eq!(POLL.load(), 2);
            assert_eq!(SCHEDULE.load(), 1);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
            assert_eq!(chan.len(), 0);
        });

        thread::sleep(ms(100));

        w.wake();
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        handle.cancel();
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        drop(handle);
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        thread::sleep(ms(200));

        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 1);
        assert_eq!(DROP_S.load(), 1);
        assert_eq!(DROP_D.load(), 1);
        assert_eq!(chan.len(), 0);
    })
    .unwrap();
}

#[test]
fn cancel_and_wake_during_run() {
    future!(f, waker, POLL, DROP_F);
    schedule!(s, chan, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    task.run();
    let w = waker();
    w.wake_by_ref();
    let task = chan.recv().unwrap();

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            drop(waker());
            assert_eq!(POLL.load(), 2);
            assert_eq!(SCHEDULE.load(), 1);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
            assert_eq!(chan.len(), 0);
        });

        thread::sleep(ms(100));

        handle.cancel();
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        drop(handle);
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        w.wake();
        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        assert_eq!(chan.len(), 0);

        thread::sleep(ms(200));

        assert_eq!(POLL.load(), 2);
        assert_eq!(SCHEDULE.load(), 1);
        assert_eq!(DROP_F.load(), 1);
        assert_eq!(DROP_S.load(), 1);
        assert_eq!(DROP_D.load(), 1);
        assert_eq!(chan.len(), 0);
    })
    .unwrap();
}
