use std::fs;
use std::path::Path;

use cfg_if::cfg_if;

use crate::future::Future;
use crate::io;
use crate::task::blocking;

/// A builder for creating directories in various manners.
///
/// This type is an async version of [`std::fs::DirBuilder`].
///
/// [`std::fs::DirBuilder`]: https://doc.rust-lang.org/std/fs/struct.DirBuilder.html
#[derive(Debug)]
pub struct DirBuilder {
    recursive: bool,

    #[cfg(unix)]
    mode: Option<u32>,
}

impl DirBuilder {
    /// Creates a new builder with [`recursive`] set to `false`.
    ///
    /// [`recursive`]: #method.recursive
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::fs::DirBuilder;
    ///
    /// let builder = DirBuilder::new();
    /// ```
    pub fn new() -> DirBuilder {
        #[cfg(unix)]
        let builder = DirBuilder {
            recursive: false,
            mode: None,
        };

        #[cfg(windows)]
        let builder = DirBuilder { recursive: false };

        builder
    }

    /// Sets the option for recursive mode.
    ///
    /// This option, when `true`, means that all parent directories should be created recursively
    /// if they don't exist. Parents are created with the same security settings and permissions as
    /// the final directory.
    ///
    /// This option defaults to `false`.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::fs::DirBuilder;
    ///
    /// let mut builder = DirBuilder::new();
    /// builder.recursive(true);
    /// ```
    pub fn recursive(&mut self, recursive: bool) -> &mut Self {
        self.recursive = recursive;
        self
    }

    /// Creates a directory with the configured options.
    ///
    /// It is considered an error if the directory already exists unless recursive mode is enabled.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::DirBuilder;
    ///
    /// DirBuilder::new()
    ///     .recursive(true)
    ///     .create("/tmp/foo/bar/baz")
    ///     .await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn create<P: AsRef<Path>>(&self, path: P) -> impl Future<Output = io::Result<()>> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(self.recursive);

        #[cfg(unix)]
        {
            if let Some(mode) = self.mode {
                std::os::unix::fs::DirBuilderExt::mode(&mut builder, mode);
            }
        }

        let path = path.as_ref().to_owned();
        async move { blocking::spawn(async move { builder.create(path) }).await }
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        use crate::os::unix::fs::DirBuilderExt;
    } else if #[cfg(unix)] {
        use std::os::unix::fs::DirBuilderExt;
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs"))] {
        impl DirBuilderExt for DirBuilder {
            fn mode(&mut self, mode: u32) -> &mut Self {
                self.mode = Some(mode);
                self
            }
        }
    }
}
