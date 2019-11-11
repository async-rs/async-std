use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{Stream, Product};

impl<T, U, E> Product<Result<U, E>> for Result<T, E>
where
    T: Product<U>,
{
    #[doc = r#"
        Takes each element in the `Stream`: if it is an `Err`, no further
        elements are taken, and the `Err` is returned. Should no `Err` occur,
        the product of all elements is returned.

        # Examples

        This multiplies every integer in a vector, rejecting the product if a negative element is
        encountered:

        ```
        # fn main() { async_std::task::block_on(async {
        #
        use async_std::prelude::*;
        use async_std::stream;

        let v = stream::from_iter(vec![1, 2, 4]);
        let res: Result<i32, &'static str> = v.map(|x|
            if x < 0 {
                Err("Negative element found")
            } else {
                Ok(x)
            }).product().await;
        assert_eq!(res, Ok(8));
        #
        # }) }
        ```
    "#]
    fn product<'a, S>(stream: S) -> Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>
        where S: Stream<Item = Result<U, E>> + 'a
    {
        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            // Using `scan` here because it is able to stop the stream early
            // if a failure occurs
            let mut found_error = None;
            let out = <T as Product<U>>::product(stream
                .scan((), |_, elem| {
                    match elem {
                        Ok(elem) => Some(elem),
                        Err(err) => {
                            found_error = Some(err);
                            // Stop processing the stream on error
                            None
                        }
                    }
                })).await;
            match found_error {
                Some(err) => Err(err),
                None => Ok(out)
            }
        })
    }
}
