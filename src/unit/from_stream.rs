use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{FromStream, IntoStream};

impl FromStream<()> for () {
    #[inline]
    fn from_stream<'a, S: IntoStream<Item = ()>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        Box::pin(stream.into_stream().for_each(|_| async {}))
    }
}
