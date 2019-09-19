use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream that moves out of a vector.
#[derive(Debug)]
pub struct IntoStream<T> {
    iter: std::vec::IntoIter<T>,
}

impl<T: Send + Unpin> crate::stream::IntoStream for Vec<T> {
    type Item = T;
    type IntoStream = IntoStream<T>;

    /// Creates a consuming iterator, that is, one that moves each value out of
    /// the vector (from start to end). The vector cannot be used after calling
    /// this.
    ///
    /// # Examples
    ///
    /// ```
    /// let v = vec!["a".to_string(), "b".to_string()];
    /// for s in v.into_stream() {
    ///     // s has type String, not &String
    ///     println!("{}", s);
    /// }
    /// ```
    #[inline]
    fn into_stream(self) -> IntoStream<T> {
        let iter = self.into_iter();
        IntoStream { iter }
    }
}

impl<T: Send + Unpin> crate::stream::Stream for IntoStream<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Pin::new(&mut *self).iter.next())
    }
}

/// Slice stream.
#[derive(Debug)]
pub struct Stream<'a, T> {
    iter: std::slice::Iter<'a, T>,
}

impl<'a, T: Sync> crate::stream::Stream for Stream<'a, T> {
    type Item = &'a T;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.iter.next())
    }
}

impl<'a, T: Sync> crate::stream::IntoStream for &'a Vec<T> {
    type Item = &'a T;
    type IntoStream = Stream<'a, T>;

    fn into_stream(self) -> Stream<'a, T> {
        let iter = self.iter();
        Stream { iter }
    }
}

/// Mutable slice stream.
#[derive(Debug)]
pub struct StreamMut<'a, T> {
    iter: std::slice::IterMut<'a, T>,
}

impl<'a, T: Sync> crate::stream::Stream for StreamMut<'a, T> {
    type Item = &'a mut T;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.iter.next())
    }
}

impl<'a, T: Send + Sync> crate::stream::IntoStream for &'a mut Vec<T> {
    type Item = &'a mut T;

    type IntoStream = StreamMut<'a, T>;

    fn into_stream(self) -> StreamMut<'a, T> {
        let iter = self.iter_mut();
        StreamMut { iter }
    }
}
