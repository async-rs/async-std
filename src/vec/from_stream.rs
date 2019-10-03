use std::pin::Pin;
use std::borrow::Cow;

use crate::stream::{Extend, FromStream, IntoStream};

impl<T> FromStream<T> for Vec<T> {
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

            let mut out = vec![];
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b, T: Clone> FromStream<T> for Cow<'b, [T]> {
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

            Cow::Owned(FromStream::from_stream(stream).await)
        })
    }
}
