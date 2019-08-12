//! Asynchronous values.

#[doc(inline)]
pub use std::future::Future;

pub use pending::pending;
pub use ready::ready;

mod pending;
mod ready;
