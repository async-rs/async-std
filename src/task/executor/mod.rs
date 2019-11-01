//! Task executor.
//!
//! API bindings between `crate::task` and this module are very simple:
//!
//! * The only export is the `schedule` function.
//! * The only import is the `crate::task::Runnable` type.

pub(crate) use pool::schedule;

use sleepers::Sleepers;

mod pool;
mod sleepers;
