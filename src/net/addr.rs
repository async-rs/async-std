use crate::future::Future;
use crate::task::blocking::JoinHandle;
use futures::future::{ready, Ready};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
pub use std::net::{SocketAddr, SocketAddrV4, SocketAddrV6};

/// A trait for objects which can be converted or resolved to one or more
/// [`SocketAddr`] values.
///
/// This trait is an async version of [`std::net::ToSocketAddrs`].
///
/// [`std::net::ToSocketAddrs`]: https://doc.rust-lang.org/std/net/trait.ToSocketAddrs.html
pub trait ToSocketAddrs {
    /// Returned iterator over socket addresses which this type may correspond
    /// to.
    type Iter: Iterator<Item = SocketAddr> + Send;
    /// Returned Future
    type Output: Future<Output = crate::io::Result<Self::Iter>> + Send;
    /// Converts this object to an iterator of resolved `SocketAddr`s.
    ///
    /// The returned iterator may not actually yield any values depending on the
    /// outcome of any resolution performed.
    ///
    /// Note that this function may block a backend thread while resolution is
    /// performed.
    fn to_socket_addrs(&self) -> Self::Output;
}

impl ToSocketAddrs for SocketAddr {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for SocketAddrV4 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for SocketAddrV6 {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for (IpAddr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for (Ipv4Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for (Ipv6Addr, u16) {
    type Iter = std::option::IntoIter<SocketAddr>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl ToSocketAddrs for (&str, u16) {
    type Iter = std::vec::IntoIter<SocketAddr>;
    type Output = JoinHandle<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        let host = self.0.to_string();
        let port = self.1;
        crate::task::blocking::spawn(async move {
            std::net::ToSocketAddrs::to_socket_addrs(&(host.as_str(), port))
        })
    }
}

impl ToSocketAddrs for str {
    type Iter = std::vec::IntoIter<SocketAddr>;
    type Output = JoinHandle<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        let host = self.to_string();
        crate::task::blocking::spawn(async move { std::net::ToSocketAddrs::to_socket_addrs(&host) })
    }
}

impl<'a> ToSocketAddrs for &'a [SocketAddr] {
    type Iter = std::iter::Cloned<std::slice::Iter<'a, SocketAddr>>;
    type Output = Ready<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ready(std::net::ToSocketAddrs::to_socket_addrs(self))
    }
}

impl<T: ToSocketAddrs + ?Sized> ToSocketAddrs for &T {
    type Iter = T::Iter;
    type Output = T::Output;

    fn to_socket_addrs(&self) -> Self::Output {
        (**self).to_socket_addrs()
    }
}

impl ToSocketAddrs for String {
    type Iter = std::vec::IntoIter<SocketAddr>;
    type Output = JoinHandle<crate::io::Result<Self::Iter>>;

