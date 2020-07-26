//! Platform-specific extensions for Windows.

cfg_std! {
    pub mod io;
}

cfg_unstable! {
    #[cfg(feature = "default")]
    pub mod fs;
}
