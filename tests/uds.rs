#![cfg(unix)]

use async_std::io;
use async_std::os::unix::net::UnixDatagram;
use async_std::task;

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
