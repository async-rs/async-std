use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::pin::Pin;

use cfg_if::cfg_if;
use futures::future::{ready, Ready};

use crate::future::Future;
use crate::io;
use crate::task::blocking;
use crate::task::{Context, Poll};
use std::marker::PhantomData;

cfg_if! {
    if #[cfg(feature = "docs")] {
        #[doc(hidden)]
        pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);

        macro_rules! ret {
            ($a:lifetime, $f:tt, $i:ty) => (ImplFuture<$a, io::Result<$i>>);
        }
    } else {
        macro_rules! ret {
            ($a:lifetime, $f:tt, $i:ty) => ($f<$a, $i>);
        }
    }
}

/// A trait for objects which can be converted or resolved to one or more [`SocketAddr`] values.
///
/// This trait is an async version of [`std::net::ToSocketAddrs`].
///
/// [`std::net::ToSocketAddrs`]: https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html
/// [`SocketAddr`]: https://doc.rust-lang.org/std/net/enum.SocketAddr.html
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond to.
    type Iter: Iterator<Item = SocketAddr>;

    /// Converts this object to an iterator of resolved `SocketAddr`s.
    ///
    /// The returned iterator may not actually yield any values depending on the outcome of any
    /// resolution performed.
    ///
    /// Note that this function may block a backend thread while resolution is performed.
    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter);
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub enum ToSocketAddrsFuture<'a, I: Iterator<Item = SocketAddr>> {
    Phantom(PhantomData<&'a ()>),
    Join(blocking::JoinHandle<io::Result<I>>),
    Ready(Ready<io::Result<I>>),
}

impl<I: Iterator<Item = SocketAddr>> Future for ToSocketAddrsFuture<'_, I> {
    type Output = io::Result<I>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.get_mut() {
            ToSocketAddrsFuture::Join(f) => Pin::new(&mut *f).poll(cx),
            ToSocketAddrsFuture::Ready(f) => Pin::new(&mut *f).poll(cx),
            _ => unreachable!(),
        }
    }
}

impl ToSocketAddrs for SocketAddr {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl ToSocketAddrs for (&str, u16) {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        let host = self.0.to_string();
        let port = self.1;
        let join = blocking::spawn(async move {
            std::net::ToSocketAddrs::to_socket_addrs(&(host.as_str(), port))
        });
        ToSocketAddrsFuture::Join(join)
    }
}

impl ToSocketAddrs for str {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        let socket_addrs = self.to_string();
        let join =
            blocking::spawn(async move { std::net::ToSocketAddrs::to_socket_addrs(&socket_addrs) });
        ToSocketAddrsFuture::Join(join)
    }
}

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = std::iter::Cloned<std::slice::Iter<'a, SocketAddr>>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrsFuture::Ready(ready(std::net::ToSocketAddrs::to_socket_addrs(self)))
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        (**self).to_socket_addrs()
    }
}

impl ToSocketAddrs for String {
    type Iter = std::vec::IntoIter<SocketAddr>;

    fn to_socket_addrs(&self) -> ret!('_, ToSocketAddrsFuture, Self::Iter) {
        ToSocketAddrs::to_socket_addrs(self.as_str())
    }
}
