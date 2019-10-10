//! Native threads.

#[doc(inline)]
pub use std::thread::Result;
#[doc(inline)]
pub use std::thread::{current, panicking, park, park_timeout, sleep, spawn, yield_now};
#[doc(inline)]
pub use std::thread::{AccessError, Builder, JoinHandle, LocalKey, Thread, ThreadId};

pub use spawn_task::spawn_task;

mod spawn_task;
