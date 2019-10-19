use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;

use crate::prelude::*;
use crate::stream::{Extend, IntoStream};

/// Conversion from a `Stream`.
///
/// By implementing `FromStream` for a type, you define how it will be created from a stream.
/// This is common for types which describe a collection of some kind.
///
/// See also: [`IntoStream`].
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
///  use crate::async_std::stream::FromStream;
///  use async_std::prelude::*;
///  use async_std::stream;
///
///  let five_fives = stream::repeat(5).take(5);
///
///  let v = Vec::from_stream(five_fives).await;
///
///  assert_eq!(v, vec![5, 5, 5, 5, 5]);
/// # Ok(()) }) }
/// ```
///
/// Using `collect` to  implicitly use `FromStream`
///
///```
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// use async_std::prelude::*;
/// use async_std::stream;
/// let five_fives = stream::repeat(5).take(5);
///
/// let v: Vec<i32> = five_fives.collect().await;
///
/// assert_eq!(v, vec![5, 5, 5, 5, 5]);
/// #
/// # Ok(()) }) }
///```
///
/// Implementing `FromStream` for your type:
///
/// ```
/// use async_std::prelude::*;
/// use async_std::stream::{Extend, FromStream, IntoStream};
/// use async_std::stream;
/// use std::pin::Pin;
///
/// // A sample collection, that's just a wrapper over Vec<T>
/// #[derive(Debug)]
/// struct MyCollection(Vec<i32>);
///
/// // Let's give it some methods so we can create one and add things
/// // to it.
/// impl MyCollection {
///     fn new() -> MyCollection {
///         MyCollection(Vec::new())
///     }
///
///     fn add(&mut self, elem: i32) {
///         self.0.push(elem);
///     }
/// }
///
/// // and we'll implement FromIterator
/// impl FromStream<i32> for MyCollection {
///     fn from_stream<'a, S: IntoStream<Item = i32> + 'a>(
///         stream: S,
///     ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>> {
///         let stream = stream.into_stream();
///
///         Box::pin(async move {
///             let mut c = MyCollection::new();
///
///             let mut v = vec![];
///             v.stream_extend(stream).await;
///
///             for i in v {
///                 c.add(i);
///             }
///             c
///         })
///     }
/// }
///
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// // Now we can make a new stream...
/// let stream = stream::repeat(5).take(5);
///
/// // ...and make a MyCollection out of it
/// let c = MyCollection::from_stream(stream).await;
///
/// assert_eq!(c.0, vec![5, 5, 5, 5, 5]);
///
/// // collect works too!
///
/// let stream = stream::repeat(5).take(5);
/// let c: MyCollection = stream.collect().await;
///
/// assert_eq!(c.0, vec![5, 5, 5, 5, 5]);
/// # Ok(()) }) }
///```
///
/// [`IntoStream`]: trait.IntoStream.html
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub trait FromStream<T> {
    /// Creates a value from a stream.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    ///  use crate::async_std::stream::FromStream;
    ///  use async_std::prelude::*;
    ///  use async_std::stream;
    ///
    ///  let five_fives = stream::repeat(5).take(5);
    ///
    ///  let v = Vec::from_stream(five_fives).await;
    ///
    ///  assert_eq!(v, vec![5, 5, 5, 5, 5]);
    /// # Ok(()) }) }
    /// ```
    fn from_stream<'a, S: IntoStream<Item = T> + 'a>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>;
}

impl FromStream<()> for () {
    fn from_stream<'a, S: IntoStream<Item = ()>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        Box::pin(stream.into_stream().for_each(|_| ()))
    }
}

impl<T> FromStream<T> for Vec<T> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = vec![];
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b, T: Clone> FromStream<T> for Cow<'b, [T]> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            Cow::Owned(FromStream::from_stream(stream).await)
        })
    }
}

impl<T> FromStream<T> for Box<[T]> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            Vec::from_stream(stream).await.into_boxed_slice()
        })
    }
}

