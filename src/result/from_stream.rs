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
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() { async_std::task::block_on(async {
    /// #
    /// use async_std::prelude::*;
    /// use async_std::stream;
    ///
    /// let v = stream::from_iter(vec![1, 2]);
    /// let res: Result<Vec<u32>, &'static str> = v.map(|x: u32|
    ///     x.checked_add(1).ok_or("Overflow!")
    /// ).collect().await;
    /// assert_eq!(res, Ok(vec![2, 3]));
    /// #
    /// # }) }
    /// ```
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = Result<T, E>> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
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
        })
    }
}
