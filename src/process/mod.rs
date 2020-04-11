//! A module for working with processes.
//!
//! This module is mostly concerned with spawning and interacting with child
//! processes, but it also provides [`abort`] and [`exit`] for terminating the
//! current process.
//!
//! # Spawning a process
//!
//! The [`Command`] struct is used to configure and spawn processes:
//!
//! ```no_run
//! # fn main() { async_std::task::block_on(async {
//! #
//! use async_std::process::Command;
//!
//! let output = Command::new("echo")
//!                      .arg("Hello world")
//!                      .output()
//!                      .await
//!                      .expect("Failed to execute command");
//!
//! assert_eq!(b"Hello world\n", output.stdout.as_slice());
//! #
//! # }) }
//! ```
//!
//! Several methods on [`Command`], such as [`spawn`] or [`output`], can be used
//! to spawn a process. In particular, [`output`] spawns the child process and
//! waits until the process terminates, while [`spawn`] will return a [`Child`]
//! that represents the spawned child process.
//!
//! # Handling I/O
//!
//! The [`stdout`], [`stdin`], and [`stderr`] of a child process can be
//! configured by passing an [`Stdio`] to the corresponding method on
//! [`Command`]. Once spawned, they can be accessed from the [`Child`]. For
//! example, piping output from one command into another command can be done
//! like so:
//!
//! ```no_run
//! # fn main() { async_std::task::block_on(async {
//! #
//! use async_std::process::{Command, Stdio};
//!
//! // stdout must be configured with `Stdio::piped` in order to use
//! // `echo_child.stdout`
//! let mut echo_child = Command::new("echo")
//!     .arg("Oh no, a tpyo!")
//!     .stdout(Stdio::piped())
//!     .spawn()
//!     .expect("Failed to start echo process");
//!
//! // Note that `echo_child` is moved here, but we won't be needing
//! // `echo_child` anymore
//! let echo_out = echo_child.stdout().take().expect("Failed to open echo stdout");
//!
//! let sed_child = Command::new("sed")
//!     .arg("s/tpyo/typo/")
//!     .stdin(Stdio::from(echo_out))
//!     .stdout(Stdio::piped())
//!     .spawn()
//!     .expect("Failed to start sed process");
//!
//! let output = sed_child.output().await.expect("Failed to wait on sed");
//! assert_eq!(b"Oh no, a typo!\n", output.stdout.as_slice());
//! #
//! # }) }
//! ```
//!
//!
//! Note that [`ChildStderr`] and [`ChildStdout`] implement [`Read`] and
//! [`ChildStdin`] implements [`Write`]:
//!
//! ```no_run
//! # fn main() { async_std::task::block_on(async {
//! #
//! use async_std::process::{Command, Stdio};
//! use async_std::prelude::*;
//!
//! let mut child = Command::new("/bin/cat")
//!     .stdin(Stdio::piped())
//!     .stdout(Stdio::piped())
//!     .spawn()
//!     .expect("failed to execute child");
//!
//! {
//!     // limited borrow of stdin
//!     let stdin = child.stdin.as_mut().expect("failed to get stdin");
//!     stdin.write_all(b"test").await.expect("failed to write to stdin");
//! }
//!
//! let output = child
//!     .output()
//!     .await
//!     .expect("failed to wait on child");
//!
//! assert_eq!(b"test", output.stdout.as_slice());
//! #
//! # }) }
//! ```
//!
//! # Caveats
//!
//! While similar to the standard library, this crate's `Child` type differs
//! importantly in the behavior of `drop`. In the standard library, a child
//! process will continue running after the instance of `std::process::Child`
//! is dropped. In this crate, however, because `async_std::process::Child` is a
//! future of the child's `ExitStatus`, a child process is terminated if
//! `async_std::process::Child` is dropped. The behavior of the standard library can
//! be regained with the `Child::forget` method.

use crate::io;
use crate::prelude::*;
use crate::task::{Context, Poll};

use std::ffi::OsStr;
use std::path::Path;
use std::pin::Pin;

use futures_io::AsyncRead;

#[doc(inline)]
pub use std::process::ExitStatus;

#[doc(inline)]
pub use std::process::Output;

#[doc(inline)]
pub use std::process::Stdio;

#[cfg(not(target_os = "windows"))]
#[path = "unix/mod.rs"]
mod imp;

#[cfg(not(target_os = "windows"))]
use std::os::unix::io::{AsRawFd, FromRawFd};

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod imp;

mod kill;

use kill::{ChildDropGuard, Kill};

