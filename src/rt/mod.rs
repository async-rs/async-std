//! The runtime.

use std::thread;

use once_cell::sync::Lazy;

use crate::utils::abort_on_panic;

pub use reactor::{Reactor, Watcher};
pub use runtime::Runtime;

mod monitor;
mod reactor;
mod runtime;

/// The global runtime.
pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    thread::Builder::new()
        .name("async-std/runtime".to_string())
        .spawn(|| abort_on_panic(|| RUNTIME.run()))
        .expect("cannot start a runtime thread");

    Runtime::new()
});

pub fn scale_up() {
    RUNTIME.scale_up();
}

pub fn scale_down() {
    RUNTIME.scale_down();
}
