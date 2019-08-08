# Overview

`async-std` and `async-task` along with their [supporting libraries][organization] are a two libraries making your life in async programming easier. They provide fundamental implementations for downstream libraries and applications alike.

`async-std` provides an interface to all important primitives: filesystem operations, network operations and concurrency basics like timers. It also exposes `async-task` in a model similar to the `thread` module found in the Rust standard lib. The name reflects the approach of this library: it is a closely modeled to the Rust main standard library as possible, replacing all components by async counterparts. You can read more about `async-std` in [the overview chapter][overview-std].

`async-task` is a library for implementing asynchronous tasks quickly. For the purpose of this documentation, you will mainly interact with it through the `async_std::task` module. Still, it has some nice properties to be aware of, which you can read up on in the [`async-task` overview chapter][overview-task].

[organization]: https://github.com/async-std/async-std
[overview-std]: overview/async-std/
[overview-task]: overview/async-task/