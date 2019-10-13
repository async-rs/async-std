use std::ffi::OsString;

use crate::path::Path;

/// This struct is an async version of [`std::path::PathBuf`].
///
/// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.PathBuf.html
#[derive(Debug, PartialEq)]
pub struct PathBuf {
    inner: std::path::PathBuf,
}

impl PathBuf {
    /// Coerces to a [`Path`] slice.
    ///
    /// [`Path`]: struct.Path.html
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let p = PathBuf::from("/test");
    /// assert_eq!(Path::new("/test"), p.as_path());
    /// ```
    pub fn as_path(&self) -> &Path {
        self.inner.as_path().into()
    }

    /// Converts this `PathBuf` into a [boxed][`Box`] [`Path`].
    ///
    /// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
    /// [`Path`]: struct.Path.html
    pub fn into_boxed_path(self) -> Box<Path> {
        let rw = Box::into_raw(self.inner.into_boxed_path()) as *mut Path;
        unsafe { Box::from_raw(rw) }
    }

    /// Consumes the `PathBuf`, yielding its internal [`OsString`] storage.
    ///
    /// [`OsString`]: https://doc.rust-lang.org/std/ffi/struct.OsString.html
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::PathBuf;
    ///
    /// let p = PathBuf::from("/the/head");
    /// let os_str = p.into_os_string();
    /// ```
    pub fn into_os_string(self) -> OsString {
        self.inner.into_os_string()
    }

    /// Allocates an empty `PathBuf`.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::PathBuf;
    ///
    /// let path = PathBuf::new();
    /// ```
    pub fn new() -> PathBuf {
        std::path::PathBuf::new().into()
    }

    /// Truncates `self` to [`self.parent`].
    ///
    /// Returns `false` and does nothing if [`self.parent`] is [`None`].
    /// Otherwise, returns `true`.
    ///
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    /// [`self.parent`]: struct.PathBuf.html#method.parent
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let mut p = PathBuf::from("/test/test.rs");
    ///
    /// p.pop();
    /// assert_eq!(Path::new("/test"), p.as_ref());
    /// p.pop();
    /// assert_eq!(Path::new("/"), p.as_ref());
    /// ```
    pub fn pop(&mut self) -> bool {
        self.inner.pop()
    }
}

impl From<std::path::PathBuf> for PathBuf {
    fn from(path: std::path::PathBuf) -> PathBuf {
        PathBuf { inner: path }
    }
}

impl Into<std::path::PathBuf> for PathBuf {
    fn into(self) -> std::path::PathBuf {
        self.inner.into()
    }
}

impl From<OsString> for PathBuf {
    fn from(path: OsString) -> PathBuf {
        std::path::PathBuf::from(path).into()
    }
}

impl From<&str> for PathBuf {
    fn from(path: &str) -> PathBuf {
        std::path::PathBuf::from(path).into()
    }
}

impl AsRef<Path> for PathBuf {
    fn as_ref(&self) -> &Path {
        Path::new(&self.inner)
    }
}

impl AsRef<std::path::Path> for PathBuf {
    fn as_ref(&self) -> &std::path::Path {
        self.inner.as_ref()
    }
}
