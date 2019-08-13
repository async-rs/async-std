//! Async version of the Rust standard library.
//!
//! This crate is an async version of [`std`].
//! Higher-level documentation in the form of the book
//! ["Async programming in Rust with async-std"][book]
//! is available.
//!
//! [`std`]: https://doc.rust-lang.org/std/index.html
//! [book]: https://book.async.rs
//!
//! # Examples
//!
//! Spawn a task and block the current thread on its result:
//!
//! ```
//! # #![feature(async_await)]
//! use async_std::task;
//!
//! fn main() {
//!     task::block_on(async {
//!         println!("Hello, world!");
//!     })
//! }
//! ```
//!
//! Use sockets in a familiar way, with low-friction built-in timeouts:
//!
//! ```no_run
//! #![feature(async_await)]
//!
//! use std::time::Duration;
//!
//! use async_std::{
//!     prelude::*,
//!     task,
//!     net::TcpStream,
//! };
//!
//! async fn get() -> std::io::Result<Vec<u8>> {
//!     let mut stream = TcpStream::connect("example.com:80").await?;
//!     stream.write_all(b"GET /index.html HTTP/1.0\r\n\r\n").await?;
//!
//!     let mut buf = vec![];
//!     stream.read_to_end(&mut buf)
//!         .timeout(Duration::from_secs(5))
//!         .await?;
//!
//!     Ok(buf)
//! }
//!
//! fn main() {
//!     task::block_on(async {
//!         let raw_response = get().await.expect("request");
//!         let response = String::from_utf8(raw_response)
//!             .expect("utf8 conversion");
//!         println!("received: {}", response);
//!     });
//! }
//! ```

#![feature(async_await)]
#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![doc(test(attr(deny(rust_2018_idioms, warnings))))]
#![doc(test(attr(allow(unused_extern_crates, unused_variables))))]
#![doc(html_logo_url = "https://async.rs/images/logo--hero.svg")]

pub mod fs;
pub mod future;
pub mod io;
pub mod net;
pub mod os;
pub mod prelude;
pub mod stream;
pub mod sync;
pub mod task;
pub mod time;

pub(crate) mod utils;
