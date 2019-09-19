use crate::stream::{FromStream, IntoStream};

use std::pin::Pin;

impl<T: Send + Unpin> FromStream<T> for Vec<T> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + Send + Unpin + 'a>>
    where
        <S as IntoStream>::IntoStream: Send + 'a,
    {
        let _stream = stream.into_stream();

        // Box::pin(async move {
        //     pin_utils::pin_mut!(stream);

        //     let mut out = vec![];
        //     while let Some(item) = stream.next().await {
        //         out.push(item);
        //     }
        //     out
        // })
        panic!();
    }
}
