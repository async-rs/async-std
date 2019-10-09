//! Native threads.

mod spawn_task;

#[doc(inline)]
pub use std::thread::{spawn, JoinHandle};

pub use spawn_task::spawn_task;
