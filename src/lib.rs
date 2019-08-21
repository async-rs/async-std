//! Async version of the Rust standard library.
//!
//! This crate is an async version of [`std`].
//!
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
//! use async_std::task;
//!
//! fn main() {
//!     task::block_on(async {
//!         println!("Hello, world!");
//!     })
//! }
//! ```
//!
//! See [here](https://github.com/async-rs/async-std/tree/master/examples)
//! for more examples.

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

pub(crate) mod utils;
