#![cfg(unix)]

use async_std::io;
use async_std::os::unix::net::{UnixDatagram, UnixListener, UnixStream};
use async_std::prelude::*;
use async_std::task;

use tempdir::TempDir;

use std::time::Duration;

const JULIUS_CAESAR: &[u8] = b"
    Friends, Romans, countrymen - lend me your ears!
    I come not to praise Caesar, but to bury him.
";

#[test]
fn send_recv() -> io::Result<()> {
    task::block_on(async {
        let (socket1, socket2) = UnixDatagram::pair().unwrap();
        socket1.send(JULIUS_CAESAR).await?;

        let mut buf = vec![0; 1024];
        let n = socket2.recv(&mut buf).await?;
        assert_eq!(&buf[..n], JULIUS_CAESAR);

        Ok(())
    })
}

#[test]
fn into_raw_fd() -> io::Result<()> {
    use async_std::os::unix::io::{FromRawFd, IntoRawFd};
    task::block_on(async {
        let (socket1, socket2) = UnixDatagram::pair().unwrap();
        socket1.send(JULIUS_CAESAR).await?;

        let mut buf = vec![0; 1024];

        let socket2 = unsafe { UnixDatagram::from_raw_fd(socket2.into_raw_fd()) };
        let n = socket2.recv(&mut buf).await?;
        assert_eq!(&buf[..n], JULIUS_CAESAR);

        Ok(())
    })
}

const PING: &[u8] = b"ping";
const PONG: &[u8] = b"pong";
const TEST_TIMEOUT: Duration = Duration::from_secs(3);

#[test]
fn socket_ping_pong() {
    let tmp_dir = TempDir::new("socket_ping_pong").expect("Temp dir not created");
    let sock_path = tmp_dir.as_ref().join("sock");
    let iter_cnt = 16;

    let listener =
        task::block_on(async { UnixListener::bind(&sock_path).await.expect("Socket bind") });

    let server_handle = std::thread::spawn(move || {
        task::block_on(async { ping_pong_server(listener, iter_cnt).await }).unwrap()
    });

    let client_handle = std::thread::spawn(move || {
        task::block_on(async { ping_pong_client(&sock_path, iter_cnt).await }).unwrap()
    });

    client_handle.join().unwrap();
    server_handle.join().unwrap();
}

async fn ping_pong_server(listener: UnixListener, iterations: u32) -> std::io::Result<()> {
    let mut incoming = listener.incoming();
    let mut buf = [0; 1024];
    for _ix in 0..iterations {
        if let Some(s) = incoming.next().await {
            let mut s = s?;
            let n = s.read(&mut buf[..]).await?;
            assert_eq!(&buf[..n], PING);
            s.write_all(&PONG).await?;
        }
    }
    Ok(())
}

async fn ping_pong_client(socket: &std::path::PathBuf, iterations: u32) -> std::io::Result<()> {
    let mut buf = [0; 1024];
    for _ix in 0..iterations {
        let mut socket = UnixStream::connect(&socket).await?;
        socket.write_all(&PING).await?;
        let n = async_std::io::timeout(TEST_TIMEOUT, socket.read(&mut buf[..])).await?;
        assert_eq!(&buf[..n], PONG);
    }
    Ok(())
}
