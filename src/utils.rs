use alloc::string::String;

/// Calls a function and aborts if it panics.
///
/// This is useful in unsafe code where we can't recover from panics.
#[cfg(feature = "default")]
#[inline]
pub fn abort_on_panic<T>(f: impl FnOnce() -> T) -> T {
    struct Bomb;

    impl Drop for Bomb {
        fn drop(&mut self) {
            std::process::abort();
        }
    }

    let bomb = Bomb;
    let t = f();
    std::mem::forget(bomb);
    t
}

/// Generates a random number in `0..n`.
#[cfg(any(feature = "unstable", feature = "default"))]
pub fn random(n: u32) -> u32 {
    use std::cell::Cell;
    use std::num::Wrapping;

    thread_local! {
        static RNG: Cell<Wrapping<u32>> = {
            // Take the address of a local value as seed.
            let mut x = 0i32;
            let r = &mut x;
            let addr = r as *mut i32 as usize;
            Cell::new(Wrapping(addr as u32))
        }
    }

    RNG.with(|rng| {
        // This is the 32-bit variant of Xorshift.
        //
        // Source: https://en.wikipedia.org/wiki/Xorshift
        let mut x = rng.get();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        rng.set(x);

        // This is a fast alternative to `x % n`.
        //
        // Author: Daniel Lemire
        // Source: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction/
        ((u64::from(x.0)).wrapping_mul(u64::from(n)) >> 32) as u32
    })
}

/// Add additional context to errors
pub(crate) trait Context {
    fn context(self, message: impl Fn() -> String) -> Self;
}

/// Defers evaluation of a block of code until the end of the scope.
#[cfg(feature = "default")]
#[doc(hidden)]
macro_rules! defer {
    ($($body:tt)*) => {
        let _guard = {
            pub struct Guard<F: FnOnce()>(Option<F>);

            impl<F: FnOnce()> Drop for Guard<F> {
                fn drop(&mut self) {
                    (self.0).take().map(|f| f());
                }
            }

            Guard(Some(|| {
                let _ = { $($body)* };
            }))
        };
    };
}

/// Declares unstable items.
#[doc(hidden)]
macro_rules! cfg_unstable {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "unstable")]
            #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
            $item
        )*
    }
}

/// Declares unstable and default items.
#[doc(hidden)]
macro_rules! cfg_unstable_default {
    ($($item:item)*) => {
        $(
            #[cfg(all(feature = "default", feature = "unstable"))]
            #[cfg_attr(feature = "docs", doc(unstable))]
            $item
        )*
    }
}

/// Declares Unix-specific items.
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(any(unix, feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(unix)))]
            $item
        )*
    }
}

/// Declares Windows-specific items.
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(any(windows, feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(windows)))]
            $item
        )*
    }
}

/// Declares items when the "docs" feature is enabled.
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! cfg_docs {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "docs")]
            $item
        )*
    }
}

/// Declares items when the "docs" feature is disabled.
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! cfg_not_docs {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "docs"))]
            $item
        )*
    }
}

/// Declares std items.
#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! cfg_std {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "std")]
            $item
        )*
    }
}

/// Declares no-std items.
#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! cfg_alloc {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "alloc")]
            $item
        )*
    }
}

/// Declares default items.
#[allow(unused_macros)]
#[doc(hidden)]
macro_rules! cfg_default {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "default")]
            $item
        )*
    }
}

/// Defines an extension trait for a base trait.
///
/// In generated docs, the base trait will contain methods from the extension trait. In actual
/// code, the base trait will be re-exported and the extension trait will be hidden. We then
/// re-export the extension trait from the prelude.
///
/// Inside invocations of this macro, we write a definitions that looks similar to the final
/// rendered docs, and the macro then generates all the boilerplate for us.
#[allow(unused_macros)]
#[doc(hidden)]
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

        #[doc = $doc_ext:tt]
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
            pub struct ImplFuture<T>(core::marker::PhantomData<T>);
        }

        // A fake `impl Future` type that borrows its environment.
        #[allow(dead_code)]
        mod borrowed {
            #[doc(hidden)]
            pub struct ImplFuture<'a, T>(core::marker::PhantomData<&'a T>);
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
        #[doc = $doc_ext]
        pub trait $ext: $name {
            extension_trait!(@ext () $($body_ext)*);
        }

        // Blanket implementation of the extension trait for any type implementing the base trait.
        impl<T: $name + ?Sized> $ext for T {}

        // Shim trait impls that only appear in docs.
        $(#[cfg(feature = "docs")] $imp)*
    };

    // Optimization: expand `$head` eagerly before starting a new method definition.
    (@ext ($($head:tt)*) #[doc = $d:literal] $($tail:tt)*) => {
        $($head)* extension_trait!(@ext (#[doc = $d]) $($tail)*);
    };

    // Parse the return type in an extension method.
    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> $(+ $lt:lifetime)? [$f:ty] $($tail:tt)*) => {
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

    // Parse imports at the beginning of the macro.
    ($import:item $($tail:tt)*) => {
        #[cfg(feature = "docs")]
        $import

        extension_trait!($($tail)*);
    };
}
