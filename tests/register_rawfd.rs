#![cfg(all(unix, feature = "unstable"))]

use async_std::future::poll_fn;
use async_std::net::{TcpListener, TcpStream};
use async_std::os::unix::io::{register, unregister, FromRawFd, Interest, IntoRawFd};
use async_std::task::{self, block_on, Poll};

#[test]
fn register_stream() {
    block_on(async {
        for i in 0..3 {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            let t = task::spawn(async move { listener.accept().await });

            let stream = TcpStream::connect(&addr).await.unwrap().into_raw_fd();

            let mut woken = false;
            poll_fn(|cx| {
                if woken {
                    Poll::Ready(())
                } else {
                    match i {
                        0 => register(cx, stream, Interest::Read).unwrap(),
                        1 => register(cx, stream, Interest::Write).unwrap(),
                        2 => register(cx, stream, Interest::Both).unwrap(),
                        _ => unreachable!(),
                    }

                    unregister(stream).unwrap();
                    woken = true;

                    Poll::Pending
                }
            })
            .await;

            t.await.unwrap();
            unsafe {
                TcpStream::from_raw_fd(stream);
            }
        }
    });
}
