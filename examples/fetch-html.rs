//! Fetches the HTML contents of the Rust website.

#![feature(async_await)]

use std::error::Error;

use async_std::task;

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    task::block_on(async {
        // let contents = surf::get("https://www.rust-lang.org").recv_string().await?;
        // println!("{}", contents);
        Ok(())
    })
}
