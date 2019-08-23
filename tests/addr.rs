use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

use async_std::net::{SocketAddr, ToSocketAddrs};
use async_std::task;

fn tsa<A: ToSocketAddrs>(a: A) -> Result<Vec<SocketAddr>, String> {
    let socket_addrs = task::block_on(a.to_socket_addrs());
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