/// Representation of a running or exited child process.
///
/// This structure is used to represent and manage child processes. A child
/// process is created via the [`Command`] struct, which configures the
/// spawning process and can itself be constructed using a builder-style
/// interface.
#[derive(Debug)]
pub struct Child {
    child: ChildDropGuard<imp::Child>,

    /// The handle for writing to the child's standard input (stdin), if it has
    /// been captured.
    pub stdin: Option<ChildStdin>,
    /// The handle for reading from the child's standard output (stdout), if it
    /// has been captured.
    pub stdout: Option<ChildStdout>,
    /// The handle for reading from the child's standard error (stderr), if it
    /// has been captured.
    pub stderr: Option<ChildStderr>,
}

impl Child {
    /// Returns the OS-assigned process identifier associated with this child.
    pub fn id(&self) -> u32 {
        self.child.inner.id()
    }

    /// Forces the child process to exit. If the child has already exited, an [`InvalidInput`]
    /// error is returned.
    ///
    /// The mapping to [`ErrorKind`]s is not part of the compatibility contract of the function,
    /// especially the [`Other`] kind might change to more specific kinds in the future.
    ///
    /// This is equivalent to sending a SIGKILL on Unix platforms.
    ///
    /// [`ErrorKind`]: ../io/enum.ErrorKind.html
    /// [`InvalidInput`]: ../io/enum.ErrorKind.html#variant.InvalidInput
    /// [`Other`]: ../io/enum.ErrorKind.html#variant.Other
    pub fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }

    /// Returns a handle for writing to the child's stdin, if it has been
    /// captured
    pub fn stdin(&mut self) -> &mut Option<ChildStdin> {
        &mut self.stdin
    }

    /// Returns a handle for writing to the child's stdout, if it has been
    /// captured
    pub fn stdout(&mut self) -> &mut Option<ChildStdout> {
        &mut self.stdout
    }

    /// Returns a handle for writing to the child's stderr, if it has been
    /// captured
    pub fn stderr(&mut self) -> &mut Option<ChildStderr> {
        &mut self.stderr
    }

    /// Returns a future that will resolve to an `Output`, containing the exit
    /// status, stdout, and stderr of the child process.
    ///
    /// The returned future will simultaneously waits for the child to exit and
    /// collect all remaining output on the stdout/stderr handles, returning an
    /// `Output` instance.
    ///
    /// The stdin handle to the child process, if any, will be closed before
    /// waiting. This helps avoid deadlock: it ensures that the child does not
    /// block waiting for input from the parent, while the parent waits for the
    /// child to exit.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent. In
    /// order to capture the output into this `Output` it is necessary to create
    /// new pipes between parent and child. Use `stdout(Stdio::piped())` or
    /// `stderr(Stdio::piped())`, respectively, when creating a `Command`.
    pub async fn output(mut self) -> io::Result<Output> {
        drop(self.stdin().take());

        let stdout = self.stdout().take();
        let stderr = self.stderr().take();

        let status_handle = &mut self;
        let stdout_handle = read_to_end(stdout);
        let stderr_handle = read_to_end(stderr);

        let (status, stdout, stderr) =
            async_macros::try_join!(status_handle, stdout_handle, stderr_handle).await?;

        Ok(Output {
            status,
            stdout,
            stderr,
        })
    }
}

impl Future for Child {
    type Output = io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        Pin::new(&mut self.child).poll(cx)
    }
}

/// A handle to a child process's standard input (stdin).
#[derive(Debug)]
pub struct ChildStdin {
    inner: imp::ChildStdin,
}

/// A handle to a child process's standard output (stdout).
#[derive(Debug)]
pub struct ChildStdout {
    inner: imp::ChildStdout,
}

/// A handle to a child process's stderr.
#[derive(Debug)]
pub struct ChildStderr {
    inner: imp::ChildStderr,
}

/// A  process builder, providing fine-grained control over how a new process should be spawned.
#[derive(Debug)]
pub struct Command {
    inner: std::process::Command,
}

#[cfg(not(target_os = "windows"))]
impl From<ChildStdin> for Stdio {
    /// Converts a `ChildStdin` into a `Stdio`
    fn from(child: ChildStdin) -> Stdio {
        unsafe { Stdio::from_raw_fd(child.inner.get_ref().as_raw_fd()) }
    }
}

