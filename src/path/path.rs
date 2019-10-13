use std::ffi::OsStr;

use crate::path::{Components, PathBuf};
use crate::{fs, io};

/// This struct is an async version of [`std::path::Path`].
///
/// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.Path.html
#[derive(Debug, PartialEq)]
pub struct Path {
    inner: std::path::Path,
}

impl Path {
    /// Yields the underlying [`OsStr`] slice.
    ///
    /// [`OsStr`]: https://doc.rust-lang.org/std/ffi/struct.OsStr.html
    pub fn as_os_str(&self) -> &OsStr {
        self.inner.as_os_str()
    }

    /// Returns the canonical, absolute form of the path with all intermediate
    /// components normalized and symbolic links resolved.
    ///
    /// This is an alias to [`fs::canonicalize`].
    ///
    /// [`fs::canonicalize`]: ../fs/fn.canonicalize.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let path = Path::new("/foo/test/../test/bar.rs");
    /// assert_eq!(path.canonicalize().await.unwrap(), PathBuf::from("/foo/test/bar.rs"));
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn canonicalize(&self) -> io::Result<PathBuf> {
        fs::canonicalize(self).await
    }

    /// Produces an iterator over the [`Component`]s of the path.
    ///
    /// When parsing the path, there is a small amount of normalization:
    ///
    /// * Repeated separators are ignored, so `a/b` and `a//b` both have
    ///   `a` and `b` as components.
    ///
    /// * Occurrences of `.` are normalized away, except if they are at the
    ///   beginning of the path. For example, `a/./b`, `a/b/`, `a/b/.` and
    ///   `a/b` all have `a` and `b` as components, but `./a/b` starts with
    ///   an additional [`CurDir`] component.
    ///
    /// * A trailing slash is normalized away, `/a/b` and `/a/b/` are equivalent.
    ///
    /// Note that no other normalization takes place; in particular, `a/c`
    /// and `a/b/../c` are distinct, to account for the possibility that `b`
    /// is a symbolic link (so its parent isn't `a`).
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, Component};
    /// use std::ffi::OsStr;
    ///
    /// let mut components = Path::new("/tmp/foo.txt").components();
    ///
    /// assert_eq!(components.next(), Some(Component::RootDir));
    /// assert_eq!(components.next(), Some(Component::Normal(OsStr::new("tmp"))));
    /// assert_eq!(components.next(), Some(Component::Normal(OsStr::new("foo.txt"))));
    /// assert_eq!(components.next(), None)
    /// ```
    ///
    /// [`Component`]: enum.Component.html
    /// [`CurDir`]: enum.Component.html#variant.CurDir
    pub fn components(&self) -> Components<'_> {
        self.inner.components()
    }

    /// Returns `true` if the path points at an existing entity.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file. In case of broken symbolic links this will return `false`.
    ///
    /// If you cannot access the directory containing the file, e.g., because of a
    /// permission error, this will return `false`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::path::Path;
    /// assert_eq!(Path::new("does_not_exist.txt").exists().await, false);
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// # See Also
    ///
    /// This is a convenience function that coerces errors to false. If you want to
    /// check errors, call [fs::metadata].
    ///
    /// [fs::metadata]: ../fs/fn.metadata.html
    pub async fn exists(&self) -> bool {
        fs::metadata(self).await.is_ok()
    }

    /// Queries the file system to get information about a file, directory, etc.
    ///
    /// This function will traverse symbolic links to query information about the
    /// destination file.
    ///
    /// This is an alias to [`fs::metadata`].
    ///
    /// [`fs::metadata`]: ../fs/fn.metadata.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::path::Path;
    ///
    /// let path = Path::new("/Minas/tirith");
    /// let metadata = path.metadata().await.expect("metadata call failed");
    /// println!("{:?}", metadata.file_type());
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(self).await
    }

    /// Queries the metadata about a file without following symlinks.
    ///
    /// This is an alias to [`fs::symlink_metadata`].
    ///
    /// [`fs::symlink_metadata`]: ../fs/fn.symlink_metadata.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::path::Path;
    ///
    /// let path = Path::new("/Minas/tirith");
    /// let metadata = path.symlink_metadata().await.expect("symlink_metadata call failed");
    /// println!("{:?}", metadata.file_type());
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(self).await
    }

    /// Directly wraps a string slice as a `Path` slice.
    ///
    /// This is a cost-free conversion.
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::Path;
    ///
    /// Path::new("foo.txt");
    /// ```
    ///
    /// You can create `Path`s from `String`s, or even other `Path`s:
    ///
    /// ```
    /// use async_std::path::Path;
    ///
    /// let string = String::from("foo.txt");
    /// let from_string = Path::new(&string);
    /// let from_path = Path::new(&from_string);
    /// assert_eq!(from_string, from_path);
    /// ```
    pub fn new<S: AsRef<OsStr> + ?Sized>(s: &S) -> &Path {
        unsafe { &*(std::path::Path::new(s) as *const std::path::Path as *const Path) }
    }

    /// Converts a `Path` to an owned [`PathBuf`].
    ///
    /// [`PathBuf`]: struct.PathBuf.html
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let path_buf = Path::new("foo.txt").to_path_buf();
    /// assert_eq!(path_buf, PathBuf::from("foo.txt"));
    /// ```
    pub fn to_path_buf(&self) -> PathBuf {
        PathBuf::from(self.inner.to_path_buf())
    }
}

impl<'a> From<&'a std::path::Path> for &'a Path {
    fn from(path: &'a std::path::Path) -> &'a Path {
        &Path::new(path.as_os_str())
    }
}

impl<'a> Into<&'a std::path::Path> for &'a Path {
    fn into(self) -> &'a std::path::Path {
        std::path::Path::new(&self.inner)
    }
}

impl AsRef<OsStr> for Path {
    fn as_ref(&self) -> &OsStr {
        self.inner.as_ref()
    }
}

impl AsRef<Path> for Path {
    fn as_ref(&self) -> &Path {
        self
    }
}

impl AsRef<Path> for OsStr {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl AsRef<Path> for str {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}
