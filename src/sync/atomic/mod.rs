//! Atomic types
//!
//! Atomic types provide primitive shared-memory communication between
//! threads, and are the building blocks of other concurrent
//! types.
//!
//! This module defines atomic versions of a select number of primitive
//! types, including [`AtomicBool`], [`AtomicIsize`], [`AtomicUsize`],
//! [`AtomicI8`], [`AtomicU16`], etc.
//! Atomic types present operations that, when used correctly, synchronize
//! updates between threads.
//!
//! [`AtomicBool`]: struct.AtomicBool.html
//! [`AtomicIsize`]: struct.AtomicIsize.html
//! [`AtomicUsize`]: struct.AtomicUsize.html
//! [`AtomicI8`]: struct.AtomicI8.html
//! [`AtomicU16`]: struct.AtomicU16.html
//!
//! Each method takes an [`Ordering`] which represents the strength of
//! the memory barrier for that operation. These orderings are the
//! same as [LLVM atomic orderings][1]. For more information see the [nomicon][2].
//!
//! [`Ordering`]: enum.Ordering.html
//!
//! [1]: https://llvm.org/docs/LangRef.html#memory-model-for-concurrent-operations
//! [2]: https://doc.rust-lang.org/nomicon/atomics.html
//!
//! Atomic variables are safe to share between threads (they implement [`Sync`])
//! but they do not themselves provide the mechanism for sharing and follow the
//! [threading model](https://doc.rust-lang.org/std/thread/index.html#the-threading-model) of rust.
//! The most common way to share an atomic variable is to put it into an [`Arc`][arc] (an
//! atomically-reference-counted shared pointer).
//!
//! [`Sync`]: https://doc.rust-lang.org/std/marker/trait.Sync.html
//! [arc]: ../struct.Arc.html
//!
//! Atomic types may be stored in static variables, initialized using
//! the constant initializers like [`AtomicBool::new`]. Atomic statics
//! are often used for lazy global initialization.
//!
//! [`AtomicBool::new`]: struct.AtomicBool.html#method.new
//!
//! # Portability
//!
//! All atomic types in this module are guaranteed to be [lock-free] if they're
//! available. This means they don't internally acquire a global mutex. Atomic
//! types and operations are not guaranteed to be wait-free. This means that
//! operations like `fetch_or` may be implemented with a compare-and-swap loop.
//!
//! Atomic operations may be implemented at the instruction layer with
//! larger-size atomics. For example some platforms use 4-byte atomic
//! instructions to implement `AtomicI8`. Note that this emulation should not
//! have an impact on correctness of code, it's just something to be aware of.
//!
//! The atomic types in this module may not be available on all platforms. The
//! atomic types here are all widely available, however, and can generally be
//! relied upon existing. Some notable exceptions are:
//!
//! * PowerPC and MIPS platforms with 32-bit pointers do not have `AtomicU64` or
//!   `AtomicI64` types.
//! * ARM platforms like `armv5te` that aren't for Linux do not have any atomics
//!   at all.
//! * ARM targets with `thumbv6m` do not have atomic operations at all.
//!
//! Note that future platforms may be added that also do not have support for
//! some atomic operations. Maximally portable code will want to be careful
//! about which atomic types are used. `AtomicUsize` and `AtomicIsize` are
//! generally the most portable, but even then they're not available everywhere.
//! For reference, the `std` library requires pointer-sized atomics, although
//! `core` does not.
//!
//! Currently you'll need to use `#[cfg(target_arch)]` primarily to
//! conditionally compile in code with atomics. There is an unstable
//! `#[cfg(target_has_atomic)]` as well which may be stabilized in the future.
//!
//! [lock-free]: https://en.wikipedia.org/wiki/Non-blocking_algorithm
//!
//! # Examples
//!
//! A simple spinlock:
//!
//! ```
//! use std::sync::Arc;
//! use std::sync::atomic::{AtomicUsize, Ordering};
//! use std::thread;
//!
//! fn main() {
//!     let spinlock = Arc::new(AtomicUsize::new(1));
//!
//!     let spinlock_clone = spinlock.clone();
//!     let thread = thread::spawn(move|| {
//!         spinlock_clone.store(0, Ordering::SeqCst);
//!     });
//!
//!     // Wait for the other thread to release the lock
//!     while spinlock.load(Ordering::SeqCst) != 0 {}
//!
//!     if let Err(panic) = thread.join() {
//!         println!("Thread had an error: {:?}", panic);
//!     }
//! }
//! ```
//!
//! Keep a global count of live threads:
//!
//! ```
//! use std::sync::atomic::{AtomicUsize, Ordering};
//!
//! static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);
//!
//! let old_thread_count = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::SeqCst);
//! println!("live threads: {}", old_thread_count + 1);
//! ```

#[doc(inline)]
pub use std::sync::atomic::{
    AtomicBool, AtomicI16, AtomicI32, AtomicI64, AtomicI8, AtomicIsize, AtomicPtr, AtomicU16,
    AtomicU32, AtomicU64, AtomicU8, AtomicUsize,
};

#[doc(inline)]
pub use std::sync::atomic::{compiler_fence, fence, spin_loop_hint};
