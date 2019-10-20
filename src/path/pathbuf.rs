use std::ffi::{OsStr, OsString};
#[cfg(feature = "unstable")]
use std::pin::Pin;

use crate::path::Path;
#[cfg(feature = "unstable")]
use crate::prelude::*;
#[cfg(feature = "unstable")]
use crate::stream::{Extend, FromStream, IntoStream};

/// This struct is an async version of [`std::path::PathBuf`].
///
/// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.PathBuf.html
#[derive(Debug, PartialEq)]
pub struct PathBuf {
    inner: std::path::PathBuf,
}

impl PathBuf {
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

    /// Extends `self` with `path`.
    ///
    /// If `path` is absolute, it replaces the current path.
    ///
    /// On Windows:
    ///
    /// * if `path` has a root but no prefix (e.g., `\windows`), it
    ///   replaces everything except for the prefix (if any) of `self`.
    /// * if `path` has a prefix but no root, it replaces `self`.
    ///
    /// # Examples
    ///
    /// Pushing a relative path extends the existing path:
    ///
    /// ```
    /// use async_std::path::PathBuf;
    ///
    /// let mut path = PathBuf::from("/tmp");
    /// path.push("file.bk");
    /// assert_eq!(path, PathBuf::from("/tmp/file.bk"));
    /// ```
    ///
    /// Pushing an absolute path replaces the existing path:
    ///
    /// ```
    /// use async_std::path::PathBuf;
    ///
    /// let mut path = PathBuf::from("/tmp");
    /// path.push("/etc");
    /// assert_eq!(path, PathBuf::from("/etc"));
    /// ```
    pub fn push<P: AsRef<Path>>(&mut self, path: P) {
        self.inner.push(path.as_ref())
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

    /// Updates [`self.file_name`] to `file_name`.
    ///
    /// If [`self.file_name`] was [`None`], this is equivalent to pushing
    /// `file_name`.
    ///
    /// Otherwise it is equivalent to calling [`pop`] and then pushing
    /// `file_name`. The new path will be a sibling of the original path.
    /// (That is, it will have the same parent.)
    ///
    /// [`self.file_name`]: struct.PathBuf.html#method.file_name
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    /// [`pop`]: struct.PathBuf.html#method.pop
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::PathBuf;
    ///
    /// let mut buf = PathBuf::from("/");
    /// assert!(buf.file_name() == None);
    /// buf.set_file_name("bar");
    /// assert!(buf == PathBuf::from("/bar"));
    /// assert!(buf.file_name().is_some());
    /// buf.set_file_name("baz.txt");
    /// assert!(buf == PathBuf::from("/baz.txt"));
    /// ```
    pub fn set_file_name<S: AsRef<OsStr>>(&mut self, file_name: S) {
        self.inner.set_file_name(file_name)
    }

    /// Updates [`self.extension`] to `extension`.
    ///
    /// Returns `false` and does nothing if [`self.file_name`] is [`None`],
    /// returns `true` and updates the extension otherwise.
    ///
    /// If [`self.extension`] is [`None`], the extension is added; otherwise
    /// it is replaced.
    ///
    /// [`self.file_name`]: struct.PathBuf.html#method.file_name
    /// [`self.extension`]: struct.PathBuf.html#method.extension
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// # Examples
    ///
    /// ```
    /// use async_std::path::{Path, PathBuf};
    ///
    /// let mut p = PathBuf::from("/feel/the");
    ///
    /// p.set_extension("force");
    /// assert_eq!(Path::new("/feel/the.force"), p.as_path());
    ///
    /// p.set_extension("dark_side");
    /// assert_eq!(Path::new("/feel/the.dark_side"), p.as_path());
    /// ```
    pub fn set_extension<S: AsRef<OsStr>>(&mut self, extension: S) -> bool {
        self.inner.set_extension(extension)
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

    /// Converts this `PathBuf` into a [boxed][`Box`] [`Path`].
    ///
    /// [`Box`]: https://doc.rust-lang.org/std/boxed/struct.Box.html
    /// [`Path`]: struct.Path.html
    pub fn into_boxed_path(self) -> Box<Path> {
        let rw = Box::into_raw(self.inner.into_boxed_path()) as *mut Path;
        unsafe { Box::from_raw(rw) }
    }
}

impl std::ops::Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Path {
        self.as_ref()
    }
}

impl std::borrow::Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        &**self
    }
}

impl From<std::path::PathBuf> for PathBuf {
    fn from(path: std::path::PathBuf) -> PathBuf {
        PathBuf { inner: path }
    }
}

impl Into<std::path::PathBuf> for PathBuf {
    fn into(self) -> std::path::PathBuf {
        self.inner
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

#[cfg(feature = "unstable")]
impl<P: AsRef<Path>> Extend<P> for PathBuf {
    fn stream_extend<'a, S: IntoStream<Item = P>>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        P: 'a,
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        //TODO: This can be added back in once this issue is resolved:
        // https://github.com/rust-lang/rust/issues/58234
        //self.reserve(stream.size_hint().0);

        Box::pin(stream.for_each(move |item| self.push(item.as_ref())))
    }
}

#[cfg(feature = "unstable")]
impl<'b, P: AsRef<Path> + 'b> FromStream<P> for PathBuf {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = P>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = Self::new();
            out.stream_extend(stream).await;
            out
        })
    }
}
