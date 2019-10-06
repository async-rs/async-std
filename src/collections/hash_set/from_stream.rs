use std::collections::HashSet;
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;

use crate::stream::{Extend, FromStream, IntoStream};

impl<T, H> FromStream<T> for HashSet<T, H>
where
    T: Eq + Hash,
    H: BuildHasher + Default,
{
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

            let mut out = HashSet::with_hasher(Default::default());
            out.stream_extend(stream).await;
            out
        })
    }
}
