#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ClonedFuture<S> {
    stream: S,
}
