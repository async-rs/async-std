//! Windows asynchronous process handling.ven
//!
//! Like with Unix we don't actually have a way of registering a process with an
//! IOCP object. As a result we similarly need another mechanism for getting a
//! signal when a process has exited. For now this is implemented with the
//! `RegisterWaitForSingleObject` function in the kernel32.dll.
//!
//! This strategy is the same that libuv takes and essentially just queues up a
//! wait for the process in a kernel32-specific thread pool. Once the object is
//! notified (e.g. the process exits) then we have a callback that basically
//! just completes a `Oneshot`.
//!
//! The `poll_exit` implementation will attempt to wait for the process in a
//! nonblocking fashion, but failing that it'll fire off a
//! `RegisterWaitForSingleObject` and then wait on the other end of the oneshot
//! from then on out.

use std::fmt;
use std::future::Future;
use std::io::{self, Read, Write};
use std::os::windows::prelude::*;
use std::os::windows::process::ExitStatusExt;
use std::pin::Pin;
use std::process;
use std::ptr;
use std::task::{Context, Poll};

use futures_channel::oneshot::{channel as oneshot, Receiver, Sender};
use futures_io::{AsyncRead, AsyncWrite};
use mio_named_pipes::NamedPipe;
use winapi::shared::minwindef::*;
use winapi::shared::winerror::*;
use winapi::um::handleapi::*;
use winapi::um::processthreadsapi::*;
use winapi::um::synchapi::*;
use winapi::um::threadpoollegacyapiset::*;
use winapi::um::winbase::*;
use winapi::um::winnt::*;

use crate::net::driver::Watcher;
use crate::process::{
    kill::{ChildDropGuard, Kill},
    Child as SpawnedChild, ChildStderr as SpawnedChildStderr, ChildStdin as SpawnedChildStdin,
    ChildStdout as SpawnedChildStdout, ExitStatus,
};

#[must_use = "futures do nothing unless polled"]
pub struct Child {
    child: process::Child,
    waiting: Option<Waiting>,
}

impl fmt::Debug for Child {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Child")
            .field("pid", &self.id())
            .field("child", &self.child)
            .field("waiting", &"..")
            .finish()
    }
}

struct Waiting {
    rx: Receiver<()>,
    wait_object: HANDLE,
    tx: *mut Option<Sender<()>>,
}

unsafe impl Sync for Waiting {}
unsafe impl Send for Waiting {}

pub(crate) fn spawn_child(cmd: &mut process::Command) -> io::Result<SpawnedChild> {
    let mut child = cmd.spawn()?;
    let stdin = stdio(child.stdin.take())?;
    let stdout = stdio(child.stdout.take())?;
    let stderr = stdio(child.stderr.take())?;

    Ok(SpawnedChild {
        child: ChildDropGuard::new(Child {
            child,
            waiting: None,
        }),
        stdin: stdin.map(|stdin| SpawnedChildStdin { inner: stdin }),
        stdout: stdout.map(|stdout| SpawnedChildStdout { inner: stdout }),
        stderr: stderr.map(|stderr| SpawnedChildStderr { inner: stderr }),
    })
}

impl Child {
    pub fn id(&self) -> u32 {
        self.child.id()
    }
}

impl Kill for Child {
    fn kill(&mut self) -> io::Result<()> {
        self.child.kill()
    }
}

impl Future for Child {
    type Output = io::Result<ExitStatus>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<<Self as Future>::Output> {
        loop {
            if let Some(ref mut w) = self.waiting {
                match Pin::new(&mut w.rx).poll(cx) {
                    Poll::Ready(Ok(())) => {}
                    Poll::Ready(Err(_)) => panic!("cancellation was not supposed to happen"),
                    Poll::Pending => return Poll::Pending,
                }
                return match try_wait(&self.child) {
                    Ok(status) => Poll::Ready(Ok(status.expect("not ready yet"))),
                    Err(err) => Poll::Ready(Err(err)),
                };
            }

            match try_wait(&self.child) {
                Ok(Some(e)) => return Poll::Ready(Ok(e)),
                Ok(None) => {}
                Err(err) => return Poll::Ready(Err(err)),
            }
            let (tx, rx) = oneshot();
            let ptr = Box::into_raw(Box::new(Some(tx)));
            let mut wait_object = ptr::null_mut();
            let rc = unsafe {
                RegisterWaitForSingleObject(
                    &mut wait_object,
                    self.child.as_raw_handle(),
                    Some(callback),
                    ptr as *mut _,
                    INFINITE,
                    WT_EXECUTEINWAITTHREAD | WT_EXECUTEONLYONCE,
                )
            };
            if rc == 0 {
                let err = io::Error::last_os_error();
                drop(unsafe { Box::from_raw(ptr) });
                return Poll::Ready(Err(err));
            }
            self.waiting = Some(Waiting {
                rx,
                wait_object,
                tx: ptr,
            });
        }
    }
}

impl Drop for Waiting {
    fn drop(&mut self) {
        unsafe {
            let rc = UnregisterWaitEx(self.wait_object, INVALID_HANDLE_VALUE);
            if rc == 0 {
                panic!("failed to unregister: {}", io::Error::last_os_error());
            }
            drop(Box::from_raw(self.tx));
        }
    }
}

unsafe extern "system" fn callback(ptr: PVOID, _timer_fired: BOOLEAN) {
    let complete = &mut *(ptr as *mut Option<Sender<()>>);
    let _ = complete.take().unwrap().send(());
}

pub fn try_wait(child: &process::Child) -> io::Result<Option<ExitStatus>> {
    unsafe {
        match WaitForSingleObject(child.as_raw_handle(), 0) {
            WAIT_OBJECT_0 => {}
            WAIT_TIMEOUT => return Ok(None),
            _ => return Err(io::Error::last_os_error()),
        }
        let mut status = 0;
        let rc = GetExitCodeProcess(child.as_raw_handle(), &mut status);
        if rc == FALSE {
            Err(io::Error::last_os_error())
        } else {
            Ok(Some(ExitStatus::from_raw(status)))
        }
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

pub type ChildStdin = Watcher<NamedPipe>;
pub type ChildStdout = Watcher<NamedPipe>;
pub type ChildStderr = Watcher<NamedPipe>;

fn stdio<T>(option: Option<T>) -> io::Result<Option<Watcher<NamedPipe>>>
where
    T: IntoRawHandle,
{
    let io = match option {
        Some(io) => io,
        None => return Ok(None),
    };
    let pipe = unsafe { NamedPipe::from_raw_handle(io.into_raw_handle()) };
    let io = Watcher::new(pipe);
    Ok(Some(io))
}
