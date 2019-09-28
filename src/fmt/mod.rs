//! Utilities for formatting and printing `String`s.
//!
//! This library is an asynchronous version of [`std::fmt`].
//!
//! [`std::fmt`]: https://doc.rust-lang.org/std/fmt/index.html

// Structs re-export
pub use std::fmt::{
    Arguments, DebugList, DebugMap, DebugSet, DebugStruct, DebugTuple, Error, Formatter,
};

// Enums re-export
pub use std::fmt::Alignment;

// Traits re-export
pub use std::fmt::{
    Binary, Debug, Display, LowerExp, LowerHex, Octal, Pointer, UpperExp, UpperHex,
};

// Functions re-export
pub use std::fmt::format;

// Type defs re-export
pub use std::fmt::Result;
