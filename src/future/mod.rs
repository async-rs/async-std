//! Asynchronous values.

#[doc(inline)]
pub use std::future::Future;

pub use pending::pending;
pub use poll_fn::poll_fn;
pub use ready::ready;

mod pending;
mod poll_fn;
mod ready;
