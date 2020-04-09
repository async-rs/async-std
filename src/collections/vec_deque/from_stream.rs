use std::collections::VecDeque;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, FromStream, IntoStream};

impl<T> FromStream<T> for VecDeque<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            let mut out = VecDeque::new();
            stream::extend(&mut out, stream).await;
            out
        })
    }
}
