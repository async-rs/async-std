# Streams

Streams are a way to evaluate a stream of data; they are the async
equivalent of iterators.

`async-std` has provided quite a few helper methods to generate streams.
To get started, check out the `streams` package documentation.
It contains a bunch of helpful examples for how to create simple streams.

## What about a custom Stream?

Let's go over an async structure that would be improved by `Stream`.
We have this `SlowCounter` structure.
It ~~performs an expensive and unpredictable asynchronous operation~~ counts slowly.


```rust
struct SlowCounter {
    count: usize,
}

impl SlowCounter {
    pub fn new() -> Self {
        Self { count: Default::default() }
    }

    pub async fn count(&mut self) -> usize {
        task::sleep(Duration::from_secs_f32(0.5)).await;
        self.count += 1;
        self.count
    }
}
```

To use our structure we call `count()` and `await` the results.

```rust
fn main() {
    task::block_on(async {
        let mut counter = SlowCounter::new();

        assert_eq!(counter.count().await, 1);
        assert_eq!(counter.count().await, 2);
        assert_eq!(counter.count().await, 3);
    });
}
```

That works fine, but it would be nice to be able to use the standard Rust
idioms for iteration.
Here's the syntax we hope to use:

```rust
while let Some(i) = counter.next().await {
	dbg!(i);  // i = 1, 2, 3...
}
```

But of course, there is no `next` method on our `SlowCounter` so we would
see something like:

```rust
error[E0599]: no method named `next` found for struct `SlowCounter` in the current scope
  --> examples/async_std_stream_example.rs:29:26
   |
9  | struct SlowCounter {
   | ------------------
   | |
   | method `next` not found for this
   | doesn't satisfy `SlowCounter: async_std::stream::stream::StreamExt`
   | doesn't satisfy `SlowCounter: futures_core::stream::Stream`
```

> Note: Those `StreamExt` and `Stream` are the same trait! `async-std` re-exports `futures_core`.

Lets add a simple implementation.

```rust
impl Stream for SlowCounter {
    type Item = usize;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(Some(task::block_on(self.get_mut().count())))
    }
}
```

Alright, let's break this down.
From the middle out we are first unpinning ourself with `get_mut()`.
Now that we have a mutable reference to `self`, we can `count()`.
But notice that we don't actually `await` count, we just tell `task` to
`block_on` the future.
We then indicate that we want to return `Some` value, which will be a `uint`,
and when all of that is complete we will return `Poll::Ready(Some<T>)`.

Now we can count slowly _forever_.
Delightful.

If we want to count for less than forever we can use this sort of pattern.

```rust
for _ in 0..10 {
	dbg!(counter.next().await);
}
```

Or I'm pretty sure there's some way to use the helper methods in `stream`
to do this, but it's eluding me and I wanted to timebox this little exercise.
Anyone want to add that bit in before we land this PR? <3

