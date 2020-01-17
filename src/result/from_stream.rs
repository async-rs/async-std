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
    fn from_stream<'a, S: IntoStream<Item = Result<T, E>> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            // Using `take_while` here because it is able to stop the stream early
            // if a failure occurs
            let mut is_error = false;
            let mut found_error = None;
            let out: V = stream
                .take_while(|elem| {
                    // Stop processing the stream on `Err`
                    !is_error
                        && (elem.is_ok() || {
                            is_error = true;
                            // Capture first `Err`
                            true
                        })
                })
                .filter_map(|elem| match elem {
                    Ok(value) => Some(value),
                    Err(err) => {
                        found_error = Some(err);
                        None
                    }
                })
                .collect()
                .await;

            if is_error {
                Err(found_error.unwrap())
            } else {
                Ok(out)
            }
        })
    }
}
