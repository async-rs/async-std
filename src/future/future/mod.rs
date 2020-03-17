cfg_unstable! {
    mod delay;
    mod flatten;
    mod race;
    mod try_race;
    mod join;
    mod try_join;

    use std::time::Duration;
    use delay::DelayFuture;
    use flatten::FlattenFuture;
    use crate::future::IntoFuture;
    use race::Race;
    use try_race::TryRace;
    use join::Join;
    use try_join::TryJoin;
}

cfg_unstable_default! {
    use crate::future::timeout::TimeoutFuture;
}

extension_trait! {
    use core::pin::Pin;
    use core::ops::{Deref, DerefMut};

    use crate::task::{Context, Poll};

    #[doc = r#"
        A future represents an asynchronous computation.

        A future is a value that may not have finished computing yet. This kind of
        "asynchronous value" makes it possible for a thread to continue doing useful
        work while it waits for the value to become available.

        The [provided methods] do not really exist in the trait itself, but they become
        available when [`FutureExt`] from the [prelude] is imported:

        ```
        # #[allow(unused_imports)]
        use async_std::prelude::*;
        ```

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
        [provided methods]: #provided-methods
        [`FutureExt`]: ../prelude/trait.FutureExt.html
        [prelude]: ../prelude/index.html
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

    #[doc = r#"
        Extension methods for [`Future`].

        [`Future`]: ../future/trait.Future.html
    "#]
    pub trait FutureExt: core::future::Future {
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
        #[cfg(feature = "unstable")]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn delay(self, dur: Duration) -> impl Future<Output = Self::Output> [DelayFuture<Self>]
        where
            Self: Sized,
        {
            DelayFuture::new(self, dur)
        }

        /// Flatten out the execution of this future when the result itself
        /// can be converted into another future.
        ///
        /// # Examples
        ///
        /// ```
        /// # async_std::task::block_on(async {
        /// use async_std::prelude::*;
        ///
        /// let nested_future = async { async { 1 } };
        /// let future = nested_future.flatten();
        /// assert_eq!(future.await, 1);
        /// # })
        /// ```
        #[cfg(feature = "unstable")]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn flatten(
            self,
        ) -> impl Future<Output = <Self::Output as IntoFuture>::Output>
            [FlattenFuture<Self, <Self::Output as IntoFuture>::Future>]
        where
            Self: Sized,
            <Self as Future>::Output: IntoFuture,
        {
           FlattenFuture::new(self)
        }

        #[doc = r#"
            Waits for one of two similarly-typed futures to complete.

            Awaits multiple futures simultaneously, returning the output of the
            first future that completes.

            This function will return a new future which awaits for either one of both
            futures to complete. If multiple futures are completed at the same time,
            resolution will occur in the order that they have been passed.

            Note that this function consumes all futures passed, and once a future is
            completed, all other futures are dropped.

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
        #[cfg(feature = "unstable")]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn race<F>(
            self,
            other: F,
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

            [`race`]: #method.race

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
        #[cfg(feature = "unstable")]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn try_race<F, T, E>(
            self,
            other: F
        ) -> impl Future<Output = <Self as std::future::Future>::Output> [TryRace<Self, F>]
        where
            Self: std::future::Future<Output = Result<T, E>> + Sized,
            F: std::future::Future<Output = <Self as std::future::Future>::Output>,
        {
            TryRace::new(self, other)
        }

        #[doc = r#"
            Waits for two similarly-typed futures to complete.

            Awaits multiple futures simultaneously, returning the output of the
            futures once both complete.

            This function returns a new future which polls both futures
            concurrently.

            # Examples

            ```
            # async_std::task::block_on(async {
            use async_std::prelude::*;
            use async_std::future;

            let a = future::ready(1u8);
            let b = future::ready(2u16);

            let f = a.join(b);
            assert_eq!(f.await, (1u8, 2u16));
            # });
            ```
        "#]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn join<F>(
            self,
            other: F
        ) -> impl Future<Output = (<Self as std::future::Future>::Output, <F as std::future::Future>::Output)> [Join<Self, F>]
        where
            Self: std::future::Future + Sized,
            F: std::future::Future,
        {
            Join::new(self, other)
        }

        #[doc = r#"
            Waits for two similarly-typed fallible futures to complete.

            Awaits multiple futures simultaneously, returning all results once
            complete.

            `try_join` is similar to [`join`], but returns an error immediately
            if a future resolves to an error.

            [`join`]: #method.join

            # Examples

            ```
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::future;

            let a = future::ready(Err::<u8, &str>("Error"));
            let b = future::ready(Ok(1u8));

            let f = a.try_join(b);
            assert_eq!(f.await, Err("Error"));

            let a = future::ready(Ok::<u8, String>(1u8));
            let b = future::ready(Ok::<u16, String>(2u16));

            let f = a.try_join(b);
            assert_eq!(f.await, Ok((1u8, 2u16)));
            #
            # Ok(()) }) }
            ```
        "#]
        #[cfg(any(feature = "unstable", feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn try_join<F, A, B, E>(
            self,
            other: F
        ) -> impl Future<Output = Result<(A, B), E>> [TryJoin<Self, F>]
        where
            Self: std::future::Future<Output = Result<A, E>> + Sized,
            F: std::future::Future<Output = Result<B, E>>,
        {
            TryJoin::new(self, other)
        }

        #[doc = r#"
            Waits for both the future and a timeout, if the timeout completes before
            the future, it returns an TimeoutError.

            # Example
            ```
            # async_std::task::block_on(async {
            #
            use std::time::Duration;

            use async_std::prelude::*;
            use async_std::future;

            let fut = future::ready(0);
            let dur = Duration::from_millis(100);
            let res = fut.timeout(dur).await;
            assert!(res.is_ok());

            let fut = future::pending::<()>();
            let dur = Duration::from_millis(100);
            let res = fut.timeout(dur).await;
            assert!(res.is_err())
            #
            # });
            ```
        "#]
        #[cfg(any(all(feature = "default", feature = "unstable"), feature = "docs"))]
        #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
        fn timeout(self, dur: Duration) -> impl Future<Output = Self::Output> [TimeoutFuture<Self>]
            where Self: Sized
        {
            TimeoutFuture::new(self, dur)
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
