use std::borrow::Cow;
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl Extend<char> for String {
    fn stream_extend<'a, S: IntoStream<Item = char> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();

        self.reserve(stream.size_hint().0);

        Box::pin(stream.for_each(move |c| {
            self.push(c);
            async {}
        }))
    }
}

impl<'b> Extend<&'b char> for String {
    fn stream_extend<'a, S: IntoStream<Item = &'b char> + 'a>(
        &'a mut self,
        //TODO: Remove the underscore when uncommenting the body of this impl
        _stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        'b: 'a,
    {
        //TODO: This can be uncommented when `copied` is added to Stream/StreamExt
        //Box::pin(stream.into_stream().copied())
        unimplemented!()
    }
}

impl<'b> Extend<&'b str> for String {
    fn stream_extend<'a, S: IntoStream<Item = &'b str> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        'b: 'a,
    {
        Box::pin(stream.into_stream().for_each(move |s| {
            self.push_str(s);
            async {}
        }))
    }
}

impl Extend<String> for String {
    fn stream_extend<'a, S: IntoStream<Item = String> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |s| {
            self.push_str(&s);
            async {}
        }))
    }
}

impl<'b> Extend<Cow<'b, str>> for String {
    fn stream_extend<'a, S: IntoStream<Item = Cow<'b, str>> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        'b: 'a,
    {
        Box::pin(stream.into_stream().for_each(move |s| {
            self.push_str(&s);
            async {}
        }))
    }
}
