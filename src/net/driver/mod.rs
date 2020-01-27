use std::fmt;
use std::sync::{Arc, Mutex};

use mio::{self, Evented};
use once_cell::sync::Lazy;
use slab::Slab;

use crate::io;
use crate::task::{Context, Poll, Waker};
use crate::utils::abort_on_panic;

/// Data associated with a registered I/O handle.
#[derive(Debug)]
struct Entry {
    /// A unique identifier.
    token: mio::Token,

    /// Tasks that are blocked on reading from this I/O handle.
    readers: Mutex<Readers>,

    /// Thasks that are blocked on writing to this I/O handle.
    writers: Mutex<Writers>,
}

/// The set of `Waker`s interested in read readiness.
#[derive(Debug)]
struct Readers {
    /// Flag indicating read readiness.
    /// (cf. `Watcher::poll_read_ready`)
    ready: bool,
    /// The `Waker`s blocked on reading.
    wakers: Vec<Waker>
}

/// The set of `Waker`s interested in write readiness.
#[derive(Debug)]
struct Writers {
    /// Flag indicating write readiness.
    /// (cf. `Watcher::poll_write_ready`)
    ready: bool,
    /// The `Waker`s blocked on writing.
    wakers: Vec<Waker>
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
            readers: Mutex::new(Readers { ready: false, wakers: Vec::new() }),
            writers: Mutex::new(Writers { ready: false, wakers: Vec::new() }),
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

/// The state of the global networking driver.
static REACTOR: Lazy<Reactor> = Lazy::new(|| {
    // Spawn a thread that waits on the poller for new events and wakes up tasks blocked on I/O
    // handles.
    std::thread::Builder::new()
        .name("async-std/net".to_string())
        .spawn(move || {
            // If the driver thread panics, there's not much we can do. It is not a
            // recoverable error and there is no place to propagate it into so we just abort.
            abort_on_panic(|| {
                main_loop().expect("async networking thread has panicked");
            })
        })
        .expect("cannot start a thread driving blocking tasks");

    Reactor::new().expect("cannot initialize reactor")
});

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

                    // Wake up reader tasks blocked on this I/O handle.
                    if !(readiness & reader_interests()).is_empty() {
                        let mut readers = entry.readers.lock().unwrap();
                        readers.ready = true;
                        for w in readers.wakers.drain(..) {
                            w.wake();
                        }
                    }

                    // Wake up writer tasks blocked on this I/O handle.
                    if !(readiness & writer_interests()).is_empty() {
                        let mut writers = entry.writers.lock().unwrap();
                        writers.ready = true;
                        for w in writers.wakers.drain(..) {
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
pub struct Watcher<T: Evented> {
    /// Data associated with the I/O handle.
    entry: Arc<Entry>,

    /// The I/O event source.
    source: Option<T>,
}

impl<T: Evented> Watcher<T> {
    /// Creates a new I/O handle.
    ///
    /// The provided I/O event source will be kept registered inside the reactor's poller for the
    /// lifetime of the returned I/O handle.
    pub fn new(source: T) -> Watcher<T> {
        Watcher {
            entry: REACTOR
                .register(&source)
                .expect("cannot register an I/O event source"),
            source: Some(source),
        }
    }

    /// Returns a reference to the inner I/O event source.
    pub fn get_ref(&self) -> &T {
        self.source.as_ref().unwrap()
    }

    /// Polls the inner I/O source for a non-blocking read operation.
    ///
    /// If the operation returns an error of the `io::ErrorKind::WouldBlock` kind, the current task
    /// will be registered for wakeup when the I/O source becomes readable.
    pub fn poll_read_with<'a, F, R>(&'a self, cx: &mut Context<'_>, mut f: F) -> Poll<io::Result<R>>
    where
        F: FnMut(&'a T) -> io::Result<R>,
    {
        // If the operation isn't blocked, return its result.
        match f(self.source.as_ref().unwrap()) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            res => return Poll::Ready(res),
        }

        // Lock the waker list.
        let mut readers = self.entry.readers.lock().unwrap();

        // Try running the operation again.
        match f(self.source.as_ref().unwrap()) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            res => return Poll::Ready(res),
        }

        // Register the task if it isn't registered already.
        if readers.wakers.iter().all(|w| !w.will_wake(cx.waker())) {
            readers.wakers.push(cx.waker().clone());
        }

        readers.ready = false;

        Poll::Pending
    }

    /// Polls the inner I/O source for a non-blocking write operation.
    ///
    /// If the operation returns an error of the `io::ErrorKind::WouldBlock` kind, the current task
    /// will be registered for wakeup when the I/O source becomes writable.
    pub fn poll_write_with<'a, F, R>(
        &'a self,
        cx: &mut Context<'_>,
        mut f: F,
    ) -> Poll<io::Result<R>>
    where
        F: FnMut(&'a T) -> io::Result<R>,
    {
        // If the operation isn't blocked, return its result.
        match f(self.source.as_ref().unwrap()) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            res => return Poll::Ready(res),
        }

        // Lock the waker list.
        let mut writers = self.entry.writers.lock().unwrap();

        // Try running the operation again.
        match f(self.source.as_ref().unwrap()) {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            res => return Poll::Ready(res),
        }

        // Register the task if it isn't registered already.
        if writers.wakers.iter().all(|w| !w.will_wake(cx.waker())) {
            writers.wakers.push(cx.waker().clone());
        }

        writers.ready = false;

        Poll::Pending
    }

    /// Polls the inner I/O source until a non-blocking read can be performed.
    ///
    /// If non-blocking reads are currently not possible, the `Waker`
    /// will be saved and notified when it can read non-blocking
    /// again.
    #[allow(dead_code)]
    pub fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        // Lock the waker list.
        let mut readers = self.entry.readers.lock().unwrap();
        if readers.ready {
            return Poll::Ready(())
        }
        // Register the task if it isn't registered already.
        if readers.wakers.iter().all(|w| !w.will_wake(cx.waker())) {
            readers.wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }

    /// Polls the inner I/O source until a non-blocking write can be performed.
    ///
    /// If non-blocking writes are currently not possible, the `Waker`
    /// will be saved and notified when it can write non-blocking
    /// again.
    pub fn poll_write_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        // Lock the waker list.
        let mut writers = self.entry.writers.lock().unwrap();
        if writers.ready {
            return Poll::Ready(())
        }
        // Register the task if it isn't registered already.
        if writers.wakers.iter().all(|w| !w.will_wake(cx.waker())) {
            writers.wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }

    /// Deregisters and returns the inner I/O source.
    ///
    /// This method is typically used to convert `Watcher`s to raw file descriptors/handles.
    #[allow(dead_code)]
    pub fn into_inner(mut self) -> T {
        let source = self.source.take().unwrap();
        REACTOR
            .deregister(&source, &self.entry)
            .expect("cannot deregister I/O event source");
        source
    }
}

impl<T: Evented> Drop for Watcher<T> {
    fn drop(&mut self) {
        if let Some(ref source) = self.source {
            REACTOR
                .deregister(source, &self.entry)
                .expect("cannot deregister I/O event source");
        }
    }
}

impl<T: Evented + fmt::Debug> fmt::Debug for Watcher<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Watcher")
            .field("entry", &self.entry)
            .field("source", &self.source)
            .finish()
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
