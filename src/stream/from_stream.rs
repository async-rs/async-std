use super::IntoStream;

use std::pin::Pin;

/// Conversion from a `Stream`.
///
/// By implementing `FromStream` for a type, you define how it will be created from a stream.
/// This is common for types which describe a collection of some kind.
///
/// See also: [`IntoStream`].
///
/// [`IntoStream`]: trait.IntoStream.html
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub trait FromStream<T: Send + Unpin> {
    /// Creates a value from a stream.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // use async_std::stream::FromStream;
    ///
    /// // let _five_fives = async_std::stream::repeat(5).take(5);
    /// ```
    fn from_stream<'a, S: IntoStream<Item = T> + Send + Unpin + 'a>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + Send + Unpin + 'a>>;
}
