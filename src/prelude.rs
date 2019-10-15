//! The async prelude.
//!
//! The prelude re-exports most commonly used traits and macros from this crate.
//!
//! # Examples
//!
//! Import the prelude with:
//!
//! ```
//! # #[allow(unused_imports)]
//! use async_std::prelude::*;
//! ```

use cfg_if::cfg_if;

#[doc(no_inline)]
pub use crate::future::Future;
#[doc(no_inline)]
pub use crate::io::BufRead as _;
#[doc(no_inline)]
pub use crate::io::Read as _;
#[doc(no_inline)]
pub use crate::io::Seek as _;
#[doc(no_inline)]
pub use crate::io::Write as _;
#[doc(no_inline)]
pub use crate::stream::Stream;
#[doc(no_inline)]
pub use crate::task_local;

#[doc(hidden)]
pub use crate::io::buf_read::BufReadExt as _;
#[doc(hidden)]
pub use crate::io::read::ReadExt as _;
#[doc(hidden)]
pub use crate::io::seek::SeekExt as _;
#[doc(hidden)]
pub use crate::io::write::WriteExt as _;
#[doc(hidden)]
pub use crate::stream::stream::StreamExt as _;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        #[doc(no_inline)]
        pub use crate::stream::DoubleEndedStream;

        #[doc(no_inline)]
        pub use crate::stream::ExactSizeStream;
    }
}
