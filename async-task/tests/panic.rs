#![feature(async_await)]

use std::future::Future;
use std::panic::catch_unwind;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread;
use std::time::Duration;

use async_task::Task;
use crossbeam::atomic::AtomicCell;
use futures::executor::block_on;
use futures::future;
use lazy_static::lazy_static;

// Creates a future with event counters.
//
// Usage: `future!(f, POLL, DROP)`
//
// The future `f` sleeps for 200 ms and then panics.
// When it gets polled, `POLL` is incremented.
// When it gets dropped, `DROP` is incremented.
macro_rules! future {
    ($name:pat, $poll:ident, $drop:ident) => {
        lazy_static! {
            static ref $poll: AtomicCell<usize> = AtomicCell::new(0);
            static ref $drop: AtomicCell<usize> = AtomicCell::new(0);
        }

        let $name = {
            struct Fut(Box<i32>);

            impl Future for Fut {
                type Output = ();

                fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
                    $poll.fetch_add(1);
                    thread::sleep(ms(200));
                    panic!()
                }
            }

            impl Drop for Fut {
                fn drop(&mut self) {
                    $drop.fetch_add(1);
                }
            }

            Fut(Box::new(0))
        };
    };
}

// Creates a schedule function with event counters.
//
// Usage: `schedule!(s, SCHED, DROP)`
//
// The schedule function `s` does nothing.
// When it gets invoked, `SCHED` is incremented.
// When it gets dropped, `DROP` is incremented.
macro_rules! schedule {
    ($name:pat, $sched:ident, $drop:ident) => {
        lazy_static! {
            static ref $sched: AtomicCell<usize> = AtomicCell::new(0);
            static ref $drop: AtomicCell<usize> = AtomicCell::new(0);
        }

        let $name = {
            struct Guard(Box<i32>);

            impl Drop for Guard {
                fn drop(&mut self) {
                    $drop.fetch_add(1);
                }
            }

            let guard = Guard(Box::new(0));
            move |_task: Task<_>| {
                &guard;
                $sched.fetch_add(1);
            }
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
fn cancel_during_run() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            assert_eq!(POLL.load(), 1);
            assert_eq!(SCHEDULE.load(), 0);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
        });

        thread::sleep(ms(100));

        handle.cancel();
        assert_eq!(POLL.load(), 1);
        assert_eq!(SCHEDULE.load(), 0);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);

        drop(handle);
        assert_eq!(POLL.load(), 1);
        assert_eq!(SCHEDULE.load(), 0);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
    })
    .unwrap();
}

#[test]
fn run_and_join() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    assert!(catch_unwind(|| task.run()).is_err());
    assert_eq!(POLL.load(), 1);
    assert_eq!(SCHEDULE.load(), 0);
    assert_eq!(DROP_F.load(), 1);
    assert_eq!(DROP_S.load(), 0);
    assert_eq!(DROP_D.load(), 0);

    assert!(block_on(handle).is_none());
    assert_eq!(POLL.load(), 1);
    assert_eq!(SCHEDULE.load(), 0);
    assert_eq!(DROP_F.load(), 1);
    assert_eq!(DROP_S.load(), 1);
    assert_eq!(DROP_D.load(), 1);
}

#[test]
fn try_join_and_run_and_join() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, mut handle, f, s, DROP_D);

    block_on(future::select(&mut handle, future::ready(())));
    assert_eq!(POLL.load(), 0);
    assert_eq!(SCHEDULE.load(), 0);
    assert_eq!(DROP_F.load(), 0);
    assert_eq!(DROP_S.load(), 0);
    assert_eq!(DROP_D.load(), 0);

    assert!(catch_unwind(|| task.run()).is_err());
    assert_eq!(POLL.load(), 1);
    assert_eq!(SCHEDULE.load(), 0);
    assert_eq!(DROP_F.load(), 1);
    assert_eq!(DROP_S.load(), 0);
    assert_eq!(DROP_D.load(), 0);

    assert!(block_on(handle).is_none());
    assert_eq!(POLL.load(), 1);
    assert_eq!(SCHEDULE.load(), 0);
    assert_eq!(DROP_F.load(), 1);
    assert_eq!(DROP_S.load(), 1);
    assert_eq!(DROP_D.load(), 1);
}

#[test]
fn join_during_run() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            assert_eq!(POLL.load(), 1);
            assert_eq!(SCHEDULE.load(), 0);
            assert_eq!(DROP_F.load(), 1);

            thread::sleep(ms(100));
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
        });

        thread::sleep(ms(100));

        assert!(block_on(handle).is_none());
        assert_eq!(POLL.load(), 1);
        assert_eq!(SCHEDULE.load(), 0);
        assert_eq!(DROP_F.load(), 1);

        thread::sleep(ms(100));
        assert_eq!(DROP_S.load(), 1);
        assert_eq!(DROP_D.load(), 1);
    })
    .unwrap();
}

#[test]
fn try_join_during_run() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, mut handle, f, s, DROP_D);

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            assert_eq!(POLL.load(), 1);
            assert_eq!(SCHEDULE.load(), 0);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
        });

        thread::sleep(ms(100));

        block_on(future::select(&mut handle, future::ready(())));
        assert_eq!(POLL.load(), 1);
        assert_eq!(SCHEDULE.load(), 0);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
        drop(handle);
    })
    .unwrap();
}

#[test]
fn drop_handle_during_run() {
    future!(f, POLL, DROP_F);
    schedule!(s, SCHEDULE, DROP_S);
    task!(task, handle, f, s, DROP_D);

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            assert!(catch_unwind(|| task.run()).is_err());
            assert_eq!(POLL.load(), 1);
            assert_eq!(SCHEDULE.load(), 0);
            assert_eq!(DROP_F.load(), 1);
            assert_eq!(DROP_S.load(), 1);
            assert_eq!(DROP_D.load(), 1);
        });

        thread::sleep(ms(100));

        drop(handle);
        assert_eq!(POLL.load(), 1);
        assert_eq!(SCHEDULE.load(), 0);
        assert_eq!(DROP_F.load(), 0);
        assert_eq!(DROP_S.load(), 0);
        assert_eq!(DROP_D.load(), 0);
    })
    .unwrap();
}
