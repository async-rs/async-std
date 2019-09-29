use std::pin::Pin;
use std::borrow::Cow;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

impl Extend<char> for String {
    fn stream_extend<'a, S: IntoStream<Item = char> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        //TODO: Add this back in when size_hint is added to stream
        // let (lower_bound, _) = stream.size_hint();
        // self.reserve(lower_bound);

        //TODO: This can just be: stream.for_each(move |c| self.push(c))
        Box::pin(stream.fold((), move |(), c| self.push(c)))
    }
}

impl<'b> Extend<&'b char> for String {
    fn stream_extend<'a, S: IntoStream<Item = &'b char> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> where 'b: 'a {
        //TODO: Box::pin(stream.into_stream().copied())
        unimplemented!()
    }
}

impl<'b> Extend<&'b str> for String {
    fn stream_extend<'a, S: IntoStream<Item = &'b str> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> where 'b: 'a {
        //TODO: This can just be: stream.into_stream().for_each(move |s| self.push_str(s))
        Box::pin(stream.into_stream().fold((), move |(), s| self.push_str(s)))
    }
}

impl Extend<String> for String {
    fn stream_extend<'a, S: IntoStream<Item = String> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        //TODO: This can just be: stream.into_stream().for_each(move |s| self.push_str(&s))
        Box::pin(stream.into_stream().fold((), move |(), s| self.push_str(&s)))
    }
}

impl<'b> Extend<Cow<'b, str>> for String {
    fn stream_extend<'a, S: IntoStream<Item = Cow<'b, str>> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> where 'b: 'a {
        //TODO: This can just be: stream.into_stream().for_each(move |s| self.push_str(&s))
        Box::pin(stream.into_stream().fold((), move |(), s| self.push_str(&s)))
    }
}
