//! OS-specific extensions.

crate::unix! {
    pub mod unix;
}

crate::windows! {
    pub mod windows;
}
