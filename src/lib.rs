//! Async version of the Rust standard library.
//!
//! Modules in this crate are organized in the same way as in the standard library, except blocking
//! functions have been replaced with async functions and threads have been replaced with
//! lightweight tasks.
//!
//! More information, reading materials, and other resources:
//!
//! * [🌐 The async-std website](https://async.rs/)
//! * [📖 The async-std book](https://book.async.rs)
//! * [🐙 GitHub repository](https://github.com/async-rs/async-std)
//! * [📒 List of code examples](https://github.com/async-rs/async-std/tree/master/examples)
//! * [💬 Discord chat](https://discord.gg/JvZeVNe)
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
//! # Features
//!
//! Items marked with
//! <span
//!   class="module-item stab portability"
//!   style="display: inline; border-radius: 3px; padding: 2px; font-size: 80%; line-height: 1.2;"
//! ><code>unstable</code></span>
//! are available only when the `unstable` Cargo feature is enabled:
//!
//! ```toml
//! [dependencies.async-std]
//! version = "0.99"
//! features = ["unstable"]
//! ```

#![cfg_attr(feature = "docs", feature(doc_cfg))]
#![warn(missing_docs, missing_debug_implementations, rust_2018_idioms)]
#![doc(test(attr(deny(rust_2018_idioms, warnings))))]
#![doc(test(attr(allow(unused_extern_crates, unused_variables))))]
#![doc(html_logo_url = "https://async.rs/images/logo--hero.svg")]
#![recursion_limit = "1024"]

use cfg_if::cfg_if;

pub mod fs;
pub mod future;
pub mod io;
pub mod net;
pub mod os;
pub mod prelude;
pub mod stream;
pub mod sync;
pub mod task;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        pub mod pin;

        mod vec;
        mod result;
        mod option;
    }
}

pub(crate) mod utils;
