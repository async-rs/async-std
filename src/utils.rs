use std::mem;
use std::process;

/// Calls a function and aborts if it panics.
///
/// This is useful in unsafe code where we can't recover from panics.
#[inline]
pub fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    struct Bomb;

    impl Drop for Bomb {
        fn drop(&mut self) {
            process::abort();
        }
    }

    let bomb = Bomb;
    let t = f();
    mem::forget(bomb);
    t
}

/// Defines an extension trait for a base trait from the `futures` crate.
///
/// In generated docs, the base trait will contain methods from the extension trait. In actual
/// code, the base trait will be re-exported and the extension trait will be hidden. We then
/// re-export the extension trait from the prelude.
///
/// Inside invocations of this macro, we write a definitions that looks similar to the final
/// rendered docs, and the macro then generates all the boilerplate for us.
#[doc(hidden)]
#[macro_export]
macro_rules! extension_trait {
    (
        // Interesting patterns:
        // - `$name`: trait name that gets rendered in the docs
        // - `$ext`: name of the hidden extension trait
        // - `$base`: base trait from the `futures` crate
        $(#[$attr:meta])*
        pub trait $name:ident [$ext:ident: $base:path] {
            $($body:tt)*
        }

        // Shim trait impls that only appear in docs.
        $($imp:item)*
    ) => {
        // A fake `impl Future` type that doesn't borrow.
        #[allow(dead_code)]
        mod owned {
            #[doc(hidden)]
            pub struct ImplFuture<T>(std::marker::PhantomData<T>);
        }

        // A fake `impl Future` type that borrows its environment.
        #[allow(dead_code)]
        mod borrowed {
            #[doc(hidden)]
            pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);
        }

        // Render a fake trait containing all methods from the base trait and the extension trait.
        #[cfg(feature = "docs")]
        $(#[$attr])*
        pub trait $name {
            extension_trait!(@doc () $($body)*);
        }

        // When not rendering docs, re-export the base trait from the futures crate.
        #[cfg(not(feature = "docs"))]
        pub use $base as $name;

        // The extension trait that adds methods to any type implementing the base trait.
        $(#[$attr])*
        pub trait $ext: $base {
            extension_trait!(@ext () $($body)*);
        }

        // Blanket implementation of the extension trait for any type implementing the base trait.
        impl<T: $base + ?Sized> $ext for T {}

        // Shim trait impls that only appear in docs.
        $(#[cfg(feature = "docs")] $imp)*
    };

    // Parse an associated type.
    (@doc ($($head:tt)*) type $name:ident; $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* type $name;) $($tail)*);
    };
    (@ext ($($head:tt)*) type $ident:ty; $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    // Parse a required method.
    (@doc ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* fn $name $args $(-> $ret)?;) $($tail)*);
    };
    (@ext ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    // Parse a provided method that exists in the base trait.
    (@doc ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)? { $($body:tt)* } $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* fn $name $args $(-> $ret)? { $($body)* }) $($tail)*);
    };
    (@ext ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)? { $($body:tt)* } $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    // Parse the return type in an extension method where the future doesn't borrow.
    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> owned::ImplFuture<$out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    // Parse the return type in an extension method where the future borrows its environment.
    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> borrowed::ImplFuture<$lt, $out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    // Parse a token that doesn't fit into any of the previous patterns.
    (@doc ($($head:tt)*) $token:tt $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* $token) $($tail)*);
    };
    (@ext ($($head:tt)*) $token:tt $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* $token) $($tail)*);
    };

    // Handle the end of the token list.
    (@doc ($($head:tt)*)) => { $($head)* };
    (@ext ($($head:tt)*)) => { $($head)* };
}

pub use crate::extension_trait;
