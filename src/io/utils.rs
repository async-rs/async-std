use crate::utils::VerboseErrorExt;

/// Wrap `std::io::Error` with additional message
///
/// *Note* Only active when `verbose-errors` feature is enabled for this crate!
///
/// Keeps the original error kind and stores the original I/O error as `source`.
impl<T> VerboseErrorExt for Result<T, std::io::Error> {
    #[cfg(feature = "verbose-errors")]
    fn verbose_context(self, message: impl Fn() -> String) -> Self {
        self.map_err(|e| verbose::Error::wrap(e, message()))
    }
}

#[cfg(feature = "verbose-errors")]
mod verbose {
    use std::{error::Error as StdError, fmt, io};

    #[derive(Debug)]
    pub(crate) struct Error {
        source: io::Error,
        message: String,
    }

    impl Error {
        pub(crate) fn wrap(source: io::Error, message: impl Into<String>) -> io::Error {
            io::Error::new(
                source.kind(),
                Error {
                    source,
                    message: message.into(),
                },
            )
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl StdError for Error {
        fn description(&self) -> &str {
            self.source.description()
        }

        fn source(&self) -> Option<&(dyn StdError + 'static)> {
            Some(&self.source)
        }
    }
}
