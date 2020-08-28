use std::future::Future;

pub(crate) fn run<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    enter(|| async_global_executor::block_on(future))
}

/// Enters the tokio context if the `tokio` feature is enabled.
fn enter<T>(f: impl FnOnce() -> T) -> T {
    #[cfg(not(feature = "tokio02"))]
    return f();

    #[cfg(feature = "tokio02")]
    {
        use std::cell::Cell;
        use tokio::runtime::Runtime;

        thread_local! {
            /// The level of nested `enter` calls we are in, to ensure that the outermost always
            /// has a runtime spawned.
            static NESTING: Cell<usize> = Cell::new(0);
        }

        /// The global tokio runtime.
        static RT: once_cell::sync::Lazy<Runtime> = once_cell::sync::Lazy::new(|| Runtime::new().expect("cannot initialize tokio"));

        NESTING.with(|nesting| {
            let res = if nesting.get() == 0 {
                nesting.replace(1);
                RT.enter(f)
            } else {
                nesting.replace(nesting.get() + 1);
                f()
            };
            nesting.replace(nesting.get() - 1);
            res
        })
    }
}
