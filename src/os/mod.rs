//! OS-specific extensions.

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
#[cfg(any(unix, feature = "docs"))]
pub mod unix;

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
#[cfg(any(windows, feature = "docs"))]
pub mod windows;
