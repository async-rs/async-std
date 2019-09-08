//! The async I/O Prelude
//!
//! The purpose of this module is to alleviate imports of many common I/O traits
//! by adding a glob import to the top of I/O heavy modules:
//!
//! ```
//! # #![allow(unused_imports)]
//! use async_std::io::prelude::*;
//! ```

#[doc(no_inline)]
pub use super::BufRead as _;
#[doc(no_inline)]
pub use super::Read as _;
#[doc(no_inline)]
pub use super::Seek as _;
#[doc(no_inline)]
pub use super::Write as _;
