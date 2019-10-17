//! Composable asynchronous iteration.
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

pub use empty::{empty, Empty};
pub use from_fn::{from_fn, FromFn};
pub use once::{once, Once};
pub use repeat::{repeat, Repeat};
pub use repeat_with::{repeat_with, RepeatWith};
pub use stream::{
    Chain, Filter, Fuse, Inspect, Scan, Skip, SkipWhile, StepBy, Stream, Take, TakeWhile, Zip,
};

pub(crate) mod stream;

mod empty;
mod from_fn;
mod once;
mod repeat;
mod repeat_with;

crate::unstable! {
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
    pub use stream::Merge;
    pub use sum::Sum;
}
