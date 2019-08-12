//! OS-specific extensions.

#[cfg(any(unix, feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unix)))]
pub mod unix;

#[cfg(any(windows, feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(windows)))]
pub mod windows;
