use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{self, IntoStream};

impl stream::Extend<()> for () {
    fn extend<'a, T: IntoStream<Item = ()> + 'a>(
        &'a mut self,
        stream: T,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            while let Some(_) = stream.next().await {}
        })
    }
}
