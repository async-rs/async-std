use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, FromStream, IntoStream};

impl<K, V, H> FromStream<(K, V)> for HashMap<K, V, H>
where
    K: Eq + Hash,
    H: BuildHasher + Default,
{
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = (K, V)> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn Future<Output = Self> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            let mut out = HashMap::with_hasher(Default::default());
            stream::extend(&mut out, stream).await;
            out
        })
    }
}
