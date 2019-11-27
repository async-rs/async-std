# `std::future` and `futures-rs`

Rust has two kinds of types commonly referred to as `Future`: 


- the first is `std::future::Future` from Rust’s [standard library](https://doc.rust-lang.org/std/future/trait.Future.html). 
- the second is `futures::future::Future` from the [futures-rs crate](https://docs.rs/futures/0.3/futures/prelude/trait.Future.html). 

The future defined in the [futures-rs](https://docs.rs/futures/0.3/futures/prelude/trait.Future.html) crate was the original implementation of the type. To enable the `async/await` syntax, the core Future trait was moved into Rust’s standard library and became `std::future::Future`. In some sense, the `std::future::Future` can be seen as a minimal subset of `futures::future::Future`.

It is critical to understand the difference between `std::future::Future` and `futures::future::Future`, and the approach that `async-std` takes towards them. In itself, `std::future::Future` is not something you want to interact with as a user—except by calling `.await` on it. The inner workings of `std::future::Future` are mostly of interest to people implementing `Future`. Make no mistake—this is very useful! Most of the functionality that used to be defined on `Future` itself has been moved to an extension trait called [`FuturesExt`](https://docs.rs/futures/0.3/futures/future/trait.FutureExt.html). From this information, you might be able to infer that the `futures` library serves as an extension to the core Rust async features.

In the same tradition as `futures`, `async-std` re-exports the core `std::future::Future` type. You can actively opt into the extensions provided by the `futures` crate by adding it to your `Cargo.toml` and importing `FuturesExt`.

## Interfaces and Stability

 `async-std` aims to be a stable and reliable library, at the level of the Rust standard library. This also means that we don't rely on the `futures` library for our interface. Yet, we appreciate that many users have come to like the conveniences that `futures-rs` brings. For that reason, `async-std` implements all `futures` traits for its types.
 
 Luckily, the approach from above gives you full flexibility. If you care about stability a lot, you can just use `async-std` as is. If you prefer the `futures` library interfaces, you link those in. Both uses are first class.

## `async_std::future`

There’s some support functions that we see as important for working with futures of any kind. These can be found in the `async_std::future` module and are covered by our stability guarantees.

## Streams and Read/Write/Seek/BufRead traits

Due to limitations of the Rust compiler, those are currently implemented in `async_std`, but cannot be implemented by users themselves.
