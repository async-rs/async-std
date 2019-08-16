# Futures

A notable point about Rust is [*fearless concurrency*](https://blog.rust-lang.org/2015/04/10/Fearless-Concurrency.html). That is the notion that you should be empowered to do concurrent things, without giving up safety. Also, Rust being a low-level language, it's about fearless concurrency *without picking a specific implementation strategy*. This means we *must* abstract over the strategy, to allow choice *later*, if we want to have any way to share code between users of different strategies.

Futures abstract over *computation*. They describe the "what", independent of the "where" and the "when". For that, they aim to break code into small, composable actions that can then be executed by a part of our system. Let's take a tour through what it means to compute things to find where we can abstract.

## Send and Sync

Luckily, concurrent Rust already has two well-known and effective concepts abstracting over sharing between concurrent parts of a program: Send and Sync. Notably, both the Send and Sync traits abstract over *strategies* of concurrent work, compose neatly, and don't prescribe an implementation.

As a quick summary, `Send` abstracts over passing data in a computation over to another concurrent computation (let's call it the receiver), losing access to it on the sender side. In many programming languages, this strategy is commonly implemented, but missing support from the language side expects you to enforce the "losing access" behaviour yourself. This is a regular source of bugs: senders keeping handles to sent things around and maybe even working with them after sending. Rust mitigates this problem by making this behaviour known. Types can be `Send` or not (by implementing the appropriate marker trait), allowing or disallowing sending them around, and the ownership and borrowing rules prevent subsequent access.

Note how we avoided any word like *"thread"*, but instead opted for "computation". The full power of `Send` (and subsequently also `Sync`) is that they relieve you of the burden of knowing *what* shares. At the point of implementation, you only need to know which method of sharing is appropriate for the type at hand. This keeps reasoning local and is not influenced by whatever implementation the user of that type later uses.

`Sync` is about sharing data between two concurrent parts of a program. This is another common pattern: as writing to a memory location or reading while another party is writing is inherently unsafe, this access needs to be moderated through synchronisation.[^1] There are many common ways for two parties to agree on not using the same part in memory at the same time, for example mutexes and spinlocks. Again, Rust gives you the option of (safely!) not caring. Rust gives you the ability to express that something *needs* synchronisation while not being specific about the *how*.

`Send` and `Sync` can be composed in interesting fashions, but that's beyond the scope here. You can find examples in the [Rust Book][rust-book-sync].

[rust-book-sync]: https://doc.rust-lang.org/stable/book/ch16-04-extensible-concurrency-sync-and-send.html

To sum up: Rust gives us the ability to safely abstract over important properties of concurrent programs, their data sharing. It does so in a very lightweight fashion; the language itself only knows about the two markers `Send` and `Sync` and helps us a little by deriving them itself, when possible. The rest is a library concern.

## An easy view of computation

While computation is a subject to write a whole [book](https://computationbook.com/) about, a very simplified view suffices for us:

- computation is a sequence of composable operations
- they can branch based on a decision
- they either run to succession and yield a result, or they can yield an error

## Deferring computation

As mentioned above `Send` and `Sync` are about data. But programs are not only about data, they also talk about *computing* the data. And that's what [`Futures`][futures] do. We are going to have a close look at how that works in the next chapter. Let's look at what Futures allow us to express, in English. Futures go from this plan:

- Do X
- If X succeeds, do Y

towards

- Start doing X
- Once X succeeds, start doing Y

Remember the talk about "deferred computation" in the intro? That's all it is. Instead of telling the computer what to execute and decide upon *now*, you tell it what to start doing and how to react on potential events the... well... `Future`.

[futures]: https://doc.rust-lang.org/std/future/trait.Future.html

## Orienting towards the beginning

Let's have a look at a simple function, specifically the return value:

    fn read_file(path: &str) -> Result<String, io::Error> {
        let mut file = File.open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?; 
        contents
    }

You can call that at any time, so you are in full control on when you call it. But here's the problem: the moment you call it, you transfer control to the called function. It returns a value.
Note that this return value talks about the past. The past has a drawback: all decisions have been made. It has an advantage: the outcome is visible. We can unwrap the presents of program past and then decide what to do with it.

But here's a problem: we wanted to abstract over *computation* to be allowed to let someone else choose how to run it. That's fundamentally incompatible with looking at the results of previous computation all the time. So, let's find a type that describes a computation without running it. Let's look at the function again:

    fn read_file(path: &str) -> Result<String, io::Error> {
        let mut file = File.open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?; 
        contents
    }

Speaking in terms of time, we can only take action *before* calling the function or *after* the function returned. This is not desirable, as it takes from us the ability to do something *while* it runs. When working with parallel code, this would take from us the ability to start a parallel task while the first runs (because we gave away control).

This is the moment where we could reach for [threads](https://en.wikipedia.org/wiki/Thread_). But threads are a very specific concurrency primitive and we said that we are searching for an abstraction.
What we are searching is something that represents ongoing work towards a result in the future. Whenever we say `something` in Rust, we almost always mean a trait. Let's start with an incomplete definition of the `Future` trait:

    trait Future {
        type Output;
    
        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>;
    }

Ignore `Pin` and `Context` for now, you don't need them for high-level understanding. Looking at it closely, we see the following: it is generic over the `Output`. It provides a function called `poll`, which allows us to check on the state of the current computation.
Every call to `poll()` can result in one of these two cases:

1. The future is done, `poll` will return [`Poll::Ready`](https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Ready)
2. The future has not finished executing, it will return [`Poll::Pending`](https://doc.rust-lang.org/std/task/enum.Poll.html#variant.Pending)

This allows us to externally check if a `Future` has finished doing its work, or is finally done and can give us the value. The most simple way (but not efficient) would be to just constantly poll futures in a loop. There's optimisations here, and this is what a good runtime is does for you.
Note that calling `poll` after case 1 happened may result in confusing behaviour. See the [futures-docs](https://doc.rust-lang.org/std/future/trait.Future.html) for details.

## Async

While the `Future` trait has existed in Rust for a while, it was inconvenient to build and describe them. For this, Rust now has a special syntax: `async`. The example from above, implemented in `async-std`, would look like this:

    use async_std::fs::File;
    
    async fn read_file(path: &str) -> Result<String, io::Error> {
        let mut file = File.open(path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?; 
        contents
    }

Amazingly little difference, right? All we did is label the function `async` and insert 2 special commands: `.await`.

This function sets up a deferred computation. When this function is called, it will produce a `Future<Output=String>` instead of immediately returning a String. (Or, more precisely, generate a type for you that implements `Future<Output=String>`.)

## What does `.await` do?

The `.await` postfix does exactly what it says on the tin: the moment you use it, the code will wait until the requested action (e.g. opening a file or reading all data in it) is finished. `.await?` is not special, it’s just the application of the `?` operator to the result of `.await`. So, what is gained over the initial code example? We’re getting futures and then immediately waiting for them?

The `.await` points act as a marker. Here, the code will wait for a `Future` to produce its value. How will a future finish? You don’t need to care! The marker allows the code later *executing* this piece of code (usually called the “runtime”) when it can take some time to care about all the other things it has to do. It will come back to this point when the operation you are doing in the background is done. This is why this style of programming is also called *evented programming*. We are waiting for *things to happen* (e.g. a file to be opened) and then react (by starting to read).

When executing 2 or more of these functions at the same time, our runtime system is then able to fill the wait time with handling *all the other events* currently going on.

## Conclusion

Working from values, we searched for something that expresses *working towards a value available sometime later*. From there, we talked about the concept of polling.

A `Future` is any data type that does not represent a value, but the ability to *produce a value at some point in the future*. Implementations of this are very varied and detailed depending on use-case, but the interface is simple.

Next, we will introduce you to `tasks`, which we need to actually *run* Futures.

[^1]: Two parties reading while it is guaranteed that no one is writing is always safe.
