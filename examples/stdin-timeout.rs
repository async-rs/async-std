//! Reads a line from stdin, or exits with an error if nothing is read in 5 seconds.

#![feature(async_await)]

use std::time::Duration;

use async_std::{io, prelude::*, task};

fn main() -> io::Result<()> {
    task::block_on(async {
        let stdin = io::stdin();
        let mut line = String::new();

        match stdin
            .read_line(&mut line)
            .timeout(Duration::from_secs(5))
            .await
        {
            Ok(res) => {
                res?;
                print!("Got line: {}", line);
            }
            Err(_) => println!("You have only 5 seconds to enter a line. Try again :)"),
        }

        Ok(())
    })
}
