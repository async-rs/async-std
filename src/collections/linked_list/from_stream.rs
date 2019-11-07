use std::collections::LinkedList;
use std::pin::Pin;

use crate::stream::{self, FromStream, IntoStream};

impl<T> FromStream<T> for LinkedList<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = LinkedList::new();
            stream::extend(&mut out, stream).await;
            out
        })
    }
}
