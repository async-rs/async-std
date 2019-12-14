//! Unix-specific I/O extensions.

#[cfg(feature = "unstable")]
use crate::task::Context;
#[cfg(feature = "unstable")]
use crate::io;

cfg_not_docs! {
    pub use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

    #[cfg(feature = "unstable")]
    use mio::unix::EventedFd;
    #[cfg(feature = "unstable")]
    use crate::net::driver::REACTOR;

    /// Registers an I/O handle so that the current task gets woken up when it becomes ready.
    #[cfg(feature = "unstable")]
    pub fn register(cx: &mut Context<'_>, fd: RawFd, interest: Interest) -> io::Result<()> {
        let evented = EventedFd(&fd);
        let entry = REACTOR.register(&evented, Some(fd))?;
        if interest == Interest::Read || interest == Interest::Both {
            let mut list = entry.readers.lock().unwrap();
            if list.iter().all(|w| !w.will_wake(cx.waker())) {
                list.push(cx.waker().clone());
            }
        }
        if interest == Interest::Write || interest == Interest::Both {
            let mut list = entry.writers.lock().unwrap();
            if list.iter().all(|w| !w.will_wake(cx.waker())) {
                list.push(cx.waker().clone());
            }
        }
        Ok(())
    }

    /// Unregisters an I/O handle.
    #[cfg(feature = "unstable")]
    pub fn unregister(fd: RawFd) -> io::Result<()> {
        let evented = EventedFd(&fd);
        REACTOR.deregister_fd(&evented, fd)
    }
}

cfg_docs! {
    /// Raw file descriptors.
    pub type RawFd = std::os::raw::c_int;

    /// A trait to extract the raw unix file descriptor from an underlying
    /// object.
    ///
    /// This is only available on unix platforms and must be imported in order
    /// to call the method. Windows platforms have a corresponding `AsRawHandle`
    /// and `AsRawSocket` set of traits.
    pub trait AsRawFd {
        /// Extracts the raw file descriptor.
        ///
        /// This method does **not** pass ownership of the raw file descriptor
        /// to the caller. The descriptor is only guaranteed to be valid while
        /// the original object has not yet been destroyed.
        fn as_raw_fd(&self) -> RawFd;
    }

    /// A trait to express the ability to construct an object from a raw file
    /// descriptor.
    pub trait FromRawFd {
        /// Constructs a new instance of `Self` from the given raw file
        /// descriptor.
        ///
        /// This function **consumes ownership** of the specified file
        /// descriptor. The returned object will take responsibility for closing
        /// it when the object goes out of scope.
        ///
        /// This function is also unsafe as the primitives currently returned
        /// have the contract that they are the sole owner of the file
        /// descriptor they are wrapping. Usage of this function could
        /// accidentally allow violating this contract which can cause memory
        /// unsafety in code that relies on it being true.
        unsafe fn from_raw_fd(fd: RawFd) -> Self;
    }

    /// A trait to express the ability to consume an object and acquire ownership of
    /// its raw file descriptor.
    pub trait IntoRawFd {
        /// Consumes this object, returning the raw underlying file descriptor.
        ///
        /// This function **transfers ownership** of the underlying file descriptor
        /// to the caller. Callers are then the unique owners of the file descriptor
        /// and must close the descriptor once it's no longer needed.
        fn into_raw_fd(self) -> RawFd;
    }

    /// Registers an I/O handle so that the current task gets woken up when it becomes ready.
    #[doc(cfg(unstable))]
    pub fn register(cx: &mut Context<'_>, fd: RawFd, interest: Interest) -> io::Result<()> {
        unreachable!()
    }

    /// Unregisters an I/O handle.
    #[doc(cfg(unstable))]
    pub fn unregister(fd: RawFd) -> io::Result<()> {
        unreachable!()
    }
}

cfg_unstable! {
    /// Decides which possibility a task is interested in.
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Interest {
        /// A task is interested in reading from a file descriptor.
        Read,
        /// A task is interested in writing to a file descriptor.
        Write,
        /// A task is interested in either being able to write or being able to read from a file
        /// descriptor.
        Both,
    }
}
