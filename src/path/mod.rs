//! Cross-platform path manipulation.
//!
//! This module is an async version of [`std::path`].
//!
//! [`std::path`]: https://doc.rust-lang.org/std/path/index.html

mod path;
mod pathbuf;

// Structs re-export
#[doc(inline)]
pub use std::path::{Ancestors, Components, Display, Iter, PrefixComponent, StripPrefixError};

// Enums re-export
#[doc(inline)]
pub use std::path::{Component, Prefix};

// Constants re-export
#[doc(inline)]
pub use std::path::MAIN_SEPARATOR;

// Functions re-export
#[doc(inline)]
pub use std::path::is_separator;

pub use path::Path;
pub use pathbuf::PathBuf;
