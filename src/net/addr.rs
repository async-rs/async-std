use std::future;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::pin::Pin;

use crate::io;
use crate::task::{spawn_blocking};
use crate::utils::Context as ErrorContext;

/// Converts or resolves addresses to [`SocketAddr`] values.
///
/// This trait is an async version of [`std::net::ToSocketAddrs`].
///
/// [`std::net::ToSocketAddrs`]: https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html
/// [`SocketAddr`]: enum.SocketAddr.html
///
/// # Examples
///
/// ```
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::net::ToSocketAddrs;
///
/// let addr = "localhost:8080".to_socket_addrs().await?.next().unwrap();
/// println!("resolved: {:?}", addr);
/// #
/// # Ok(()) }) }
/// ```
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond to.
    type Iter: Iterator<Item = SocketAddr>;

    /// Converts this object to an iterator of resolved `SocketAddr`s.
    ///
    /// The returned iterator may not actually yield any values depending on the outcome of any
    /// resolution performed.
    ///
    /// Note that this function may block a backend thread while resolution is performed.
    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>>;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        Box::pin(future::ready(Ok(Some(*self).into_iter())))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        Box::pin(future::ready(Ok(Some(SocketAddr::V4(*self)).into_iter())))
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        Box::pin(future::ready(Ok(Some(SocketAddr::V6(*self)).into_iter())))
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        let (ip, port) = *self;
        match ip {
            IpAddr::V4(a) => Box::pin(future::ready(Ok(Some(SocketAddr::V4(SocketAddrV4::new(a, port))).into_iter()))),
            IpAddr::V6(a) => Box::pin(future::ready(Ok(Some(SocketAddr::V6(SocketAddrV6::new(a, port, 0, 0))).into_iter()))),
        }
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        let (ip, port) = *self;
        Box::pin(future::ready(Ok(Some(SocketAddr::V4(SocketAddrV4::new(ip, port))).into_iter())))
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        let (ip, port) = *self;
        Box::pin(future::ready(Ok(Some(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0))).into_iter())))
    }
}

impl ToSocketAddrs for (&str, u16) {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        let (host, port) = *self;

        if let Ok(addr) = host.parse::<Ipv4Addr>() {
            let addr = SocketAddrV4::new(addr, port);
            return Box::pin(future::ready(Ok(vec![SocketAddr::V4(addr)].into_iter())));
        }

        if let Ok(addr) = host.parse::<Ipv6Addr>() {
            let addr = SocketAddrV6::new(addr, port, 0, 0);
            return Box::pin(future::ready(Ok(vec![SocketAddr::V6(addr)].into_iter())));
        }

        let host = host.to_string();
        let task = spawn_blocking(move || {
            let addr = (host.as_str(), port);
            std::net::ToSocketAddrs::to_socket_addrs(&addr)
                .context(|| format!("could not resolve address `{:?}`", addr))
        });

        Box::pin(task)
    }
}

impl ToSocketAddrs for str {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        if let Ok(addr) = self.parse() {
            return Box::pin(future::ready(Ok(vec![addr].into_iter())));
        }

        let addr = self.to_string();
        let task = spawn_blocking(move || {
            std::net::ToSocketAddrs::to_socket_addrs(addr.as_str())
                .context(|| format!("could not resolve address `{:?}`", addr))
        });
        
        Box::pin(task)
    }
}

impl<'b> ToSocketAddrs for &'b [SocketAddr] {
    type Iter = std::iter::Cloned<std::slice::Iter<'b, SocketAddr>>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        Box::pin(future::ready(Ok(self.iter().cloned())))
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        (**self).to_socket_addrs()
    }
}

impl ToSocketAddrs for String {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = io::Result<Self::Iter>> + 'a>> {
        (&**self).to_socket_addrs()
    }
}
