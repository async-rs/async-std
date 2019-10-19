use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, LinkedList, VecDeque};
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;

use crate::prelude::*;
use crate::stream::IntoStream;

/// Extend a collection with the contents of a stream.
///
/// Streams produce a series of values asynchronously, and collections can also be thought of as a
/// series of values. The `Extend` trait bridges this gap, allowing you to extend a collection
/// asynchronously by including the contents of that stream. When extending a collection with an
/// already existing key, that entry is updated or, in the case of collections that permit multiple
/// entries with equal keys, that entry is inserted.
///
/// ## Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream::{self, Extend};
///
/// let mut v: Vec<usize> = vec![1, 2];
/// let s = stream::repeat(3usize).take(3);
/// v.stream_extend(s).await;
///
/// assert_eq!(v, vec![1, 2, 3, 3, 3]);
/// #
/// # }) }
/// ```
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub trait Extend<A> {
    /// Extends a collection with the contents of a stream.
    fn stream_extend<'a, T: IntoStream<Item = A> + 'a>(
        &'a mut self,
        stream: T,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>>
    where
        A: 'a;
}

impl Extend<()> for () {
    fn stream_extend<'a, T: IntoStream<Item = ()> + 'a>(
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

impl<T> Extend<T> for Vec<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        self.reserve(stream.size_hint().0);
        Box::pin(stream.for_each(move |item| self.push(item)))
    }
}

impl Extend<char> for String {
    fn stream_extend<'a, S: IntoStream<Item = char> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        self.reserve(stream.size_hint().0);
        Box::pin(stream.for_each(move |c| self.push(c)))
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
        Box::pin(stream.into_stream().for_each(move |s| self.push_str(s)))
    }
}

impl Extend<String> for String {
    fn stream_extend<'a, S: IntoStream<Item = String> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |s| self.push_str(&s)))
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
        Box::pin(stream.into_stream().for_each(move |s| self.push_str(&s)))
    }
}

impl<T: Ord> Extend<T> for BinaryHeap<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        self.reserve(stream.size_hint().0);
        Box::pin(stream.for_each(move |item| self.push(item)))
    }
}

impl<K: Ord, V> Extend<(K, V)> for BTreeMap<K, V> {
    fn stream_extend<'a, S: IntoStream<Item = (K, V)> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |(k, v)| {
            self.insert(k, v);
        }))
    }
}

impl<T: Ord> Extend<T> for BTreeSet<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        Box::pin(stream.into_stream().for_each(move |item| {
            self.insert(item);
        }))
    }
}

impl<K, V, H> Extend<(K, V)> for HashMap<K, V, H>
where
    K: Eq + Hash,
    H: BuildHasher + Default,
{
    fn stream_extend<'a, S: IntoStream<Item = (K, V)> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();

        // The following is adapted from the hashbrown source code:
        // https://github.com/rust-lang/hashbrown/blob/d1ad4fc3aae2ade446738eea512e50b9e863dd0c/src/map.rs#L2470-L2491
        //
        // Keys may be already present or show multiple times in the stream. Reserve the entire
        // hint lower bound if the map is empty. Otherwise reserve half the hint (rounded up), so
        // the map will only resize twice in the worst case.

        let additional = if self.is_empty() {
            stream.size_hint().0
        } else {
            (stream.size_hint().0 + 1) / 2
        };
        self.reserve(additional);

        Box::pin(stream.for_each(move |(k, v)| {
            self.insert(k, v);
        }))
    }
}

impl<T, H> Extend<T> for HashSet<T, H>
where
    T: Eq + Hash,
    H: BuildHasher + Default,
{
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        // The Extend impl for HashSet in the standard library delegates to the internal HashMap.
        // Thus, this impl is just a copy of the async Extend impl for HashMap in this crate.

        let stream = stream.into_stream();

        // The following is adapted from the hashbrown source code:
        // https://github.com/rust-lang/hashbrown/blob/d1ad4fc3aae2ade446738eea512e50b9e863dd0c/src/map.rs#L2470-L2491
        //
        // Keys may be already present or show multiple times in the stream. Reserve the entire
        // hint lower bound if the map is empty. Otherwise reserve half the hint (rounded up), so
        // the map will only resize twice in the worst case.

        let additional = if self.is_empty() {
            stream.size_hint().0
        } else {
            (stream.size_hint().0 + 1) / 2
        };
        self.reserve(additional);

        Box::pin(stream.for_each(move |item| {
            self.insert(item);
        }))
    }
}

impl<T> Extend<T> for LinkedList<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        Box::pin(stream.for_each(move |item| self.push_back(item)))
    }
}

impl<T> Extend<T> for VecDeque<T> {
    fn stream_extend<'a, S: IntoStream<Item = T> + 'a>(
        &'a mut self,
        stream: S,
    ) -> Pin<Box<dyn Future<Output = ()> + 'a>> {
        let stream = stream.into_stream();
        self.reserve(stream.size_hint().0);
        Box::pin(stream.for_each(move |item| self.push_back(item)))
    }
}
