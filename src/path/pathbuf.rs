use std::ffi::OsString;

use crate::path::Path;

/// This struct is an async version of [`std::path::PathBuf`].
///
/// [`std::path::Path`]: https://doc.rust-lang.org/std/path/struct.PathBuf.html
#[derive(Debug, PartialEq)]
pub struct PathBuf {
    inner: std::path::PathBuf,
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
