use crate::io;
use crate::prelude::*;

use std::pin::Pin;
use std::task::{Context, Poll};

/// A drop guard which ensures the child process is killed on drop to maintain
/// the contract of dropping a Future leads to "cancellation".
#[derive(Debug)]
pub(crate) struct ChildDropGuard<T: Kill> {
    inner: T,
    kill_on_drop: bool,
}

impl<T: Kill> ChildDropGuard<T> {
    pub(crate) fn new(inner: T) -> Self {
        Self {
            inner,
            kill_on_drop: true,
        }
    }

    pub(crate) fn forget(&mut self) {
        self.kill_on_drop = false;
    }
}

impl<T: Kill> Kill for ChildDropGuard<T> {
    fn kill(&mut self) -> io::Result<()> {
        let ret = self.inner.kill();

        if ret.is_ok() {
            self.kill_on_drop = false;
        }

        ret
    }
}

impl<T: Kill> Drop for ChildDropGuard<T> {
    fn drop(&mut self) {
        if self.kill_on_drop {
            drop(self.kill());
        }
    }
}

impl<T: Future + Kill + Unpin> Future for ChildDropGuard<T> {
    type Output = T::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let ret = Pin::new(&mut self.inner).poll(cx);

        if let Poll::Ready(_) = ret {
            // Avoid the overhead of trying to kill a reaped process
            self.kill_on_drop = false;
        }

        ret
    }
}

/// An interface for killing a running process.
pub(crate) trait Kill {
    /// Forcefully kill the process.
    fn kill(&mut self) -> io::Result<()>;
}

impl<'a, T: 'a + Kill> Kill for &'a mut T {
    fn kill(&mut self) -> io::Result<()> {
        (**self).kill()
    }
}
