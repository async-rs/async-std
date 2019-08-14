//! Asynchronous values.

#[doc(inline)]
pub use std::future::Future;

pub use pending::pending;
pub use ready::ready;
pub use timeout::{timeout, TimeoutError};

mod pending;
mod ready;
mod timeout;
