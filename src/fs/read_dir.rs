use std::fs;

use std::pin::Pin;
use std::sync::Mutex;

use super::DirEntry;
use crate::future::Future;
use crate::io;
use crate::task::{blocking, Context, Poll};

/// A stream over entries in a directory.
///
/// This stream is returned by [`read_dir`] and yields items of type
/// [`io::Result`]`<`[`DirEntry`]`>`. Each [`DirEntry`] can then retrieve information like entry's
/// path or metadata.
///
/// This type is an async version of [`std::fs::ReadDir`].
///
/// [`read_dir`]: fn.read_dir.html
/// [`io::Result`]: https://doc.rust-lang.org/std/io/type.Result.html
/// [`DirEntry`]: struct.DirEntry.html
/// [`std::fs::ReadDir`]: https://doc.rust-lang.org/std/fs/struct.ReadDir.html
#[derive(Debug)]
pub struct ReadDir(Mutex<State>);

/// The state of an asynchronous `ReadDir`.
///
/// The `ReadDir` can be either idle or busy performing an asynchronous operation.
#[derive(Debug)]
enum State {
    Idle(Option<Inner>),
    Busy(blocking::JoinHandle<State>),
}

/// Inner representation of an asynchronous `DirEntry`.
#[derive(Debug)]
struct Inner {
    /// The blocking handle.
    read_dir: fs::ReadDir,

    /// The next item in the stream.
    item: Option<io::Result<DirEntry>>,
}

impl ReadDir {
    /// Creates an asynchronous `ReadDir` from a synchronous handle.
    pub(crate) fn new(inner: fs::ReadDir) -> ReadDir {
        ReadDir(Mutex::new(State::Idle(Some(Inner {
            read_dir: inner,
            item: None,
        }))))
    }
}

impl futures::Stream for ReadDir {
    type Item = io::Result<DirEntry>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let state = &mut *self.0.lock().unwrap();

        loop {
            match state {
                State::Idle(opt) => {
                    let inner = match opt.as_mut() {
                        None => return Poll::Ready(None),
                        Some(inner) => inner,
                    };

                    // Check if the operation has completed.
                    if let Some(res) = inner.item.take() {
                        return Poll::Ready(Some(res));
                    } else {
                        let mut inner = opt.take().unwrap();

                        // Start the operation asynchronously.
                        *state = State::Busy(blocking::spawn(async move {
                            match inner.read_dir.next() {
                                None => State::Idle(None),
                                Some(res) => {
                                    inner.item = Some(res.map(DirEntry::new));
                                    State::Idle(Some(inner))
                                }
                            }
                        }));
                    }
                }
                // Poll the asynchronous operation the file is currently blocked on.
                State::Busy(task) => *state = futures::ready!(Pin::new(task).poll(cx)),
            }
        }
    }
}
