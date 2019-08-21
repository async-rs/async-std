//! Filesystem manipulation operations.
//!
//! This module is an async version of [`std::fs`].
//!
//! [`std::fs`]: https://doc.rust-lang.org/std/fs/index.html
//!
//! # Examples
//!
//! Create a new file and write some bytes to it:
//!
//! ```no_run
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::fs::File;
//! use async_std::prelude::*;
//!
//! let mut file = File::create("a.txt").await?;
//! file.write_all(b"Hello, world!").await?;
//! #
//! # Ok(()) }) }
//! ```

pub use dir_builder::DirBuilder;
pub use dir_entry::DirEntry;
pub use file::File;
pub use open_options::OpenOptions;
pub use read_dir::ReadDir;

#[doc(inline)]
pub use std::fs::{FileType, Metadata};

pub use canonicalize::canonicalize;
pub use copy::copy;
pub use create_dir::create_dir;
pub use create_dir_all::create_dir_all;
pub use hard_link::hard_link;
pub use metadata::metadata;
pub use read::read;
pub use read_dir::read_dir;
pub use read_link::read_link;
pub use read_to_string::read_to_string;
pub use remove_dir::remove_dir;
pub use remove_dir_all::remove_dir_all;
pub use remove_file::remove_file;
pub use rename::rename;
pub use set_permissions::set_permissions;
pub use symlink_metadata::symlink_metadata;
pub use write::write;

mod canonicalize;
mod copy;
mod create_dir;
mod create_dir_all;
mod dir_builder;
mod dir_entry;
mod file;
mod hard_link;
mod metadata;
mod open_options;
mod read;
mod read_dir;
mod read_link;
mod read_to_string;
mod remove_dir;
mod remove_dir_all;
mod remove_file;
mod rename;
mod set_permissions;
mod symlink_metadata;
mod write;

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// Representation of the various permissions on a file.
        ///
        /// This module only currently provides one bit of information, [`readonly`],
        /// which is exposed on all currently supported platforms. Unix-specific
        /// functionality, such as mode bits, is available through the
        /// [`PermissionsExt`] trait.
        ///
        /// [`readonly`]: struct.Permissions.html#method.readonly
        /// [`PermissionsExt`]: ../os/unix/fs/trait.PermissionsExt.html
        #[derive(Clone, PartialEq, Eq, Debug)]
        pub struct Permissions {
            _private: ()
        }
    } else {
        #[doc(inline)]
        pub use std::fs::Permissions;
    }
}
