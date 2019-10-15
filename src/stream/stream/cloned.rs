#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ClonedFuture<S> {
    stream: S,
}

impl<S> ClonedFuture<S> {
    pub fn new(stream: S) -> ClonedFuture<S> {
        ClonedFuture { stream }
    }
}
