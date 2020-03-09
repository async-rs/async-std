//! Unix handling of child processes
//!
//! Right now the only "fancy" thing about this is how we implement the
//! `Future` implementation on `Child` to get the exit status. Unix offers
//! no way to register a child with epoll, and the only real way to get a
//! notification when a process exits is the SIGCHLD signal.
//!
//! Signal handling in general is *super* hairy and complicated, and it's even
//! more complicated here with the fact that signals are coalesced, so we may
//! not get a SIGCHLD-per-child.
//!
//! Our best approximation here is to check *all spawned processes* for all
//! SIGCHLD signals received. To do that we create a stream of signals, using `signal-hook`.
//!
//! Later when we poll the process's exit status we simply check to see if a
//! SIGCHLD has happened since we last checked, and while that returns "yes" we
//! keep trying.
//!
//! Note that this means that this isn't really scalable, but then again
//! processes in general aren't scalable (e.g. millions) so it shouldn't be that
//! bad in theory...

mod orphan;
mod reap;

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::pin::Pin;
use std::process;
use std::task::{Context, Poll};

use async_signals::Signals;
use futures_io::{AsyncRead, AsyncWrite};
use mio::event::Evented;
use mio::unix::{EventedFd, UnixReady};
use mio::{Poll as MioPoll, PollOpt, Ready, Token};

use self::{
    orphan::{AtomicOrphanQueue, OrphanQueue, Wait},
    reap::Reaper,
};

use crate::net::driver::Watcher;
use crate::prelude::*;
use crate::process::{
    kill::Kill, Child as SpawnedChild, ChildStderr as SpawnedChildStderr,
    ChildStdin as SpawnedChildStdin, ChildStdout as SpawnedChildStdout, ExitStatus,
};

lazy_static::lazy_static! {
    static ref ORPHAN_QUEUE: AtomicOrphanQueue<process::Child> = AtomicOrphanQueue::new();
}

pub type ChildStdin = Watcher<Fd<process::ChildStdin>>;
pub type ChildStdout = Watcher<Fd<process::ChildStdout>>;
pub type ChildStderr = Watcher<Fd<process::ChildStderr>>;

impl Wait for process::Child {
    fn id(&self) -> u32 {
        self.id()
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.try_wait()
    }
}

impl Kill for process::Child {
    fn kill(&mut self) -> io::Result<()> {
        self.kill()
    }
}

pub(crate) struct GlobalOrphanQueue;

impl fmt::Debug for GlobalOrphanQueue {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        ORPHAN_QUEUE.fmt(fmt)
    }
}

impl OrphanQueue<process::Child> for GlobalOrphanQueue {
    fn push_orphan(&self, orphan: process::Child) {
        ORPHAN_QUEUE.push_orphan(orphan)
    }

    fn reap_orphans(&self) {
        ORPHAN_QUEUE.reap_orphans()
    }
}

#[must_use = "futures do nothing unless polled"]
pub struct Child {
    pub(crate) inner: Reaper<process::Child, GlobalOrphanQueue, Signals>,
}

impl fmt::Debug for Child {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Child")
            .field("pid", &self.inner.id())
            .finish()
    }
}

pub(crate) fn spawn_child(cmd: &mut process::Command) -> io::Result<SpawnedChild> {
    let mut child = cmd.spawn()?;
    let stdin = stdio(child.stdin.take())?;
    let stdout = stdio(child.stdout.take())?;
    let stderr = stdio(child.stderr.take())?;

    let signal = Signals::new(&[libc::SIGCHLD])?;
    Ok(SpawnedChild {
        child: Child {
            inner: Reaper::new(child, GlobalOrphanQueue, signal),
        },
        stdin: stdin.map(|stdin| SpawnedChildStdin { inner: stdin }),
        stdout: stdout.map(|stdout| SpawnedChildStdout { inner: stdout }),
        stderr: stderr.map(|stderr| SpawnedChildStderr { inner: stderr }),
    })
}

impl Child {
    pub fn id(&self) -> u32 {
        self.inner.id()
    }
}

impl Kill for Child {
    fn kill(&mut self) -> io::Result<()> {
        self.inner.kill()
    }
}

impl Future for Child {
    type Output = io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        let inner = Pin::new(&mut self.inner);
        inner.poll(cx)
    }
}

impl AsyncWrite for SpawnedChildStdin {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.inner.poll_write_with_mut(cx, |inner| inner.write(buf))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.inner.poll_write_with_mut(cx, |inner| inner.flush())
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for SpawnedChildStdout {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.inner.poll_read_with_mut(cx, |inner| inner.read(buf))
    }
}

impl AsyncRead for SpawnedChildStderr {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.inner.poll_read_with_mut(cx, |inner| inner.read(buf))
    }
}

#[derive(Debug)]
pub struct Fd<T>(T);

impl<T> Evented for Fd<T>
where
    T: AsRawFd,
{
    fn register(
        &self,
        poll: &MioPoll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).register(poll, token, interest | UnixReady::hup(), opts)
    }

    fn reregister(
        &self,
        poll: &MioPoll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).reregister(poll, token, interest | UnixReady::hup(), opts)
    }

    fn deregister(&self, poll: &MioPoll) -> io::Result<()> {
        EventedFd(&self.as_raw_fd()).deregister(poll)
    }
}

impl<T: io::Read> io::Read for Fd<T> {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}

impl<T: io::Write> io::Write for Fd<T> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<T> AsRawFd for Fd<T>
where
    T: AsRawFd,
{
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

fn stdio<T>(option: Option<T>) -> io::Result<Option<Watcher<Fd<T>>>>
where
    T: AsRawFd,
{
    let io = match option {
        Some(io) => io,
        None => return Ok(None),
    };

    // Set the fd to nonblocking before we pass it to the event loop
    unsafe {
        let fd = io.as_raw_fd();
        let r = libc::fcntl(fd, libc::F_GETFL);
        if r == -1 {
            return Err(io::Error::last_os_error());
        }
        let r = libc::fcntl(fd, libc::F_SETFL, r | libc::O_NONBLOCK);
        if r == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    let io = Watcher::new(Fd(io));

    Ok(Some(io))
}
