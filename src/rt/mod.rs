//! The runtime.

use std::thread;

use once_cell::sync::Lazy;

use crate::future;

/// Dummy runtime struct.
pub struct Runtime {}

/// The global runtime.
pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    // Create an executor thread pool.
    let num_threads = num_cpus::get().max(1);
    for _ in 0..num_threads {
        thread::Builder::new()
            .name("async-std/runtime".to_string())
            .spawn(|| smol::run(future::pending::<()>()))
            .expect("cannot start a runtime thread");
    }
    Runtime {}
});
