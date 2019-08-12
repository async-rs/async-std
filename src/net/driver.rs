use std::fmt;
use std::io::{self, prelude::*};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use futures::io::{AsyncRead, AsyncWrite};
use lazy_static::lazy_static;
use mio::{self, Evented};
use slab::Slab;

use crate::task::{Context, Poll, Waker};
use crate::utils::abort_on_panic;

/// Data associated with a registered I/O handle.
#[derive(Debug)]
struct Entry {
    /// A unique identifier.
    token: mio::Token,

    /// Indicates whether this I/O handle is ready for reading, writing, or if it is disconnected.
    readiness: AtomicUsize,

    /// Tasks that are blocked on reading from this I/O handle.
    readers: Mutex<Vec<Waker>>,

    /// Thasks that are blocked on writing to this I/O handle.
    writers: Mutex<Vec<Waker>>,
}

/// The state of a networking driver.
struct Reactor {
    /// A mio instance that polls for new events.
    poller: mio::Poll,

    /// A collection of registered I/O handles.
    entries: Mutex<Slab<Arc<Entry>>>,

    /// Dummy I/O handle that is only used to wake up the polling thread.
    notify_reg: (mio::Registration, mio::SetReadiness),

    /// An identifier for the notification handle.
    notify_token: mio::Token,
}

impl Reactor {
    /// Creates a new reactor for polling I/O events.
    fn new() -> io::Result<Reactor> {
        let poller = mio::Poll::new()?;
        let notify_reg = mio::Registration::new2();

        let mut reactor = Reactor {
            poller,
            entries: Mutex::new(Slab::new()),
            notify_reg,
            notify_token: mio::Token(0),
        };

        // Register a dummy I/O handle for waking up the polling thread.
        let entry = reactor.register(&reactor.notify_reg.0)?;
        reactor.notify_token = entry.token;

        Ok(reactor)
    }

    /// Registers an I/O event source and returns its associated entry.
    fn register(&self, source: &dyn Evented) -> io::Result<Arc<Entry>> {
        let mut entries = self.entries.lock().unwrap();

        // Reserve a vacant spot in the slab and use its key as the token value.
        let vacant = entries.vacant_entry();
        let token = mio::Token(vacant.key());

        // Allocate an entry and insert it into the slab.
        let entry = Arc::new(Entry {
            token,
            readiness: AtomicUsize::new(mio::Ready::empty().as_usize()),
            readers: Mutex::new(Vec::new()),
            writers: Mutex::new(Vec::new()),
        });
        vacant.insert(entry.clone());

        // Register the I/O event source in the poller.
        let interest = mio::Ready::all();
        let opts = mio::PollOpt::edge();
        self.poller.register(source, token, interest, opts)?;

        Ok(entry)
    }

    /// Deregisters an I/O event source associated with an entry.
    fn deregister(&self, source: &dyn Evented, entry: &Entry) -> io::Result<()> {
        // Deregister the I/O object from the mio instance.
        self.poller.deregister(source)?;

        // Remove the entry associated with the I/O object.
        self.entries.lock().unwrap().remove(entry.token.0);

        Ok(())
    }

    // fn notify(&self) {
    //     self.notify_reg
    //         .1
    //         .set_readiness(mio::Ready::readable())
    //         .unwrap();
    // }
}

lazy_static! {
    /// The state of the global networking driver.
    static ref REACTOR: Reactor = {
        // Spawn a thread that waits on the poller for new events and wakes up tasks blocked on I/O
        // handles.
        std::thread::Builder::new()
            .name("async-net-driver".to_string())
            .spawn(move || {
                // If the driver thread panics, there's not much we can do. It is not a
                // recoverable error and there is no place to propagate it into so we just abort.
                abort_on_panic(|| {
                    main_loop().expect("async networking thread has panicked");
                })
            })
            .expect("cannot start a thread driving blocking tasks");

        Reactor::new().expect("cannot initialize reactor")
    };
}

/// Waits on the poller for new events and wakes up tasks blocked on I/O handles.
fn main_loop() -> io::Result<()> {
    let reactor = &REACTOR;
    let mut events = mio::Events::with_capacity(1000);

    loop {
        // Block on the poller until at least one new event comes in.
        reactor.poller.poll(&mut events, None)?;

        // Lock the entire entry table while we're processing new events.
        let entries = reactor.entries.lock().unwrap();

        for event in events.iter() {
            let token = event.token();

            if token == reactor.notify_token {
                // If this is the notification token, we just need the notification state.
                reactor.notify_reg.1.set_readiness(mio::Ready::empty())?;
            } else {
                // Otherwise, look for the entry associated with this token.
                if let Some(entry) = entries.get(token.0) {
                    // Set the readiness flags from this I/O event.
                    let readiness = event.readiness();
                    entry
                        .readiness
                        .fetch_or(readiness.as_usize(), Ordering::SeqCst);

                    // Wake up reader tasks blocked on this I/O handle.
                    if !(readiness & reader_interests()).is_empty() {
                        for w in entry.readers.lock().unwrap().drain(..) {
                            w.wake();
                        }
                    }

                    // Wake up writer tasks blocked on this I/O handle.
                    if !(readiness & writer_interests()).is_empty() {
                        for w in entry.writers.lock().unwrap().drain(..) {
                            w.wake();
                        }
                    }
                }
            }
        }
    }
}

