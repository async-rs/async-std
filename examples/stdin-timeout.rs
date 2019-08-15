//! Reads a line from stdin, or exits with an error if nothing is read in 5 seconds.

#![feature(async_await)]

use std::time::Duration;

use async_std::io;
use async_std::prelude::*;
use async_std::task;

fn main() -> io::Result<()> {
    task::block_on(io::timeout(Duration::from_secs(5), async {
        let stdin = io::stdin();

        let mut line = String::new();
        stdin.read_line(&mut line).await?;

        print!("Got line: {}", line);
        Ok(())
    }))
}
