use std::pin::Pin;
use std::collections::BTreeMap;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    fn stream_extend<'a, S: IntoStream<Item = (K, V)> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |(k, v)| {
            self.insert(k, v);
        }))
    }
}