/// An I/O handle powered by the networking driver.
///
/// This handle wraps an I/O event source and exposes a "futurized" interface on top of it,
/// implementing traits `AsyncRead` and `AsyncWrite`.
pub struct IoHandle<T: Evented> {
    /// Data associated with the I/O handle.
    entry: Arc<Entry>,

    /// The I/O event source.
    source: T,
}

impl<T: Evented> IoHandle<T> {
    /// Creates a new I/O handle.
    ///
    /// The provided I/O event source will be kept registered inside the reactor's poller for the
    /// lifetime of the returned I/O handle.
    pub fn new(source: T) -> IoHandle<T> {
        IoHandle {
            entry: REACTOR
                .register(&source)
                .expect("cannot register an I/O event source"),
            source,
        }
    }

    /// Returns a reference to the inner I/O event source.
    pub fn get_ref(&self) -> &T {
        &self.source
    }

    /// Polls the I/O handle for reading.
    ///
    /// If reading from the I/O handle would block, `Poll::Pending` will be returned.
    pub fn poll_readable(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mask = reader_interests();
        let mut readiness = mio::Ready::from_usize(self.entry.readiness.load(Ordering::SeqCst));

        if (readiness & mask).is_empty() {
            self.entry.readers.lock().unwrap().push(cx.waker().clone());
            readiness = mio::Ready::from_usize(self.entry.readiness.fetch_or(0, Ordering::SeqCst));
        }

        if (readiness & mask).is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Clears the readability status.
    ///
    /// This method is usually called when an attempt at reading from the OS-level I/O handle
    /// returns `io::ErrorKind::WouldBlock`.
    pub fn clear_readable(&self, cx: &mut Context<'_>) -> io::Result<()> {
        let mask = reader_interests() - hup();
        self.entry
            .readiness
            .fetch_and(!mask.as_usize(), Ordering::SeqCst);

        if self.poll_readable(cx)?.is_ready() {
            // Wake the current task.
            cx.waker().wake_by_ref();
        }

        Ok(())
    }

    /// Polls the I/O handle for writing.
    ///
    /// If writing into the I/O handle would block, `Poll::Pending` will be returned.
    pub fn poll_writable(&self, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mask = writer_interests();
        let mut readiness = mio::Ready::from_usize(self.entry.readiness.load(Ordering::SeqCst));

        if (readiness & mask).is_empty() {
            self.entry.writers.lock().unwrap().push(cx.waker().clone());
            readiness = mio::Ready::from_usize(self.entry.readiness.fetch_or(0, Ordering::SeqCst));
        }

        if (readiness & mask).is_empty() {
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }

    /// Clears the writability status.
    ///
    /// This method is usually called when an attempt at writing from the OS-level I/O handle
    /// returns `io::ErrorKind::WouldBlock`.
    pub fn clear_writable(&self, cx: &mut Context<'_>) -> io::Result<()> {
        let mask = writer_interests() - hup();
        self.entry
            .readiness
            .fetch_and(!mask.as_usize(), Ordering::SeqCst);

        if self.poll_writable(cx)?.is_ready() {
            // Wake the current task.
            cx.waker().wake_by_ref();
        }

        Ok(())
    }
}

impl<T: Evented> Drop for IoHandle<T> {
    fn drop(&mut self) {
        REACTOR
            .deregister(&self.source, &self.entry)
            .expect("cannot deregister I/O event source");
    }
}

impl<T: Evented + fmt::Debug> fmt::Debug for IoHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IoHandle")
            .field("entry", &self.entry)
            .field("source", &self.source)
            .finish()
    }
}

impl<T: Evented + Unpin + Read> AsyncRead for IoHandle<T> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        futures::ready!(Pin::new(&mut *self).poll_readable(cx)?);

        match self.source.read(buf) {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_readable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }
}

impl<'a, T: Evented + Unpin> AsyncRead for &'a IoHandle<T>
where
    &'a T: Read,
{
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        futures::ready!(Pin::new(&mut *self).poll_readable(cx)?);

        match (&self.source).read(buf) {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_readable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }
}

impl<T: Evented + Unpin + Write> AsyncWrite for IoHandle<T> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        futures::ready!(self.poll_writable(cx)?);

        match self.source.write(buf) {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_writable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        futures::ready!(self.poll_writable(cx)?);

        match self.source.flush() {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_writable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl<'a, T: Evented + Unpin> AsyncWrite for &'a IoHandle<T>
where
    &'a T: Write,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        futures::ready!(self.poll_writable(cx)?);

        match (&self.source).write(buf) {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_writable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        futures::ready!(self.poll_writable(cx)?);

        match (&self.source).flush() {
            Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                self.clear_writable(cx)?;
                Poll::Pending
            }
            res => Poll::Ready(res),
        }
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

/// Returns a mask containing flags that interest tasks reading from I/O handles.
#[inline]
fn reader_interests() -> mio::Ready {
    mio::Ready::all() - mio::Ready::writable()
}

/// Returns a mask containing flags that interest tasks writing into I/O handles.
#[inline]
fn writer_interests() -> mio::Ready {
    mio::Ready::writable() | hup()
}

/// Returns a flag containing the hangup status.
#[inline]
fn hup() -> mio::Ready {
    #[cfg(unix)]
    let ready = mio::unix::UnixReady::hup().into();

    #[cfg(not(unix))]
    let ready = mio::Ready::empty();

    ready
}
