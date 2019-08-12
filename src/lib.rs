//! Asynchronous standard library.
//!
//! This crate is an async version of [`std`].
//!
//! [`std`]: https://doc.rust-lang.org/std/index.html
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

#![feature(async_await)]
#![cfg_attr(feature = "docs.rs", feature(doc_cfg))]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
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