#[cfg(not(target_os = "windows"))]
impl From<ChildStdout> for Stdio {
    /// Converts a `ChildStdout` into a `Stdio`
    fn from(child: ChildStdout) -> Stdio {
        unsafe { Stdio::from_raw_fd(child.inner.get_ref().as_raw_fd()) }
    }
}

#[cfg(not(target_os = "windows"))]
impl From<ChildStderr> for Stdio {
    /// Converts a `ChildStderr` into a `Stdio`
    fn from(child: ChildStderr) -> Stdio {
        unsafe { Stdio::from_raw_fd(child.inner.get_ref().as_raw_fd()) }
    }
}

#[cfg(not(target_os = "windows"))]
impl From<crate::fs::File> for Stdio {
    /// Converts a `File` into a `Stdio`
    fn from(file: crate::fs::File) -> Stdio {
        unsafe { Stdio::from_raw_fd(file.as_raw_fd()) }
    }
}

impl Command {
    /// Constructs a new `Command` for launching the program at
    /// path `program`, with the following default configuration:
    ///
    /// * No arguments to the program
    /// * Inherit the current process's environment
    /// * Inherit the current process's working directory
    /// * Inherit stdin/stdout/stderr for `spawn` or `status`, but create pipes for `output`
    ///
    /// Builder methods are provided to change these defaults and
    /// otherwise configure the process.
    ///
    /// If `program` is not an absolute path, the `PATH` will be searched in
    /// an OS-defined way.
    ///
    /// The search path to be used may be controlled by setting the
    /// `PATH` environment variable on the Command,
    /// but this has some implementation limitations on Windows.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```no_run
    /// use std::process::Command;
    ///
    /// Command::new("sh")
    ///         .spawn()
    ///         .expect("sh command failed to start");
    /// ```
    pub fn new<S: AsRef<OsStr>>(program: S) -> Command {
        Command {
            inner: std::process::Command::new(program),
        }
    }

    /// Adds an argument to pass to the program.
    ///
    /// Only one argument can be passed per use.
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.inner.arg(arg);
        self
    }

    /// Adds multiple arguments to pass to the program.
    ///
    /// To pass a single argument see [`arg`].
    ///
    /// [`arg`]: #method.arg
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.inner.args(args);
        self
    }

    /// Inserts or updates an environment variable mapping.
    ///
    /// Note that environment variable names are case-insensitive (but case-preserving) on Windows,
    /// and case-sensitive on all other platforms.    
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Command
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.env(key, val);
        self
    }

    /// Adds or updates multiple environment variable mappings.
    pub fn envs<I, K, V>(&mut self, vars: I) -> &mut Command
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        self.inner.envs(vars);
        self
    }

    /// Removes an environment variable mapping.
    pub fn env_remove<K: AsRef<OsStr>>(&mut self, key: K) -> &mut Command {
        self.inner.env_remove(key);
        self
    }

    /// Clears the entire environment map for the child process.
    pub fn env_clear(&mut self) -> &mut Command {
        self.inner.env_clear();
        self
    }

    /// Sets the working directory for the child process.
    ///
    /// # Platform-specific behavior
    ///
    /// If the program path is relative (e.g., `"./script.sh"`), it's ambiguous
    /// whether it should be interpreted relative to the parent's working
    /// directory or relative to `current_dir`. The behavior in this case is
    /// platform specific and unstable, and it's recommended to use
    /// [`canonicalize`] to get an absolute program path instead.
    ///
    /// [`canonicalize`]: ../fs/fn.canonicalize.html
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Command {
        self.inner.current_dir(dir);
        self
    }

    /// Configuration for the child process's standard input (stdin) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: struct.Stdio.html#method.inherit
    /// [`piped`]: struct.Stdio.html#method.piped
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.inner.stdin(cfg);
        self
    }

    /// Configuration for the child process's standard output (stdout) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: struct.Stdio.html#method.inherit
    /// [`piped`]: struct.Stdio.html#method.piped
    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.inner.stdout(cfg);
        self
    }

    /// Configuration for the child process's standard error (stderr) handle.
    ///
    /// Defaults to [`inherit`] when used with `spawn` or `status`, and
    /// defaults to [`piped`] when used with `output`.
    ///
    /// [`inherit`]: struct.Stdio.html#method.inherit
    /// [`piped`]: struct.Stdio.html#method.piped    
    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Command {
        self.inner.stderr(cfg);
        self
    }

    /// Executes the command as a child process, returning a handle to it.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    pub fn spawn(&mut self) -> io::Result<Child> {
        let child = imp::spawn_child(&mut self.inner)?;
        Ok(child)
    }

    /// Executes the command as a child process, waiting for it to finish and
    /// collecting all of its output.
    ///
    /// By default, stdout and stderr are captured (and used to provide the
    /// resulting output). Stdin is not inherited from the parent and any
    /// attempt by the child process to read from the stdin stream will result
    /// in the stream immediately closing.    
    pub async fn output(&mut self) -> io::Result<Output> {
        self.stdout(Stdio::piped());
        self.stderr(Stdio::piped());
        let child = imp::spawn_child(&mut self.inner)?;

        let output = child.output().await?;

        Ok(output)
    }

    /// Executes a command as a child process, waiting for it to finish and
    /// collecting its exit status.
    ///
    /// By default, stdin, stdout and stderr are inherited from the parent.
    pub async fn status(&mut self) -> io::Result<ExitStatus> {
        let child = imp::spawn_child(&mut self.inner)?;

        let status = child.await?;

        Ok(status)
    }
}

