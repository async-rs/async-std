//! Unix-specific filesystem extensions.

use std::path::Path;

use cfg_if::cfg_if;

use crate::io;
use crate::task::blocking;

/// Creates a new symbolic link on the filesystem.
///
/// The `dst` path will be a symbolic link pointing to the `src` path.
///
/// This function is an async version of [`std::os::unix::fs::symlink`].
///
/// [`std::os::unix::fs::symlink`]: https://doc.rust-lang.org/std/os/unix/fs/fn.symlink.html
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::os::unix::fs::symlink;
///
/// symlink("a.txt", "b.txt").await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> io::Result<()> {
    let src = src.as_ref().to_owned();
    let dst = dst.as_ref().to_owned();
    blocking::spawn(async move { std::os::unix::fs::symlink(&src, &dst) }).await
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// Unix-specific extensions to `DirBuilder`.
        pub trait DirBuilderExt {
            /// Sets the mode to create new directories with. This option defaults to
            /// `0o777`.
            fn mode(&mut self, mode: u32) -> &mut Self;
        }
    } else {
        #[doc(inline)]
        pub use std::os::unix::fs::DirBuilderExt;
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// Unix-specific extension methods for `DirEntry`.
        pub trait DirEntryExt {
            /// Returns the underlying `d_ino` field in the contained `dirent`
            /// structure.
            fn ino(&self) -> u64;
        }
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// Unix-specific extensions to `OpenOptions`.
        pub trait OpenOptionsExt {
            /// Sets the mode bits that a new file will be created with.
            ///
            /// If a new file is created as part of a `File::open_opts` call then this
            /// specified `mode` will be used as the permission bits for the new file.
            /// If no `mode` is set, the default of `0o666` will be used.
            /// The operating system masks out bits with the systems `umask`, to produce
            /// the final permissions.
            fn mode(&mut self, mode: u32) -> &mut Self;

            /// Pass custom flags to the `flags` argument of `open`.
            ///
            /// The bits that define the access mode are masked out with `O_ACCMODE`, to
            /// ensure they do not interfere with the access mode set by Rusts options.
            ///
            /// Custom flags can only set flags, not remove flags set by Rusts options.
            /// This options overwrites any previously set custom flags.
            fn custom_flags(&mut self, flags: i32) -> &mut Self;
        }
    } else {
        #[doc(inline)]
        pub use std::os::unix::fs::OpenOptionsExt;
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        /// Unix-specific extensions to [`fs::Permissions`].
        ///
        /// [`fs::Permissions`]: ../../../fs/struct.Permissions.html
        pub trait PermissionsExt {
            /// Returns the underlying raw `st_mode` bits that contain the standard
            /// Unix permissions for this file.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// use std::fs::File;
            /// use std::os::unix::fs::PermissionsExt;
            ///
            /// fn main() -> std::io::Result<()> {
            ///     let f = File::create("foo.txt")?;
            ///     let metadata = f.metadata()?;
            ///     let permissions = metadata.permissions();
            ///
            ///     println!("permissions: {:o}", permissions.mode());
            ///     Ok(()) }
            /// ```
            fn mode(&self) -> u32;

            /// Sets the underlying raw bits for this set of permissions.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// use std::fs::File;
            /// use std::os::unix::fs::PermissionsExt;
            ///
            /// fn main() -> std::io::Result<()> {
            ///     let f = File::create("foo.txt")?;
            ///     let metadata = f.metadata()?;
            ///     let mut permissions = metadata.permissions();
            ///
            ///     permissions.set_mode(0o644); // Read/write for owner and read for others.
            ///     assert_eq!(permissions.mode(), 0o644);
            ///     Ok(()) }
            /// ```
            fn set_mode(&mut self, mode: u32);

            /// Creates a new instance of `Permissions` from the given set of Unix
            /// permission bits.
            ///
            /// # Examples
            ///
            /// ```
            /// use std::fs::Permissions;
            /// use std::os::unix::fs::PermissionsExt;
            ///
            /// // Read/write for owner and read for others.
            /// let permissions = Permissions::from_mode(0o644);
            /// assert_eq!(permissions.mode(), 0o644);
            /// ```
            fn from_mode(mode: u32) -> Self;
        }
    }
}
