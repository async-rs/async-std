use std::borrow::Cow;
use std::pin::Pin;

use crate::stream::{Extend, FromStream, IntoStream};

impl FromStream<char> for String {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = char>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<&'b char> for String {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = &'b char>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<&'b str> for String {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = &'b str>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl FromStream<String> for String {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = String>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<Cow<'b, str>> for String {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = Cow<'b, str>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}
