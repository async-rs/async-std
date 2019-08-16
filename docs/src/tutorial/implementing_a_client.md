## Implementing a client

Let's now implement the client for the chat.
Because the protocol is line-based, the implementation is pretty straightforward:

* Lines read from stdin should be send over the socket.
* Lines read from the socket should be echoed to stdout.

Unlike the server, the client needs only limited concurrency, as it interacts with only a single user.
For this reason, async doesn't bring a lot of performance benefits in this case.

However, async is still useful for managing concurrency!
Specifically, the client should *simultaneously* read from stdin and from the socket.
Programming this with threads is cumbersome, especially when implementing clean shutdown.
With async, we can just use the `select!` macro.

```rust
#![feature(async_await)]

use std::net::ToSocketAddrs;

use futures::select;
use futures::FutureExt;

use async_std::{
    prelude::*,
    net::TcpStream,
    task,
    io::{stdin, BufReader},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


fn main() -> Result<()> {
    task::block_on(try_main("127.0.0.1:8080"))
}

async fn try_main(addr: impl ToSocketAddrs) -> Result<()> {
    let stream = TcpStream::connect(addr).await?;
    let (reader, mut writer) = (&stream, &stream); // 1
    let reader = BufReader::new(reader);
    let mut lines_from_server = futures::StreamExt::fuse(reader.lines()); // 2

    let stdin = BufReader::new(stdin());
    let mut lines_from_stdin = futures::StreamExt::fuse(stdin.lines()); // 2
    loop {
        select! { // 3
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    println!("{}", line);
                },
                None => break,
            },
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    writer.write_all(line.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                }
                None => break,
            }
        }
    }
    Ok(())
}
```

1. Here we split `TcpStream` into read and write halfs: there's `impl AsyncRead for &'_ TcpStream`, just like the one in std.
2. We crate a steam of lines for both the socket and stdin.
3. In the main select loop, we print the lines we receive from server and send the lines we read from the console.
