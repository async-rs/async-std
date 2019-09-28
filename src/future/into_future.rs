use crate::future::Future;

/// Convert a type into a `Future`.
pub trait IntoFuture {
    /// The type of value produced on completion.
    type Output;

    /// Which kind of future are we turning this into?
    type Future: Future<Output = Self::Output>;

    /// Create a future from a value
    fn into_future(self) -> Self::Future;
}

impl<T: Future> IntoFuture for T {
    type Output = T::Output;

    type Future = T;

    fn into_future(self) -> Self::Future {
        self
    }
}
