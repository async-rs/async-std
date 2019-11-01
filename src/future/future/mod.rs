cfg_unstable! {
    mod delay;
    mod race;
    mod try_race;

    use std::time::Duration;

    use delay::DelayFuture;
    use race::Race;
    use try_race::TryRace;
}

extension_trait! {
    use std::pin::Pin;
    use std::ops::{Deref, DerefMut};

    use crate::task::{Context, Poll};

    #[doc = r#"
        A future represents an asynchronous computation.

        A future is a value that may not have finished computing yet. This kind of
        "asynchronous value" makes it possible for a thread to continue doing useful
        work while it waits for the value to become available.

        # The `poll` method

        The core method of future, `poll`, *attempts* to resolve the future into a
        final value. This method does not block if the value is not ready. Instead,
        the current task is scheduled to be woken up when it's possible to make
        further progress by `poll`ing again. The `context` passed to the `poll`
        method can provide a [`Waker`], which is a handle for waking up the current
        task.

        When using a future, you generally won't call `poll` directly, but instead
        `.await` the value.

        [`Waker`]: ../task/struct.Waker.html
    "#]
    pub trait Future {
        #[doc = r#"
            The type of value produced on completion.
        "#]
        type Output;

        #[doc = r#"
            Attempt to resolve the future to a final value, registering
            the current task for wakeup if the value is not yet available.

            # Return value

            This function returns:

            - [`Poll::Pending`] if the future is not ready yet
            - [`Poll::Ready(val)`] with the result `val` of this future if it
              finished successfully.

            Once a future has finished, clients should not `poll` it again.

            When a future is not ready yet, `poll` returns `Poll::Pending` and
            stores a clone of the [`Waker`] copied from the current [`Context`].
            This [`Waker`] is then woken once the future can make progress.
            For example, a future waiting for a socket to become
            readable would call `.clone()` on the [`Waker`] and store it.
            When a signal arrives elsewhere indicating that the socket is readable,
            [`Waker::wake`] is called and the socket future's task is awoken.
            Once a task has been woken up, it should attempt to `poll` the future
            again, which may or may not produce a final value.

            Note that on multiple calls to `poll`, only the [`Waker`] from the
            [`Context`] passed to the most recent call should be scheduled to
            receive a wakeup.

            # Runtime characteristics

            Futures alone are *inert*; they must be *actively* `poll`ed to make
            progress, meaning that each time the current task is woken up, it should
            actively re-`poll` pending futures that it still has an interest in.

            The `poll` function is not called repeatedly in a tight loop -- instead,
            it should only be called when the future indicates that it is ready to
            make progress (by calling `wake()`). If you're familiar with the
            `poll(2)` or `select(2)` syscalls on Unix it's worth noting that futures
            typically do *not* suffer the same problems of "all wakeups must poll
            all events"; they are more like `epoll(4)`.

            An implementation of `poll` should strive to return quickly, and should
            not block. Returning quickly prevents unnecessarily clogging up
            threads or event loops. If it is known ahead of time that a call to
            `poll` may end up taking awhile, the work should be offloaded to a
            thread pool (or something similar) to ensure that `poll` can return
            quickly.

            # Panics

            Once a future has completed (returned `Ready` from `poll`), calling its
            `poll` method again may panic, block forever, or cause other kinds of
            problems; the `Future` trait places no requirements on the effects of
            such a call. However, as the `poll` method is not marked `unsafe`,
            Rust's usual rules apply: calls must never cause undefined behavior
            (memory corruption, incorrect use of `unsafe` functions, or the like),
            regardless of the future's state.

            [`Poll::Pending`]: ../task/enum.Poll.html#variant.Pending
            [`Poll::Ready(val)`]: ../task/enum.Poll.html#variant.Ready
            [`Context`]: ../task/struct.Context.html
            [`Waker`]: ../task/struct.Waker.html
            [`Waker::wake`]: ../task/struct.Waker.html#method.wake
        "#]
        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
    }

    pub trait FutureExt: std::future::Future {
        /// Returns a Future that delays execution for a specified time.
        ///
        /// # Examples
        ///
        /// ```
        /// # async_std::task::block_on(async {
        /// use async_std::prelude::*;
        /// use async_std::future;
        /// use std::time::Duration;
        ///
        /// let a = future::ready(1).delay(Duration::from_millis(2000));
        /// dbg!(a.await);
        /// # })
        /// ```
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        fn delay(self, dur: Duration) -> impl Future<Output = Self::Output> [DelayFuture<Self>]
        where
            Self: Future + Sized
        {
            DelayFuture::new(self, dur)
        }

        #[doc = r#"
            Waits for one of two similarly-typed futures to complete.

            Awaits multiple futures simultaneously, returning the output of the
            first future that completes.

            This function will return a new future which awaits for either one of both
            futures to complete. If multiple futures are completed at the same time,
            resolution will occur in the order that they have been passed.

            Note that this macro consumes all futures passed, and once a future is
            completed, all other futures are dropped.

            This macro is only usable inside of async functions, closures, and blocks.

            # Examples

            ```
            # async_std::task::block_on(async {
            use async_std::prelude::*;
            use async_std::future;

            let a = future::pending();
            let b = future::ready(1u8);
            let c = future::ready(2u8);

            let f = a.race(b).race(c);
            assert_eq!(f.await, 1u8);
            # });
            ```
        "#]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn race<F>(
            self,
            other: F
        ) -> impl Future<Output = <Self as std::future::Future>::Output> [Race<Self, F>]
        where
            Self: std::future::Future + Sized,
            F: std::future::Future<Output = <Self as std::future::Future>::Output>,
        {
            Race::new(self, other)
        }

        #[doc = r#"
            Waits for one of two similarly-typed fallible futures to complete.

            Awaits multiple futures simultaneously, returning all results once complete.

            `try_race` is similar to [`race`], but keeps going if a future
            resolved to an error until all futures have been resolved. In which case
            an error is returned.

            The ordering of which value is yielded when two futures resolve
            simultaneously is intentionally left unspecified.

            # Examples

            ```
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::future;
            use std::io::{Error, ErrorKind};

            let a = future::pending::<Result<_, Error>>();
            let b = future::ready(Err(Error::from(ErrorKind::Other)));
            let c = future::ready(Ok(1u8));

            let f = a.try_race(b).try_race(c);
            assert_eq!(f.await?, 1u8);
            #
            # Ok(()) }) }
            ```
        "#]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn try_race<F: std::future::Future, T, E>(
            self,
            other: F
        ) -> impl Future<Output = <Self as std::future::Future>::Output> [TryRace<Self, F>]
        where
            Self: std::future::Future<Output = Result<T, E>> + Sized,
            F: std::future::Future<Output = <Self as std::future::Future>::Output>,
        {
            TryRace::new(self, other)
        }
    }

    impl<F: Future + Unpin + ?Sized> Future for Box<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<F: Future + Unpin + ?Sized> Future for &mut F {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<P> Future for Pin<P>
    where
        P: DerefMut + Unpin,
        <P as Deref>::Target: Future,
    {
        type Output = <<P as Deref>::Target as Future>::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<F: Future> Future for std::panic::AssertUnwindSafe<F> {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            unreachable!("this impl only appears in the rendered docs")
        }
    }
}
