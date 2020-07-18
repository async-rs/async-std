use std::cell::RefCell;
use std::future::Future;
use std::task::{Context, Poll};

static GLOBAL_EXECUTOR: once_cell::sync::Lazy<multitask::Executor> = once_cell::sync::Lazy::new(multitask::Executor::new);

struct Executor {
    local_executor: multitask::LocalExecutor,
    parker: async_io::parking::Parker,
}

thread_local! {
    static EXECUTOR: RefCell<Executor> = RefCell::new({
        let (parker, unparker) = async_io::parking::pair();
        let local_executor = multitask::LocalExecutor::new(move || unparker.unpark());
        Executor { local_executor, parker }
    });
}

pub(crate) fn spawn<F, T>(future: F) -> multitask::Task<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    GLOBAL_EXECUTOR.spawn(future)
}

#[cfg(feature = "unstable")]
pub(crate) fn local<F, T>(future: F) -> multitask::Task<T>
where
    F: Future<Output = T> + 'static,
    T: 'static,
{
    EXECUTOR.with(|executor| executor.borrow().local_executor.spawn(future))
}

pub(crate) fn run<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    enter(|| EXECUTOR.with(|executor| {
        let executor = executor.borrow();
        let unparker = executor.parker.unparker();
        let global_ticker = GLOBAL_EXECUTOR.ticker(move || unparker.unpark());
        let unparker = executor.parker.unparker();
        let waker = async_task::waker_fn(move || unparker.unpark());
        let cx = &mut Context::from_waker(&waker);
        pin_utils::pin_mut!(future);
        loop {
            if let Poll::Ready(res) = future.as_mut().poll(cx) {
                return res;
            }
            if let Ok(false) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| executor.local_executor.tick() || global_ticker.tick())) {
                executor.parker.park();
            }
        }
    }))
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
