use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;

const THE_WINTERS_TALE: &[u8] = b"
    Each your doing,
    So singular in each particular,
    Crowns what you are doing in the present deed,
    That all your acts are queens.
";

#[test]
fn connect() -> io::Result<()> {
    thread::spawn_task(async {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let t = task::spawn(async move { listener.accept().await });

        let stream2 = TcpStream::connect(&addr).await?;
        let stream1 = t.await?.0;

        assert_eq!(stream1.peer_addr()?, stream2.local_addr()?);
        assert_eq!(stream2.peer_addr()?, stream1.local_addr()?);

        Ok(())
    })
}

#[test]
fn incoming_read() -> io::Result<()> {
    thread::spawn_task(async {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        task::spawn(async move {
            let mut stream = TcpStream::connect(&addr).await?;
            stream.write_all(THE_WINTERS_TALE).await?;
            io::Result::Ok(())
        });

        let mut buf = vec![0; 1024];
        let mut incoming = listener.incoming();
        let mut stream = incoming.next().await.unwrap()?;

        let n = stream.read(&mut buf).await?;
        assert_eq!(&buf[..n], THE_WINTERS_TALE);

        Ok(())
    })
}
