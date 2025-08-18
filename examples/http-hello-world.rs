//! HTTP Hello World server
//!
//! To make an HTTP request do:
//!
//! ```sh
//! curl http://localhost:8080/foo
//! ```

use async_std::io;
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use async_std::task;

async fn process(stream: &mut TcpStream) -> io::Result<()> {
    let msg = "HTTP/1.1 200 OK
Content-Length: 11
Content-Type: text/plain

hello world";
    let mut buffer = [0; 512];
    stream.read(&mut buffer).await?;
    stream.write_all(msg.as_bytes()).await?;

    Ok(())
}

fn main() -> io::Result<()> {
    task::block_on(async {
        let listener = TcpListener::bind("127.0.0.1:8080").await?;
        println!("Listening on {}", listener.local_addr()?);

        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let mut stream = stream?;
            task::spawn(async move {
                process(&mut stream).await.unwrap();
            });
        }
        Ok(())
    })
}
