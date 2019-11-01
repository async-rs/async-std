//! Asynchronous values.
//!
//! ## Base Futures Concurrency
//!
//! Often it's desireable to await multiple futures as if it was a single
//! future. The `join` family of operations converts multiple futures into a
//! single future that returns all of their outputs. The `race` family of
//! operations converts multiple future into a single future that returns the
//! first output.
//!
//! For operating on futures the following macros can be used:
//!
//! | Name               | Return signature | When does it return?     |
//! | ---                | ---              | ---                      |
//! | [`future::join!`]  | `(T1, T2)`       | Wait for all to complete
//! | [`Future::race`]   | `T`              | Return on first value
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
//! In the case of `try_race`, instead of returning the first future that
//! completes it returns the first future that _successfully_ completes. This
//! means `try_race` will keep going until any one of the futures returns
//! `Ok`, or _all_ futures have returned `Err`.
//!
//! However sometimes it can be useful to use the base variants of the macros
//! even on futures that return `Result`. Here is an overview of operations that
//! work on `Result`, and their respective semantics:
//!
//! | Name                   | Return signature               | When does it return? |
//! | ---                    | ---                            | ---                  |
//! | [`future::join!`]      | `(Result<T, E>, Result<T, E>)` | Wait for all to complete
//! | [`future::try_join!`]  | `Result<(T1, T2), E>`          | Return on first `Err`, wait for all to complete
//! | [`Future::race`]       | `Result<T, E>`                 | Return on first value
//! | [`Future::try_race`]   | `Result<T, E>`                 | Return on first `Ok`, reject on last Err
//!
//! [`future::join!`]: macro.join.html
//! [`future::try_join!`]: macro.try_join.html
//! [`Future::race`]: trait.Future.html#method.race
//! [`Future::try_race`]: trait.Future.html#method.try_race

#[doc(inline)]
pub use async_macros::{join, try_join};

pub use future::Future;
pub use pending::pending;
pub use poll_fn::poll_fn;
pub use ready::ready;
pub use timeout::{timeout, TimeoutError};

pub(crate) mod future;
mod pending;
mod poll_fn;
mod ready;
mod timeout;

cfg_unstable! {
    pub use into_future::IntoFuture;
    mod into_future;
}
