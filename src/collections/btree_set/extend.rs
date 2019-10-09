use std::collections::BTreeSet;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl<T: Ord> Extend<T> for BTreeSet<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |item| {
            self.insert(item);
            async {}
        }))
    }
}
