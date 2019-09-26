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

#[doc(hidden)]
#[macro_export]
macro_rules! extension_trait {
    (@gen ($($head:tt)*) pub trait $name:ident [$ext:ident: $orig:path] { $($body:tt)* } $($imp:item)*) => {
        #[allow(dead_code)]
        mod owned {
            #[doc(hidden)]
            pub struct ImplFuture<T>(std::marker::PhantomData<T>);
        }

        #[allow(dead_code)]
        mod borrowed {
            #[doc(hidden)]
            pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);
        }

        #[cfg(feature = "docs")]
        $($head)* pub trait $name {
            extension_trait!(@doc () $($body)*);
        }

        #[cfg(not(feature = "docs"))]
        pub use $orig as $name;

        $($head)* pub trait $ext: $orig {
            extension_trait!(@ext () $($body)*);
        }

        impl<T: $orig + ?Sized> $ext for T {}

        $(#[cfg(feature = "docs")] $imp)*
    };
    (@gen ($($head:tt)*) $token:tt $($tail:tt)*) => {
        extension_trait!(@gen ($($head)* $token) $($tail)*);
    };

    (@doc ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* fn $name $args $(-> $ret)?;) $($tail)*);
    };
    (@ext ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)?; $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    (@doc ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)? { $($body:tt)* } $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* fn $name $args $(-> $ret)? { $($body)* }) $($tail)*);
    };
    (@ext ($($head:tt)*) fn $name:ident $args:tt $(-> $ret:ty)? { $($body:tt)* } $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    (@doc ($($head:tt)*) type $name:ident; $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* type $name;) $($tail)*);
    };
    (@ext ($($head:tt)*) type $ident:ty; $($tail:tt)*) => {
        extension_trait!(@ext ($($head)*) $($tail)*);
    };

    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> owned::ImplFuture<$out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    (@doc ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* -> borrowed::ImplFuture<$lt, $out>) $($tail)*);
    };
    (@ext ($($head:tt)*) -> impl Future<Output = $out:ty> + $lt:lifetime [$f:ty] $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* -> $f) $($tail)*);
    };

    (@doc ($($head:tt)*) $token:tt $($tail:tt)*) => {
        extension_trait!(@doc ($($head)* $token) $($tail)*);
    };
    (@ext ($($head:tt)*) $token:tt $($tail:tt)*) => {
        extension_trait!(@ext ($($head)* $token) $($tail)*);
    };

    (@doc ($($head:tt)*)) => { $($head)* };
    (@ext ($($head:tt)*)) => { $($head)* };

    ($($head:tt)*) => { extension_trait!(@gen () $($head)*); };
}

pub use crate::extension_trait;
