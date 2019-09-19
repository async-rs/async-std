//! Asynchronous iteration.
//!
//! This module is an async version of [`std::iter`].
//!
//! [`std::iter`]: https://doc.rust-lang.org/std/iter/index.html
//!
//! # Examples
//!
//! ```
//! # fn main() { async_std::task::block_on(async {
//! #
//! use async_std::prelude::*;
//! use async_std::stream;
//!
//! let mut s = stream::repeat(9).take(3);
//!
//! while let Some(v) = s.next().await {
//!     assert_eq!(v, 9);
//! }
//! #
//! # }) }
//! ```

use cfg_if::cfg_if;

pub use empty::{empty, Empty};
pub use once::{once, Once};
pub use repeat::{repeat, Repeat};
pub use stream::{Fuse, Scan, Stream, Take, Zip};

mod empty;
mod once;
mod repeat;
mod stream;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        mod double_ended_stream;
        mod from_stream;
        mod into_stream;

        pub use double_ended_stream::DoubleEndedStream;
        pub use from_stream::FromStream;
        pub use into_stream::IntoStream;
    }
}
