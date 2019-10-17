use std::pin::Pin;

use crate::stream::{IntoStream, Stream};
use crate::task::{Context, Poll};

/// Real logic of both `Flatten` and `FlatMap` which simply delegate to
/// this type.
#[derive(Clone, Debug)]
struct FlattenCompat<S, U> {
    stream: S,
    frontiter: Option<U>,
}
impl<S, U> FlattenCompat<S, U> {
    pin_utils::unsafe_unpinned!(stream: S);
    pin_utils::unsafe_unpinned!(frontiter: Option<U>);

    /// Adapts an iterator by flattening it, for use in `flatten()` and `flat_map()`.
    pub fn new(stream: S) -> FlattenCompat<S, U> {
        FlattenCompat {
            stream,
            frontiter: None,
        }
    }
}
