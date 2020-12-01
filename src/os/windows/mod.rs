//! Platform-specific extensions for Windows.

cfg_std! {
    pub mod io;
}

cfg_unstable! {
    #[cfg(feature = "default")]
    pub mod fs;
}

#[cfg(all(feature = "unstable", feature = "std"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[doc(inline)]
pub use async_process::windows as process;