impl<T> FromStream<T> for Rc<[T]> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            Vec::from_stream(stream).await.into()
        })
    }
}

impl<T> FromStream<T> for Arc<[T]> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            Vec::from_stream(stream).await.into()
        })
    }
}

impl<T, V> FromStream<Option<T>> for Option<V>
where
    V: FromStream<T>,
{
    /// Takes each element in the stream: if it is `None`, no further
    /// elements are taken, and `None` is returned. Should no `None`
    /// occur, a container with the values of each `Option` is returned.
    fn from_stream<'a, S: IntoStream<Item = Option<T>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            // Using `scan` here because it is able to stop the stream early
            // if a failure occurs
            let mut found_error = false;
            let out: V = stream
                .scan((), |_, elem| {
                    match elem {
                        Some(elem) => Some(elem),
                        None => {
                            found_error = true;
                            // Stop processing the stream on error
                            None
                        }
                    }
                })
                .collect()
                .await;

            if found_error { None } else { Some(out) }
        })
    }
}

impl<T, E, V> FromStream<Result<T, E>> for Result<V, E>
where
    V: FromStream<T>,
{
    /// Takes each element in the stream: if it is an `Err`, no further
    /// elements are taken, and the `Err` is returned. Should no `Err`
    /// occur, a container with the values of each `Result` is returned.
    fn from_stream<'a, S: IntoStream<Item = Result<T, E>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            // Using `scan` here because it is able to stop the stream early
            // if a failure occurs
            let mut found_error = None;
            let out: V = stream
                .scan((), |_, elem| {
                    match elem {
                        Ok(elem) => Some(elem),
                        Err(err) => {
                            found_error = Some(err);
                            // Stop processing the stream on error
                            None
                        }
                    }
                })
                .collect()
                .await;

            match found_error {
                Some(err) => Err(err),
                None => Ok(out),
            }
        })
    }
}

impl FromStream<char> for String {
    fn from_stream<'a, S: IntoStream<Item = char>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<&'b char> for String {
    fn from_stream<'a, S: IntoStream<Item = &'b char>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<&'b str> for String {
    fn from_stream<'a, S: IntoStream<Item = &'b str>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl FromStream<String> for String {
    fn from_stream<'a, S: IntoStream<Item = String>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<'b> FromStream<Cow<'b, str>> for String {
    fn from_stream<'a, S: IntoStream<Item = Cow<'b, str>>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = String::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<T: Ord> FromStream<T> for BinaryHeap<T> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = BinaryHeap::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<K: Ord, V> FromStream<(K, V)> for BTreeMap<K, V> {
    fn from_stream<'a, S: IntoStream<Item = (K, V)>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = BTreeMap::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<T: Ord> FromStream<T> for BTreeSet<T> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = BTreeSet::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<K, V, H> FromStream<(K, V)> for HashMap<K, V, H>
where
    K: Eq + Hash,
    H: BuildHasher + Default,
{
    fn from_stream<'a, S: IntoStream<Item = (K, V)>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = HashMap::with_hasher(Default::default());
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<T, H> FromStream<T> for HashSet<T, H>
where
    T: Eq + Hash,
    H: BuildHasher + Default,
{
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = HashSet::with_hasher(Default::default());
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<T> FromStream<T> for LinkedList<T> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = LinkedList::new();
            out.stream_extend(stream).await;
            out
        })
    }
}

impl<T> FromStream<T> for VecDeque<T> {
    fn from_stream<'a, S: IntoStream<Item = T>>(
        stream: S,
    ) -> Pin<Box<dyn core::future::Future<Output = Self> + 'a>>
    where
        <S as IntoStream>::IntoStream: 'a,
    {
        let stream = stream.into_stream();

        Box::pin(async move {
            pin_utils::pin_mut!(stream);

            let mut out = VecDeque::new();
            out.stream_extend(stream).await;
            out
        })
    }
}
