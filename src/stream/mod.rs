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
pub use stream::{Chain, Filter, Fuse, Inspect, Scan, Skip, SkipWhile, StepBy, Stream, Take, Zip};

pub(crate) mod stream;

mod empty;
mod once;
mod repeat;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        mod double_ended_stream;
        mod extend;
        mod from_stream;
        mod into_stream;

        pub use double_ended_stream::DoubleEndedStream;
        pub use extend::Extend;
        pub use from_stream::FromStream;
        pub use into_stream::IntoStream;

        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        #[doc(inline)]
        pub use async_macros::{join_stream as join, JoinStream as Join};
    }
}