    fn to_socket_addrs(&self) -> Self::Output {
        ToSocketAddrs::to_socket_addrs(self.as_str())
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use crate::net::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn tsa<A: ToSocketAddrs>(a: A) -> Result<Vec<SocketAddr>, String> {
        let socket_addrs = crate::task::block_on(a.to_socket_addrs());
        match socket_addrs {
            Ok(a) => Ok(a.collect()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn sa4(a: Ipv4Addr, p: u16) -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(a, p))
    }

    pub fn sa6(a: Ipv6Addr, p: u16) -> SocketAddr {
        SocketAddr::V6(SocketAddrV6::new(a, p, 0, 0))
    }

    #[test]
    fn to_socket_addr_ipaddr_u16() {
        let a = Ipv4Addr::new(77, 88, 21, 11);
        let p = 12345;
        let e = SocketAddr::V4(SocketAddrV4::new(a, p));
        assert_eq!(Ok(vec![e]), tsa((a, p)));
    }

    #[test]
    fn to_socket_addr_str_u16() {
        let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
        assert_eq!(Ok(vec![a]), tsa(("77.88.21.11", 24352)));

        let a = sa6(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 53);
        assert_eq!(Ok(vec![a]), tsa(("2a02:6b8:0:1::1", 53)));

        let a = sa4(Ipv4Addr::new(127, 0, 0, 1), 23924);
        #[cfg(not(target_env = "sgx"))]
        assert!(tsa(("localhost", 23924)).unwrap().contains(&a));
        #[cfg(target_env = "sgx")]
        let _ = a;
    }

    #[test]
    fn to_socket_addr_str() {
        let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
        assert_eq!(Ok(vec![a]), tsa("77.88.21.11:24352"));

        let a = sa6(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 53);
        assert_eq!(Ok(vec![a]), tsa("[2a02:6b8:0:1::1]:53"));

        let a = sa4(Ipv4Addr::new(127, 0, 0, 1), 23924);
        #[cfg(not(target_env = "sgx"))]
        assert!(tsa("localhost:23924").unwrap().contains(&a));
        #[cfg(target_env = "sgx")]
        let _ = a;
    }

    #[test]
    fn to_socket_addr_string() {
        let a = sa4(Ipv4Addr::new(77, 88, 21, 11), 24352);
        assert_eq!(Ok(vec![a]), tsa(&*format!("{}:{}", "77.88.21.11", "24352")));
        assert_eq!(Ok(vec![a]), tsa(&format!("{}:{}", "77.88.21.11", "24352")));
        assert_eq!(Ok(vec![a]), tsa(format!("{}:{}", "77.88.21.11", "24352")));

        let s = format!("{}:{}", "77.88.21.11", "24352");
        assert_eq!(Ok(vec![a]), tsa(s));
        // s has been moved into the tsa call
    }

    // FIXME: figure out why this fails on openbsd and fix it
    #[test]
    #[cfg(not(any(windows, target_os = "openbsd")))]
    fn to_socket_addr_str_bad() {
        assert!(tsa("1200::AB00:1234::2552:7777:1313:34300").is_err());
    }

    #[test]
    fn set_ip() {
        fn ip4(low: u8) -> Ipv4Addr {
            Ipv4Addr::new(77, 88, 21, low)
        }
        fn ip6(low: u16) -> Ipv6Addr {
            Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, low)
        }

        let mut v4 = SocketAddrV4::new(ip4(11), 80);
        assert_eq!(v4.ip(), &ip4(11));
        v4.set_ip(ip4(12));
        assert_eq!(v4.ip(), &ip4(12));

        let mut addr = SocketAddr::V4(v4);
        assert_eq!(addr.ip(), IpAddr::V4(ip4(12)));
        addr.set_ip(IpAddr::V4(ip4(13)));
        assert_eq!(addr.ip(), IpAddr::V4(ip4(13)));
        addr.set_ip(IpAddr::V6(ip6(14)));
        assert_eq!(addr.ip(), IpAddr::V6(ip6(14)));

        let mut v6 = SocketAddrV6::new(ip6(1), 80, 0, 0);
        assert_eq!(v6.ip(), &ip6(1));
        v6.set_ip(ip6(2));
        assert_eq!(v6.ip(), &ip6(2));

        let mut addr = SocketAddr::V6(v6);
        assert_eq!(addr.ip(), IpAddr::V6(ip6(2)));
        addr.set_ip(IpAddr::V6(ip6(3)));
        assert_eq!(addr.ip(), IpAddr::V6(ip6(3)));
        addr.set_ip(IpAddr::V4(ip4(4)));
        assert_eq!(addr.ip(), IpAddr::V4(ip4(4)));
    }

    #[test]
    fn set_port() {
        let mut v4 = SocketAddrV4::new(Ipv4Addr::new(77, 88, 21, 11), 80);
        assert_eq!(v4.port(), 80);
        v4.set_port(443);
        assert_eq!(v4.port(), 443);

        let mut addr = SocketAddr::V4(v4);
        assert_eq!(addr.port(), 443);
        addr.set_port(8080);
        assert_eq!(addr.port(), 8080);

        let mut v6 = SocketAddrV6::new(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 80, 0, 0);
        assert_eq!(v6.port(), 80);
        v6.set_port(443);
        assert_eq!(v6.port(), 443);

        let mut addr = SocketAddr::V6(v6);
        assert_eq!(addr.port(), 443);
        addr.set_port(8080);
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn set_flowinfo() {
        let mut v6 = SocketAddrV6::new(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 80, 10, 0);
        assert_eq!(v6.flowinfo(), 10);
        v6.set_flowinfo(20);
        assert_eq!(v6.flowinfo(), 20);
    }

    #[test]
    fn set_scope_id() {
        let mut v6 = SocketAddrV6::new(Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1), 80, 0, 10);
        assert_eq!(v6.scope_id(), 10);
        v6.set_scope_id(20);
        assert_eq!(v6.scope_id(), 20);
    }

    #[test]
    fn is_v4() {
        let v4 = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(77, 88, 21, 11), 80));
        assert!(v4.is_ipv4());
        assert!(!v4.is_ipv6());
    }

    #[test]
    fn is_v6() {
        let v6 = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0x2a02, 0x6b8, 0, 1, 0, 0, 0, 1),
            80,
            10,
            0,
        ));
        assert!(!v6.is_ipv4());
        assert!(v6.is_ipv6());
    }
}
