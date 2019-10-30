use std::sync::Arc;

use async_std::{
    io::{stdin, BufReader},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
    task,
    future::select,
};


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) fn main() -> Result<()> {
    task::block_on(try_main("127.0.0.1:8080"))
}

async fn try_main(addr: impl ToSocketAddrs) -> Result<()> {
    let stream = Arc::new(TcpStream::connect(addr).await?);
    let (reader, writer) = (stream.clone(), stream.clone());

    let incoming = task::spawn(async move {
         let mut messages = BufReader::new(&*reader).lines();
         while let Some(message) = messages.next().await {
             let message = message?;
             println!("{}", message);
         }
         Ok(())
    });

    let outgoing = task::spawn(async move {
         let mut stdin = BufReader::new(stdin()).lines();

         while let Some(line) = stdin.next().await {
             let line = line?;
             let message = format!("{}\n", line);
             (&*writer).write_all(message.as_bytes()).await?;
         }
         Ok(())
    });

    select!(incoming, outgoing).await
}
