//! OS-specific extensions.

#[cfg(any(unix, feature = "docs.rs"))]
#[cfg_attr(feature = "docs.rs", doc(cfg(unix)))]
pub mod unix;

#[cfg(any(windows, feature = "docs.rs"))]
#[cfg_attr(feature = "docs.rs", doc(cfg(windows)))]
pub mod windows;
