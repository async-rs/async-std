//! Unix-specific extensions to primitives in the `std::process` module.

cfg_not_docs! {
    pub use std::os::unix::process::ExitStatusExt;
}

cfg_docs! {
    /// Unix-specific extensions to [`process::ExitStatus`].
    pub trait ExitStatusExt {
        /// Creates a new `ExitStatus` from the raw underlying `i32` return value of
        /// a process.
        fn from_raw(raw: i32) -> Self;

        /// If the process was terminated by a signal, returns that signal.
        fn signal(&self) -> Option<i32>;
    }
}
