/// Declares task-local values.
///
/// The macro wraps any number of static declarations and makes them task-local. Attributes and
/// visibility modifiers are allowed.
///
/// Each declared value is of the accessor type [`LocalKey`].
///
/// [`LocalKey`]: task/struct.LocalKey.html
///
/// # Examples
///
/// ```
/// #
/// use std::cell::Cell;
///
/// use async_std::prelude::*;
/// use async_std::task;
///
/// task_local! {
///     static VAL: Cell<u32> = Cell::new(5);
/// }
///
/// task::block_on(async {
///     let v = VAL.with(|c| c.get());
///     assert_eq!(v, 5);
/// });
/// ```
#[cfg(feature = "default")]
#[macro_export]
macro_rules! task_local {
    () => ();

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => (
        $(#[$attr])* $vis static $name: $crate::task::LocalKey<$t> = {
            #[inline]
            fn __init() -> $t {
                $init
            }

            $crate::task::LocalKey {
                __init,
                __key: ::std::sync::atomic::AtomicU32::new(0),
            }
        };
    );

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => (
        $crate::task_local!($(#[$attr])* $vis static $name: $t = $init);
        $crate::task_local!($($rest)*);
    );
}
