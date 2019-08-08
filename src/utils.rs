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