impl From<std::process::Command> for Command {
    fn from(inner: std::process::Command) -> Self {
        Command { inner }
    }
}

async fn read_to_end<T: AsyncRead + Unpin>(source: Option<T>) -> io::Result<Vec<u8>> {
    match source {
        Some(mut source) => {
            let mut res = Vec::new();
            source.read_to_end(&mut res).await?;
            Ok(res)
        }
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Output, Stdio};
    use crate::io::prelude::*;
    use crate::io::ErrorKind;
    use crate::task;

    use std::str;

    #[test]
    fn smoke() {
        task::block_on(async {
            let p = if cfg!(target_os = "windows") {
                Command::new("cmd").args(&["/C", "exit 0"]).spawn()
            } else {
                Command::new("true").spawn()
            };
            assert!(p.is_ok());
            let p = p.unwrap();
            assert!(p.await.unwrap().success());
        })
    }

    #[test]
    fn smoke_failure() {
        task::block_on(async {
            match Command::new("if-this-is-a-binary-then-the-world-has-ended").spawn() {
                Ok(..) => panic!(),
                Err(..) => {}
            }
        })
    }

    #[test]
    fn exit_reported_right() {
        task::block_on(async {
            let p = if cfg!(target_os = "windows") {
                Command::new("cmd").args(&["/C", "exit 1"]).spawn()
            } else {
                Command::new("false").spawn()
            };
            assert!(p.is_ok());
            let p = p.unwrap();
            assert!(p.await.unwrap().code() == Some(1));
        })
    }

    #[test]
    #[cfg(unix)]
    fn signal_reported_right() {
        use crate::os::unix::process::ExitStatusExt;

        task::block_on(async {
            let mut p = Command::new("/bin/sh")
                .arg("-c")
                .arg("read a")
                .stdin(Stdio::piped())
                .spawn()
                .unwrap();
            p.kill().unwrap();
            match p.await.unwrap().signal() {
                Some(9) => {}
                result => panic!("not terminated by signal 9 (instead, {:?})", result),
            }
        })
    }

    pub async fn run_output(mut cmd: Command) -> String {
        let p = cmd.spawn();
        assert!(p.is_ok());
        let mut p = p.unwrap();
        assert!(p.stdout.is_some());
        let mut ret = String::new();
        p.stdout
            .as_mut()
            .unwrap()
            .read_to_string(&mut ret)
            .await
            .unwrap();
        assert!(p.await.unwrap().success());
        return ret;
    }

    #[test]
    fn stdout_works() {
        task::block_on(async {
            if cfg!(target_os = "windows") {
                let mut cmd = Command::new("cmd");
                cmd.args(&["/C", "echo foobar"]).stdout(Stdio::piped());
                assert_eq!(run_output(cmd).await, "foobar\r\n");
            } else {
                let mut cmd = Command::new("echo");
                cmd.arg("foobar").stdout(Stdio::piped());
                assert_eq!(run_output(cmd).await, "foobar\n");
            }
        });
    }

    #[test]
    #[cfg_attr(any(windows), ignore)]
    fn set_current_dir_works() {
        task::block_on(async {
            let mut cmd = Command::new("/bin/sh");
            cmd.arg("-c")
                .arg("pwd")
                .current_dir("/")
                .stdout(Stdio::piped());
            assert_eq!(run_output(cmd).await, "/\n");
        });
    }

    #[test]
    #[cfg_attr(any(windows), ignore)]
    fn stdin_works() {
        task::block_on(async {
            let mut p = Command::new("/bin/sh")
                .arg("-c")
                .arg("read line; echo $line")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            p.stdin
                .as_mut()
                .unwrap()
                .write("foobar".as_bytes())
                .await
                .unwrap();
            drop(p.stdin.take());
            let mut out = String::new();
            p.stdout
                .as_mut()
                .unwrap()
                .read_to_string(&mut out)
                .await
                .unwrap();
            assert!(p.await.unwrap().success());
            assert_eq!(out, "foobar\n");
        });
    }

    #[test]
    fn test_process_status() {
        task::block_on(async {
            let mut status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(&["/C", "exit 1"])
                    .status()
                    .await
                    .unwrap()
            } else {
                Command::new("false").status().await.unwrap()
            };
            assert!(status.code() == Some(1));

            status = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(&["/C", "exit 0"])
                    .status()
                    .await
                    .unwrap()
            } else {
                Command::new("true").status().await.unwrap()
            };
            assert!(status.success());
        });
    }

    #[test]
    fn test_process_output_fail_to_start() {
        task::block_on(async {
            match Command::new("/no-binary-by-this-name-should-exist")
                .output()
                .await
            {
                Err(e) => assert_eq!(e.kind(), ErrorKind::NotFound),
                Ok(..) => panic!(),
            }
        });
    }

    #[test]
    fn test_process_output_output() {
        task::block_on(async {
            let Output {
                status,
                stdout,
                stderr,
            } = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(&["/C", "echo hello"])
                    .output()
                    .await
                    .unwrap()
            } else {
                Command::new("echo").arg("hello").output().await.unwrap()
            };
            let output_str = str::from_utf8(&stdout).unwrap();

            assert!(status.success());
            assert_eq!(output_str.trim().to_string(), "hello");
            assert_eq!(stderr, Vec::new());
        });
    }

    #[test]
    fn test_process_output_error() {
        task::block_on(async {
            let Output {
                status,
                stdout,
                stderr,
            } = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(&["/C", "mkdir ."])
                    .output()
                    .await
                    .unwrap()
            } else {
                Command::new("mkdir").arg("./").output().await.unwrap()
            };

            assert!(status.code() == Some(1));
            assert_eq!(stdout, Vec::new());
            assert!(!stderr.is_empty());
        });
    }

    #[test]
    fn test_finish_once() {
        task::block_on(async {
            let prog = if cfg!(target_os = "windows") {
                Command::new("cmd").args(&["/C", "exit 1"]).spawn().unwrap()
            } else {
                Command::new("false").spawn().unwrap()
            };
            assert!(prog.await.unwrap().code() == Some(1));
        });
    }

    #[test]
    fn test_wait_with_output_once() {
        task::block_on(async {
            let prog = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(&["/C", "echo hello"])
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap()
            } else {
                Command::new("echo")
                    .arg("hello")
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap()
            };

            let Output {
                status,
                stdout,
                stderr,
            } = prog.wait_with_output().unwrap();
            let output_str = str::from_utf8(&stdout).unwrap();

            assert!(status.success());
            assert_eq!(output_str.trim().to_string(), "hello");
            assert_eq!(stderr, Vec::new());
        });
    }

    #[cfg(all(unix, not(target_os = "android")))]
    pub fn env_cmd() -> Command {
        Command::new("env")
    }
    #[cfg(target_os = "android")]
    pub fn env_cmd() -> Command {
        let mut cmd = Command::new("/system/bin/sh");
        cmd.arg("-c").arg("set");
        cmd
    }

    #[cfg(windows)]
    pub fn env_cmd() -> Command {
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg("set");
        cmd
    }

    #[test]
    fn test_override_env() {
        use std::env;

        // In some build environments (such as chrooted Nix builds), `env` can
        // only be found in the explicitly-provided PATH env variable, not in
        // default places such as /bin or /usr/bin. So we need to pass through
        // PATH to our sub-process.
        task::block_on(async {
            let mut cmd = env_cmd();
            cmd.env_clear().env("RUN_TEST_NEW_ENV", "123");
            if let Some(p) = env::var_os("PATH") {
                cmd.env("PATH", &p);
            }
            let result = cmd.output().await.unwrap();
            let output = String::from_utf8_lossy(&result.stdout).to_string();

            assert!(
                output.contains("RUN_TEST_NEW_ENV=123"),
                "didn't find RUN_TEST_NEW_ENV inside of:\n\n{}",
                output
            );
        });
    }

    #[test]
    fn test_add_to_env() {
        task::block_on(async {
            let result = env_cmd()
                .env("RUN_TEST_NEW_ENV", "123")
                .output()
                .await
                .unwrap();
            let output = String::from_utf8_lossy(&result.stdout).to_string();

            assert!(
                output.contains("RUN_TEST_NEW_ENV=123"),
                "didn't find RUN_TEST_NEW_ENV inside of:\n\n{}",
                output
            );
        });
    }

    #[test]
    fn test_capture_env_at_spawn() {
        use std::env;

        task::block_on(async {
            let mut cmd = env_cmd();
            cmd.env("RUN_TEST_NEW_ENV1", "123");

            // This variable will not be present if the environment has already
            // been captured above.
            env::set_var("RUN_TEST_NEW_ENV2", "456");
            let result = cmd.output().await.unwrap();
            env::remove_var("RUN_TEST_NEW_ENV2");

            let output = String::from_utf8_lossy(&result.stdout).to_string();

            assert!(
                output.contains("RUN_TEST_NEW_ENV1=123"),
                "didn't find RUN_TEST_NEW_ENV1 inside of:\n\n{}",
                output
            );
            assert!(
                output.contains("RUN_TEST_NEW_ENV2=456"),
                "didn't find RUN_TEST_NEW_ENV2 inside of:\n\n{}",
                output
            );
        });
    }

    #[test]
    fn test_interior_nul_in_progname_is_error() {
        match Command::new("has-some-\0\0s-inside").spawn() {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_interior_nul_in_arg_is_error() {
        match Command::new("echo").arg("has-some-\0\0s-inside").spawn() {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_interior_nul_in_args_is_error() {
        match Command::new("echo")
            .args(&["has-some-\0\0s-inside"])
            .spawn()
        {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_interior_nul_in_current_dir_is_error() {
        match Command::new("echo")
            .current_dir("has-some-\0\0s-inside")
            .spawn()
        {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_interior_nul_in_env_key_is_error() {
        match env_cmd().env("has-some-\0\0s-inside", "value").spawn() {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    #[test]
    fn test_interior_nul_in_env_value_is_error() {
        match env_cmd().env("key", "has-some-\0\0s-inside").spawn() {
            Err(e) => assert_eq!(e.kind(), ErrorKind::InvalidInput),
            Ok(_) => panic!(),
        }
    }

    /// Tests that process creation flags work by debugging a process.
    /// Other creation flags make it hard or impossible to detect
    /// behavioral changes in the process.
    #[test]
    #[cfg(windows)]
    fn test_creation_flags() {
        use crate::os::windows::process::CommandExt;
        use crate::sys::c::{BOOL, DWORD, INFINITE};
        #[repr(C, packed)]
        struct DEBUG_EVENT {
            pub event_code: DWORD,
            pub process_id: DWORD,
            pub thread_id: DWORD,
            // This is a union in the real struct, but we don't
            // need this data for the purposes of this test.
            pub _junk: [u8; 164],
        }

        extern "system" {
            fn WaitForDebugEvent(lpDebugEvent: *mut DEBUG_EVENT, dwMilliseconds: DWORD) -> BOOL;
            fn ContinueDebugEvent(
                dwProcessId: DWORD,
                dwThreadId: DWORD,
                dwContinueStatus: DWORD,
            ) -> BOOL;
        }

        const DEBUG_PROCESS: DWORD = 1;
        const EXIT_PROCESS_DEBUG_EVENT: DWORD = 5;
        const DBG_EXCEPTION_NOT_HANDLED: DWORD = 0x80010001;

        let mut child = Command::new("cmd")
            .creation_flags(DEBUG_PROCESS)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"exit\r\n").unwrap();
        let mut events = 0;
        let mut event = DEBUG_EVENT {
            event_code: 0,
            process_id: 0,
            thread_id: 0,
            _junk: [0; 164],
        };
        loop {
            if unsafe { WaitForDebugEvent(&mut event as *mut DEBUG_EVENT, INFINITE) } == 0 {
                panic!("WaitForDebugEvent failed!");
            }
            events += 1;

            if event.event_code == EXIT_PROCESS_DEBUG_EVENT {
                break;
            }

            if unsafe {
                ContinueDebugEvent(event.process_id, event.thread_id, DBG_EXCEPTION_NOT_HANDLED)
            } == 0
            {
                panic!("ContinueDebugEvent failed!");
            }
        }
        assert!(events > 0);
    }

    #[test]
    fn test_command_implements_send() {
        fn take_send_type<T: Send>(_: T) {}
        take_send_type(Command::new(""))
    }
}
