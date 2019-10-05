use std::collections::LinkedList;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl<T> Extend<T> for LinkedList<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        //TODO: Add this back in when size_hint is added to Stream/StreamExt
        //let (lower_bound, _) = stream.size_hint();
        //self.reserve(lower_bound);
        Box::pin(stream.for_each(move |item| self.push_back(item)))
    }
}
