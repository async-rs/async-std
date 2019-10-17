//! OS-specific extensions.

crate::cfg_unix! {
    pub mod unix;
}

crate::cfg_windows! {
    pub mod windows;
}
