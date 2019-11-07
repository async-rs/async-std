//! Cross-platform path manipulation.
//!
//! This module is an async version of [`std::path`].
//!
//! This module provides two types, [`PathBuf`] and [`Path`][`Path`] (akin to [`String`]
//! and [`str`]), for working with paths abstractly. These types are thin wrappers
//! around [`OsString`] and [`OsStr`] respectively, meaning that they work directly
//! on strings according to the local platform's path syntax.
//!
//! Paths can be parsed into [`Component`]s by iterating over the structure
//! returned by the [`components`] method on [`Path`]. [`Component`]s roughly
//! correspond to the substrings between path separators (`/` or `\`). You can
//! reconstruct an equivalent path from components with the [`push`] method on
//! [`PathBuf`]; note that the paths may differ syntactically by the
//! normalization described in the documentation for the [`components`] method.
//!
//! [`std::path`]: https://doc.rust-lang.org/std/path/index.html
//!
//! ## Simple usage
//!
//! Path manipulation includes both parsing components from slices and building
//! new owned paths.
//!
//! To parse a path, you can create a [`Path`] slice from a [`str`]
//! slice and start asking questions:
//!
//! ```
//! use async_std::path::Path;
//! use std::ffi::OsStr;
//!
//! let path = Path::new("/tmp/foo/bar.txt");
//!
//! let parent = path.parent();
//! assert_eq!(parent, Some(Path::new("/tmp/foo")));
//!
//! let file_stem = path.file_stem();
//! assert_eq!(file_stem, Some(OsStr::new("bar")));
//!
//! let extension = path.extension();
//! assert_eq!(extension, Some(OsStr::new("txt")));
//! ```
//!
//! To build or modify paths, use [`PathBuf`]:
//!
//! ```
//! use async_std::path::PathBuf;
//!
//! // This way works...
//! let mut path = PathBuf::from("c:\\");
//!
//! path.push("windows");
//! path.push("system32");
//!
//! path.set_extension("dll");
//! ```
//!
//! [`Component`]: enum.Component.html
//! [`components`]: struct.Path.html#method.components
//! [`PathBuf`]: struct.PathBuf.html
//! [`Path`]: struct.Path.html
//! [`push`]: struct.PathBuf.html#method.push
//! [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
//!
//! [`str`]: https://doc.rust-lang.org/std/primitive.str.html
//! [`OsString`]: https://doc.rust-lang.org/std/ffi/struct.OsString.html
//! [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html

mod ancestors;
mod path;
mod pathbuf;

// Structs re-export
#[doc(inline)]
pub use std::path::{Components, Display, Iter, PrefixComponent, StripPrefixError};

// Enums re-export
#[doc(inline)]
pub use std::path::{Component, Prefix};

// Constants re-export
#[doc(inline)]
pub use std::path::MAIN_SEPARATOR;

// Functions re-export
#[doc(inline)]
pub use std::path::is_separator;

use ancestors::Ancestors;
pub use path::Path;
pub use pathbuf::PathBuf;
