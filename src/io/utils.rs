use std::{error::Error, fmt, io};
use crate::utils::VerboseErrorExt;

/// Wrap `std::io::Error` with additional message
///
/// *Note* Only active when `verbose-errors` feature is enabled for this crate!
///
/// Keeps the original error kind and stores the original I/O error as `source`.
impl<T> VerboseErrorExt for Result<T, io::Error> {
    fn verbose_context(self, message: impl Fn() -> String) -> Self {
        if cfg!(feature = "verbose-errors") {
            self.map_err(|e| VerboseError::wrap(e, message()))
        } else {
            self
        }
    }
}

#[derive(Debug)]
struct VerboseError {
    source: io::Error,
    message: String,
}

impl VerboseError {
    fn wrap(source: io::Error, message: impl Into<String>) -> io::Error {
        io::Error::new(
            source.kind(),
            VerboseError {
                source,
                message: message.into(),
            },
        )
    }
}

impl fmt::Display for VerboseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for VerboseError {
    fn description(&self) -> &str {
        self.source.description()
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}
