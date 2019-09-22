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
pub use crate::io::buf_read::BufReadExt as _;
#[doc(hidden)]
pub use crate::io::read::ReadExt as _;
#[doc(hidden)]
pub use crate::io::seek::SeekExt as _;
#[doc(hidden)]
pub use crate::io::write::WriteExt as _;
