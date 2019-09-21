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
pub use crate::io::BufRead;
#[doc(no_inline)]
pub use crate::io::Read;
#[doc(no_inline)]
pub use crate::io::Seek;
#[doc(no_inline)]
pub use crate::io::Write;

#[doc(hidden)]
pub use crate::io::BufReadExt as _;
#[doc(hidden)]
pub use crate::io::ReadExt as _;
#[doc(hidden)]
pub use crate::io::SeekExt as _;
#[doc(hidden)]
pub use crate::io::WriteExt as _;
