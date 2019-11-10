use std::collections::BTreeMap;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, IntoStream};

impl<K: Ord, V> stream::Extend<(K, V)> for BTreeMap<K, V> {
    fn extend<'a, S: IntoStream<Item = (K, V)> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |(k, v)| {
            self.insert(k, v);
        }))
    }
}
