use std::borrow::Cow;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::prelude::*;
use crate::stream::{self, FromStream, IntoStream};

impl<T> FromStream<T> for Vec<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            let mut out = vec![];
            stream::extend(&mut out, stream).await;
            out
        })
    }
}

impl<'b, T: Clone> FromStream<T> for Cow<'b, [T]> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            Cow::Owned(FromStream::from_stream(stream).await)
        })
    }
}

impl<T> FromStream<T> for Box<[T]> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            Vec::from_stream(stream).await.into_boxed_slice()
        })
    }
}

impl<T> FromStream<T> for Rc<[T]> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            Vec::from_stream(stream).await.into()
        })
    }
}

impl<T> FromStream<T> for Arc<[T]> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            Vec::from_stream(stream).await.into()
        })
    }
}
