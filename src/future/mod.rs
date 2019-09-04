//! Asynchronous values.

#[doc(inline)]
pub use std::future::Future;

use cfg_if::cfg_if;

pub use pending::pending;
pub use poll_fn::poll_fn;
pub use ready::ready;

mod pending;
mod poll_fn;
mod ready;

cfg_if! {
    if #[cfg(any(feature = "unstable", feature = "docs"))] {
        mod timeout;
        pub use timeout::{timeout, TimeoutError};
    }
}
