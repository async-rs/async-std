//! The async prelude.
//!
//! The prelude re-exports the most commonly used traits in this crate.

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
