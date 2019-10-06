use std::collections::VecDeque;
use std::pin::Pin;

use crate::stream::{Extend, FromStream, IntoStream};

impl<T> FromStream<T> for VecDeque<T> {
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

            let mut out = VecDeque::new();
            out.stream_extend(stream).await;
            out
        })
    }
}
