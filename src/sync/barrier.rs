use crate::sync::{Condvar,Mutex};

/// A barrier enables multiple tasks to synchronize the beginning
/// of some computation.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::sync::{Arc, Barrier};
/// use async_std::task;
///
/// let mut handles = Vec::with_capacity(10);
/// let barrier = Arc::new(Barrier::new(10));
/// for _ in 0..10 {
///     let c = barrier.clone();
///     // The same messages will be printed together.
///     // You will NOT see any interleaving.
///     handles.push(task::spawn(async move {
///         println!("before wait");
///         c.wait().await;
///         println!("after wait");
///     }));
/// }
/// // Wait for the other futures to finish.
/// for handle in handles {
///     handle.await;
/// }
/// # });
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[derive(Debug)]
pub struct Barrier {
    state: Mutex<BarrierState>,
    cvar: Condvar,
    num_tasks: usize,
}

// The inner state of a double barrier
#[derive(Debug)]
struct BarrierState {
    count: usize,
    generation_id: usize,
}

/// A `BarrierWaitResult` is returned by `wait` when all threads in the `Barrier` have rendezvoused.
///
/// [`wait`]: struct.Barrier.html#method.wait
/// [`Barrier`]: struct.Barrier.html
///
/// # Examples
///
/// ```
/// use async_std::sync::Barrier;
///
/// let barrier = Barrier::new(1);
/// let barrier_wait_result = barrier.wait();
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[derive(Debug, Clone)]
pub struct BarrierWaitResult(bool);

impl Barrier {
    /// Creates a new barrier that can block a given number of tasks.
    ///
    /// A barrier will block `n`-1 tasks which call [`wait`] and then wake up
    /// all tasks at once when the `n`th task calls [`wait`].
    ///
    /// [`wait`]: #method.wait
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Barrier;
    ///
    /// let barrier = Barrier::new(10);
    /// ```
    pub fn new(n: usize) -> Barrier {
        Barrier {
            state: Mutex::new(BarrierState {
                count: 0,
                generation_id: 1,
            }),
            cvar: Condvar::new(),
            num_tasks: n,
        }
    }

    /// Blocks the current task until all tasks have rendezvoused here.
    ///
    /// Barriers are re-usable after all tasks have rendezvoused once, and can
    /// be used continuously.
    ///
    /// A single (arbitrary) task will receive a [`BarrierWaitResult`] that
    /// returns `true` from [`is_leader`] when returning from this function, and
    /// all other tasks will receive a result that will return `false` from
    /// [`is_leader`].
    ///
    /// [`BarrierWaitResult`]: struct.BarrierWaitResult.html
    /// [`is_leader`]: struct.BarrierWaitResult.html#method.is_leader
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::{Arc, Barrier};
    /// use async_std::task;
    ///
    /// let mut handles = Vec::with_capacity(10);
    /// let barrier = Arc::new(Barrier::new(10));
    /// for _ in 0..10 {
    ///     let c = barrier.clone();
    ///     // The same messages will be printed together.
    ///     // You will NOT see any interleaving.
    ///     handles.push(task::spawn(async move {
    ///         println!("before wait");
    ///         c.wait().await;
    ///         println!("after wait");
    ///     }));
    /// }
    /// // Wait for the other futures to finish.
    /// for handle in handles {
    ///     handle.await;
    /// }
    /// # });
    /// ```
    pub async fn wait(&self) -> BarrierWaitResult {
        let mut state = self.state.lock().await;
        let local_gen = state.generation_id;
        state.count += 1;

        if state.count < self.num_tasks {
            while local_gen == state.generation_id && state.count < self.num_tasks {
                state = self.cvar.wait(state).await;
            }

            BarrierWaitResult(false)
        } else {
            state.count = 0;
            state.generation_id = state.generation_id.wrapping_add(1);
            self.cvar.notify_all();
            BarrierWaitResult(true)
        }
    }
}

impl BarrierWaitResult {
    /// Returns `true` if this task from [`wait`] is the "leader task".
    ///
    /// Only one task will have `true` returned from their result, all other
    /// tasks will have `false` returned.
    ///
    /// [`wait`]: struct.Barrier.html#method.wait
    ///
    /// # Examples
    ///
    /// ```
    /// # async_std::task::block_on(async {
    /// #
    /// use async_std::sync::Barrier;
    ///
    /// let barrier = Barrier::new(1);
    /// let barrier_wait_result = barrier.wait().await;
    /// println!("{:?}", barrier_wait_result.is_leader());
    /// # });
    /// ```
    pub fn is_leader(&self) -> bool {
        self.0
    }
}

#[cfg(all(test, not(target_os = "unknown")))]
mod test {
    use futures::channel::mpsc::unbounded;
    use futures::sink::SinkExt;
    use futures::stream::StreamExt;

    use crate::sync::{Arc, Barrier};
    use crate::task;

    #[test]
    fn test_barrier() {
        // NOTE(dignifiedquire): Based on the test in std, I was seeing some
        // race conditions, so running it in a loop to make sure things are
        // solid.

        for _ in 0..1_000 {
            task::block_on(async move {
                const N: usize = 10;

                let barrier = Arc::new(Barrier::new(N));
                let (tx, mut rx) = unbounded();

                for _ in 0..N - 1 {
                    let c = barrier.clone();
                    let mut tx = tx.clone();
                    task::spawn(async move {
                        let res = c.wait().await;

                        tx.send(res.is_leader()).await.unwrap();
                    });
                }

                // At this point, all spawned threads should be blocked,
                // so we shouldn't get anything from the port
                let res = rx.try_next();
                assert!(match res {
                    Err(_err) => true,
                    _ => false,
                });

                let mut leader_found = barrier.wait().await.is_leader();

                // Now, the barrier is cleared and we should get data.
                for _ in 0..N - 1 {
                    if rx.next().await.unwrap() {
                        assert!(!leader_found);
                        leader_found = true;
                    }
                }
                assert!(leader_found);
            });
        }
    }
}
