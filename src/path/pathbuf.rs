use std::ffi::OsString;

/// This struct is an async version of [`std::path::PathBuf`].
///
/// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.PathBuf.html
pub struct PathBuf {
    inner: OsString,
}

impl From<std::path::PathBuf> for PathBuf {
    fn from(path: std::path::PathBuf) -> PathBuf {
        PathBuf {
            inner: path.into_os_string(),
        }
    }
}

impl Into<std::path::PathBuf> for PathBuf {
    fn into(self) -> std::path::PathBuf {
        self.inner.into()
    }
}

impl From<OsString> for PathBuf {
    fn from(path: OsString) -> PathBuf {
        PathBuf { inner: path }
    }
}

impl std::fmt::Debug for PathBuf {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner, formatter)
    }
}
