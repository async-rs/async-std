use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl<T> Extend<T> for Vec<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        Box::pin(async move {
            pin_utils::pin_mut!(stream);
            while let Some(item) = stream.next().await {
                self.push(item);
            }
        })
    }
}
