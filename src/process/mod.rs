//! A module for working with processes.
//!
//! This module is mostly concerned with spawning and interacting with child processes, but it also
//! provides abort and exit for terminating the current process.
//!
//! This is an async version of [`std::process`].
//!
//! [`std::process`]: https://doc.rust-lang.org/std/process/index.html

// Re-export structs.
pub use std::process::{ExitStatus, Output};

// Re-export functions.
pub use std::process::{abort, exit, id};
