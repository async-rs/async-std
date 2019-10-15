//! Asynchronous iteration.
//!
//! This module is an async version of [`std::iter`].
//!
//! [`std::iter`]: https://doc.rust-lang.org/std/iter/index.html
//!
//! # Examples
//!
//! ```
//! # async_std::task::block_on(async {
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
//! # })
//! ```

use cfg_if::cfg_if;

pub use empty::{empty, Empty};
pub use from_fn::{from_fn, FromFn};
pub use once::{once, Once};
pub use repeat::{repeat, Repeat};
pub use stream::{
    Chain, Filter, Fuse, Inspect, Scan, Skip, SkipWhile, StepBy, Stream, Take, TakeWhile, Zip,
};

pub(crate) mod stream;

mod empty;
mod from_fn;
mod once;
mod repeat;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        mod double_ended_stream;
        mod exact_size_stream;
        mod extend;
        mod from_stream;
        mod fused_stream;
        mod interval;
        mod into_stream;
        mod product;
        mod sum;

        pub use double_ended_stream::DoubleEndedStream;
        pub use exact_size_stream::ExactSizeStream;
        pub use extend::Extend;
        pub use from_stream::FromStream;
        pub use fused_stream::FusedStream;
        pub use interval::{interval, Interval};
        pub use into_stream::IntoStream;
        pub use product::Product;
        pub use sum::Sum;

        pub use stream::Merge;
    }
}
