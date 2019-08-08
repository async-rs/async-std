//! Prints a file given as an argument to stdout.

#![feature(async_await)]

use std::env::args;

use async_std::{fs, io, prelude::*, task};

const LEN: usize = 4 * 1024 * 1024; // 4 Mb

fn main() -> io::Result<()> {
    let path = args().nth(1).expect("missing path argument");

    task::block_on(async {
        let mut file = fs::File::open(&path).await?;
        let mut stdout = io::stdout();
        let mut buf = vec![0u8; LEN];

        loop {
            // Read a buffer from the file.
            let n = file.read(&mut buf).await?;

            // If this is the end of file, clean up and return.
            if n == 0 {
                stdout.flush().await?;
                file.close().await?;
                return Ok(());
            }

            // Write the buffer into stdout.
            stdout.write_all(&buf[..n]).await?;
        }
    })
}
