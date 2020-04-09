//! Platform-specific extensions for Unix platforms.

cfg_std! {
    pub mod io;
}

cfg_default! {
    pub mod fs;
    pub mod net;
}
