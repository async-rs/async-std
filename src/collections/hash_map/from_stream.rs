use std::pin::Pin;
use std::hash::{Hash, BuildHasher};
use std::collections::HashMap;

use crate::stream::{Extend, FromStream, IntoStream};

impl<K, V, H> FromStream<(K, V)> for HashMap<K, V, H>
where K: Eq + Hash,
      H: BuildHasher + Default {
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

            let mut out = HashMap::with_hasher(Default::default());
            out.stream_extend(stream).await;
            out
        })
    }
}
