use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{FromStream, IntoStream};

impl<T, V> FromStream<Option<T>> for Option<V>
where
    V: FromStream<T>,
{
    /// Takes each element in the stream: if it is `None`, no further
    /// elements are taken, and `None` is returned. Should no `None`
    /// occur, a container with the values of each `Option` is returned.
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = Option<T>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            // Using `scan` here because it is able to stop the stream early
            // if a failure occurs
            let mut found_error = false;
            let out: V = stream
                .scan((), |_, elem| {
                    match elem {
                        Some(elem) => Some(elem),
                        None => {
                            found_error = true;
                            // Stop processing the stream on error
                            None
                        }
                    }
                })
                .collect()
                .await;

            if found_error { None } else { Some(out) }
        })
    }
}
