use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{FromStream, IntoStream};

impl<T, E, V> FromStream<Result<T, E>> for Result<V, E>
where
    V: FromStream<T>,
{
    /// Takes each element in the stream: if it is an `Err`, no further
    /// elements are taken, and the `Err` is returned. Should no `Err`
    /// occur, a container with the values of each `Result` is returned.
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = Result<T, E>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Pin::from(Box::new(async move {
            pin_utils::pin_mut!(stream);

            // Using `scan` here because it is able to stop the stream early
            // if a failure occurs
            let mut found_error = None;
            let out: V = stream
                .scan((), |_, elem| {
                    match elem {
                        Ok(elem) => Some(elem),
                        Err(err) => {
                            found_error = Some(err);
                            // Stop processing the stream on error
                            None
                        }
                    }
                })
                .collect()
                .await;

            match found_error {
                Some(err) => Err(err),
                None => Ok(out),
            }
        }))
    }
}
