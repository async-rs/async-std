use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{FromStream, IntoStream};

impl<T> FromStream<T> for Vec<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Pin::from(Box::new(async move {
            pin_utils::pin_mut!(stream);

            let mut out = vec![];
            while let Some(item) = stream.next().await {
                out.push(item);
            }
            out
        }))
    }
}
