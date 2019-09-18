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
pub use super::BufRead;
#[doc(no_inline)]
pub use super::Read;
#[doc(no_inline)]
pub use super::Seek;
#[doc(no_inline)]
pub use super::Write;
