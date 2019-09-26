//! Asynchronous values.
//!
//! ## Base Futures Concurrency
//!
//! Often it's desireable to await multiple futures as if it was a single
//! future. The `join` family of operations converts multiple futures into a
//! single future that returns all of their outputs. The `select` family of
//! operations converts multiple future into a single future that returns the
//! first output.
//!
//! For operating on futures the following macros can be used:
//!
//! | Name             | Return signature | When does it return?     |
//! | ---              | ---              | ---                      |
//! | `future::join`   | `(T1, T2)`       | Wait for all to complete
//! | `future::select` | `T`              | Return on first value
//!
//! ## Fallible Futures Concurrency
//!
//! For operating on futures that return `Result` additional `try_` variants of
//! the macros mentioned before can be used. These macros are aware of `Result`,
//! and will behave slightly differently from their base variants.
//!
//! In the case of `try_join`, if any of the futures returns `Err` all
//! futures are dropped and an error is returned. This is referred to as
//! "short-circuiting".
//!
//! In the case of `try_select`, instead of returning the first future that
//! completes it returns the first future that _successfully_ completes. This
//! means `try_select` will keep going until any one of the futures returns
//! `Ok`, or _all_ futures have returned `Err`.
//!
//! However sometimes it can be useful to use the base variants of the macros
//! even on futures that return `Result`. Here is an overview of operations that
//! work on `Result`, and their respective semantics:
//!
//! | Name                 | Return signature               | When does it return? |
//! | ---                  | ---                            | ---                  |
//! | `future::join`       | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
//! | `future::try_join`   | `Result<(T1, T2), E>`          | Return on first `Err`, wait for all to complete
//! | `future::select`     | `Result<T, E>`                 | Return on first value
//! | `future::try_select` | `Result<T, E>`                 | Return on first `Ok`, reject on last Err

#[doc(inline)]
pub use std::future::Future;

#[doc(inline)]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub use async_macros::{join, select, try_join, try_select};

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
