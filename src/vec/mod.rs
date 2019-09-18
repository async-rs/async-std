//! Core allocation and collections library.
//!
//! This library contains stream types for `std::vec::Vec`.

mod from_stream;
mod into_stream;

// #[doc(inline)]
// pub use std::vec::Vec;

pub use into_stream::{IntoStream, Stream, StreamMut};
