use std::collections::LinkedList;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, IntoStream};

impl<T> stream::Extend<T> for LinkedList<T> {
    fn extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        Box::pin(stream.for_each(move |item| self.push_back(item)))
    }
}
