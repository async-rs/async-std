//! Counts the number of lines in a file given as an argument.

use std::env::args;

use async_std::fs::File;
use async_std::io::{self, BufReader};
use async_std::prelude::*;
use async_std::thread;

fn main() -> io::Result<()> {
    let path = args().nth(1).expect("missing path argument");

    thread::spawn_task(async {
        let file = File::open(&path).await?;
        let mut lines = BufReader::new(file).lines();
        let mut count = 0u64;

        while let Some(line) = lines.next().await {
            line?;
            count += 1;
        }

        println!("The file contains {} lines.", count);
        Ok(())
    })
}
