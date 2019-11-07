use std::collections::BTreeMap;
use std::pin::Pin;

use crate::stream::{self, FromStream, IntoStream};

impl<K: Ord, V> FromStream<(K, V)> for BTreeMap<K, V> {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = (K, V)>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = BTreeMap::new();
            stream::extend(&mut out, stream).await;
            out
        })
    }
}
