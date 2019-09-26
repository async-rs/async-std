//! Unix-specific networking extensions.

use cfg_if::cfg_if;

pub use datagram::UnixDatagram;
pub use listener::{Incoming, UnixListener};
pub use stream::UnixStream;

mod datagram;
mod listener;
mod stream;

cfg_if! {
    if #[cfg(feature = "docs")] {
        use std::fmt;
        use std::path::Path;

        /// An address associated with a Unix socket.
        ///
        /// # Examples
        ///
        /// ```
        /// use async_std::os::unix::net::UnixListener;
        ///
        /// let socket = UnixListener::bind("/tmp/socket").await?;
        /// let addr = socket.local_addr()?;
        /// ```
        #[derive(Clone)]
        pub struct SocketAddr {
            _private: (),
        }

        impl SocketAddr {
            /// Returns `true` if the address is unnamed.
            ///
            /// # Examples
            ///
            /// A named address:
            ///
            /// ```no_run
            /// use async_std::os::unix::net::UnixListener;
            ///
            /// let socket = UnixListener::bind("/tmp/socket").await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.is_unnamed(), false);
            /// ```
            ///
            /// An unnamed address:
            ///
            /// ```no_run
            /// use async_std::os::unix::net::UnixDatagram;
            ///
            /// let socket = UnixDatagram::unbound().await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.is_unnamed(), true);
            /// ```
            pub fn is_unnamed(&self) -> bool {
                unreachable!("this impl only appears in the rendered docs")
            }

            /// Returns the contents of this address if it is a `pathname` address.
            ///
            /// # Examples
            ///
            /// With a pathname:
            ///
            /// ```no_run
            /// use std::path::Path;
            ///
            /// use async_std::os::unix::net::UnixListener;
            ///
            /// let socket = UnixListener::bind("/tmp/socket").await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.as_pathname(), Some(Path::new("/tmp/socket")));
            /// ```
            ///
            /// Without a pathname:
            ///
            /// ```
            /// use async_std::os::unix::net::UnixDatagram;
            ///
            /// let socket = UnixDatagram::unbound()?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.as_pathname(), None);
            /// ```
            pub fn as_pathname(&self) -> Option<&Path> {
                unreachable!("this impl only appears in the rendered docs")
            }
        }

        impl fmt::Debug for SocketAddr {
            fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                unreachable!("this impl only appears in the rendered docs")
            }
        }
    } else {
        pub use std::os::unix::net::SocketAddr;
    }
}
