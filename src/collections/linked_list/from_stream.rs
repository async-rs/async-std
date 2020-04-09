use std::collections::LinkedList;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, FromStream, IntoStream};

impl<T> FromStream<T> for LinkedList<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            let mut out = LinkedList::new();
            stream::extend(&mut out, stream).await;
            out
        })
    }
}
