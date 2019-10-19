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

use std::io;
use std::pin::Pin;
use std::task::Context;
use crate::future::Future;
use crate::task::Poll;
use std::ffi::OsStr;

struct Child {
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    stderr: Option<ChildStderr>,
}

impl Child {
    fn id(&self) -> u32 {
        unimplemented!("need to do");
    }
    fn kill(&mut self) -> io::Result<()> {
        unimplemented!();
    }
    async fn output(self) -> io::Result<Output> {
        unimplemented!();
    }
}

impl Future for Child {
    type Output = io::Result<ExitStatus>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        unimplemented!();
    }

}

struct ChildStdin;
struct ChildStdout;
struct ChildStderr;

struct Command;

impl Command {
    fn new<S: AsRef<OsStr>>(program: S) -> Command {
        unimplemented!();
    }
    /// ```
    /// let child = Command::new("ls").spawn();
    /// let future = child.expect("failed to spawn child");
    /// let result = future.await?;
    /// assert!(!result.empty());
    /// assert!(false);
    /// ```
    fn spawn(&mut self) -> io::Result<Child> {
        unimplemented!();
    }
}
