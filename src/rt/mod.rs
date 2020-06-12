//! The runtime.

use std::env;
use std::thread;

use once_cell::sync::Lazy;

use crate::future;

/// Dummy runtime struct.
pub struct Runtime {}

/// The global runtime.
pub static RUNTIME: Lazy<Runtime> = Lazy::new(|| {
    // Create an executor thread pool.

    let thread_count = env::var("ASYNC_STD_THREAD_COUNT")
        .map(|env| {
            env.parse()
                .expect("ASYNC_STD_THREAD_COUNT must be a number")
        })
        .unwrap_or_else(|_| num_cpus::get())
        .max(1);

    let thread_name = env::var("ASYNC_STD_THREAD_NAME").unwrap_or("async-std/runtime".to_string());

    for _ in 0..thread_count {
        thread::Builder::new()
            .name(thread_name.clone())
            .spawn(|| crate::task::block_on(future::pending::<()>()))
            .expect("cannot start a runtime thread");
    }
    Runtime {}
});
