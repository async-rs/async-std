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

/// Defines an extension trait for a base trait.
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
        // - `$base`: base trait
        #[doc = $doc:tt]
        pub trait $name:ident {
            $($body_base:tt)*
        }

        pub trait $ext:ident: $base:path {
            $($body_ext:tt)*
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

        // Render a fake trait combining the bodies of the base trait and the extension trait.
        #[cfg(feature = "docs")]
        #[doc = $doc]
        pub trait $name {
            extension_trait!(@doc () $($body_base)* $($body_ext)*);
        }

        // When not rendering docs, re-export the base trait from the futures crate.
        #[cfg(not(feature = "docs"))]
        pub use $base as $name;

        // The extension trait that adds methods to any type implementing the base trait.
        /// Extension trait.
        pub trait $ext: $base {
            extension_trait!(@ext () $($body_ext)*);
        }

        // Blanket implementation of the extension trait for any type implementing the base trait.
        impl<T: $base + ?Sized> $ext for T {}

        // Shim trait impls that only appear in docs.
        $(#[cfg(feature = "docs")] $imp)*
    };

    // Parse the return type in an extension method.
    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> owned::ImplFuture<$out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> $(+ $lt:lifetime)? [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    // Parse the return type in an extension method.
    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> borrowed::ImplFuture<$lt, $out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    // Parse a token.
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
