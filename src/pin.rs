//! Types that pin data to its location in memory.
//!
//! For more documentation see [`std::pin`](https://doc.rust-lang.org/std/pin/index.html).

#[doc(inline)]
pub use std::pin::Pin;

#[doc(inline)]
pub use pin_project::pin_project;

#[doc(inline)]
pub use pin_project::pinned_drop;

#[doc(inline)]
pub use pin_project::project;

#[doc(inline)]
pub use pin_project::project_ref;

#[doc(inline)]
pub use pin_project::UnsafeUnpin;
