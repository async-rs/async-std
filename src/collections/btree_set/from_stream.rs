use std::collections::BTreeSet;
use std::pin::Pin;

use crate::stream::{self, FromStream, IntoStream};

impl<T: Ord> FromStream<T> for BTreeSet<T> {
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

            let mut out = BTreeSet::new();
            stream::Extend::extend(&mut out, stream).await;
            out
        })
    }
}
